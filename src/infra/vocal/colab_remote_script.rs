use crate::domain::vocal_separation_mode::VocalSeparationMode;

/// Generates a self-contained Python script to be executed on the Google Colab runtime.
///
/// The script performs:
/// 1. Conditional install of `audio-separator[cpu]`, `numpy`, and `soundfile` if not already importable.
/// 2. Audio separation with each specified model and `--mdxc_overlap`.
/// 3. Classification of stems by parsing the parenthetical content in the filename (e.g. `(Instrumental)`),
///    explicitly avoiding full-filename substring checks because model filenames may contain `"instrumental"`.
/// 4. Averaging stems of the target type element-wise across models using numpy and soundfile.
/// 5. High-quality 320k MP3 encoding with ffmpeg.
/// 6. SHA-256 calculation, duration, and size, then emits exactly ONE line to
///    stdout containing JSON: `{"url": "", "sha256":..., "duration":..., "size":...}`.
///    The result is left as a plain MP3 at `REMOTE_OUTPUT_PATH` for the official
///    `colab download` command to fetch; there is no encryption or external upload.
pub fn generate_remote_script(
    mode: VocalSeparationMode,
    models: &[&str],
    overlap: u32,
    batch_size: u32,
    _password: &str,
    remote_input_path: &str,
) -> String {
    let mode_str =
        serde_json::to_string(mode.as_str()).unwrap_or_else(|_| "\"instrumental\"".to_string());
    let models_json = serde_json::to_string(models).unwrap_or_else(|_| "[]".to_string());
    let input_path_json = serde_json::to_string(remote_input_path)
        .unwrap_or_else(|_| "\"/content/input_audio\"".to_string());

    format!(
        r#"#!/usr/bin/env python3
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# 0. GPU watchdog. Only the client can send "colab stop", so a SIGKILL, an OOM
# or a lost connection would leave the GPU allocated with nobody holding the
# handle. This arms a watchdog inside the runtime that kills this kernel, which
# makes Colab release the session. On the happy path the client stops the
# session first and the whole runtime dies, so the watchdog never fires.
WATCHDOG_ABSOLUTE_SECS = 3000   # hard ceiling for a hung separation
WATCHDOG_GRACE_SECS = 300       # after the result is uploaded

def _arm_watchdog(seconds):
    try:
        subprocess.Popen(
            ["setsid", "sh", "-c", f"sleep {{seconds}}; kill -9 {{os.getpid()}}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass

_arm_watchdog(WATCHDOG_ABSOLUTE_SECS)

# 1. Install dependencies only if import fails
try:
    import audio_separator
except ImportError:
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "-q", "audio-separator[cpu]"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=True,
    )
    import audio_separator

try:
    import numpy as np
    import soundfile as sf
except ImportError:
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "-q", "numpy", "soundfile"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=True,
    )
    import numpy as np
    import soundfile as sf

MODE = {mode_str}
MODELS = {models_json}
OVERLAP = {overlap}
BATCH_SIZE = {batch_size}
INPUT_AUDIO = {input_path_json}

def get_parenthetical_label(path):
    m = re.search(r'\(([^)]+)\)', Path(path).name)
    if m:
        return m.group(1).strip().lower()
    return ""

def is_target_stem(label):
    # Classify stems strictly by the parenthetical content, NOT by substring of full name.
    # In both instrumental and karaoke ensembles, the target backing/instrumental stem is labeled
    # '(Instrumental)', '(no_vocals)', or '(accompaniment)', while vocals are '(Vocals)' or '(Lead Vocal)'.
    if not label:
        return False
    # Check the explicit non-vocal names first: "(no_vocals)" contains the
    # substring "vocal" and would otherwise be classified as a vocal stem.
    if label in ("no_vocals", "accompaniment"):
        return True
    if "vocal" in label:
        return False
    return "instrumental" in label

def is_vocal_stem(label):
    # "(no_vocals)" is an instrumental label, not a vocal one.
    return "vocal" in label and label != "no_vocals"

work_dir = Path("/content/colab_sep_work")
work_dir.mkdir(parents=True, exist_ok=True)

stems = []

# 2. Run separation for each model in the ensemble
for i, model in enumerate(MODELS, 1):
    model_out = work_dir / f"stem_{{i}}"
    model_out.mkdir(parents=True, exist_ok=True)
    cmd = [
        "audio-separator",
        INPUT_AUDIO,
        "--model_filename", model,
        "--output_dir", str(model_out),
        "--output_format", "WAV",
        "--mdxc_overlap", str(OVERLAP),
        # Half precision is the single biggest speed lever on a modern GPU:
        # measured on the production RTX PRO 6000, the same model drops from
        # 24.8s to 5.7s while the vocal leak stays identical (0.0632 -> 0.0631
        # on corr(instrumental, vocals)).
        "--use_native_fp16",
        # The default batch of 1 leaves the GPU idle between frames. Measured:
        # 25s at batch 1, 8.5s at batch 8, flat above that (8.5s at 32 and 64),
        # so 8 is the knee and larger values only spend memory.
        "--mdxc_batch_size", str(BATCH_SIZE),
        "--log_level", "WARNING",
    ]
    res = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if res.returncode != 0:
        raise RuntimeError(f"audio-separator failed for {{model}}: {{res.stderr}}")

    wavs = sorted(model_out.glob("*.wav"))
    targets = [w for w in wavs if is_target_stem(get_parenthetical_label(w))]
    if len(targets) != 1:
        # Fallback if label is slightly different but still contains no vocal inside parentheses
        targets = [w for w in wavs if not is_vocal_stem(get_parenthetical_label(w))]
    if len(targets) != 1:
        raise RuntimeError(f"Unexpected stems for {{model}}: {{[w.name for w in wavs]}}")
    stems.append(targets[0])

# 3. Average stems of the same type across models using numpy and soundfile
data_list = []
min_len = None
sr = None

for s in stems:
    d, s_rate = sf.read(str(s))
    if sr is None:
        sr = s_rate
    if min_len is None or len(d) < min_len:
        min_len = len(d)
    data_list.append(d)

trimmed = [d[:min_len] for d in data_list]
avg_data = np.mean(trimmed, axis=0)

temp_wav = work_dir / "averaged.wav"
sf.write(str(temp_wav), avg_data, sr)

# 4. Encode to MP3 320k with ffmpeg
output_mp3 = work_dir / "output.mp3"
ffmpeg_cmd = [
    "ffmpeg", "-y", "-v", "error",
    "-i", str(temp_wav),
    "-map_metadata", "-1",
    "-codec:a", "libmp3lame",
    "-b:a", "320k",
    str(output_mp3)
]
subprocess.run(ffmpeg_cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

# 5. Calculate sha256, duration, and size
h = hashlib.sha256()
with open(output_mp3, "rb") as f:
    while chunk := f.read(65536):
        h.update(chunk)
sha256_hex = h.hexdigest()
size_bytes = os.path.getsize(output_mp3)

probe_res = subprocess.run(
    ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", str(output_mp3)],
    capture_output=True, text=True, check=True
)
duration_secs = float(probe_res.stdout.strip())

# 6. Output strictly ONE small JSON line. The result is a plain MP3 left in
# place at `output.mp3` (REMOTE_OUTPUT_PATH); the official `colab download`
# fetches it. `url` is kept as an empty string for compatibility with the result
# parser, which requires the field — there is no external upload anymore.
payload = {{
    "url": "",
    "sha256": sha256_hex,
    "duration": duration_secs,
    "size": size_bytes
}}
print(json.dumps(payload))
sys.stdout.flush()

# 7. The result is already on disk. If the client never confirms the stop, free
# the GPU anyway once the grace period expires.
_arm_watchdog(WATCHDOG_GRACE_SECS)
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::vocal::colab_vocal_separator_config::{COROS_MODELS, INSTRUMENTAL_MODELS};

    #[test]
    fn test_generate_remote_script_instrumental() {
        let script = generate_remote_script(
            VocalSeparationMode::Instrumental,
            &INSTRUMENTAL_MODELS,
            12,
            8,
            "secret_pass",
            "/content/audio.mp3",
        );

        assert!(script.contains("MODE = \"instrumental\""));
        assert!(script.contains("OVERLAP = 12"));
        assert!(script.contains("INPUT_AUDIO = \"/content/audio.mp3\""));
        for m in INSTRUMENTAL_MODELS {
            assert!(script.contains(m));
        }
        assert!(script.contains(r"\(([^)]+)\)"));
        assert!(script.contains("audio-separator[cpu]"));
        assert!(script.contains("np.mean"));
        assert!(script.contains("libmp3lame"));
        assert!(script.contains("320k"));
        assert!(script.contains("json.dumps(payload)"));
        assert!(script.contains("\"url\": \"\""));
        // T3: the catbox/AES/curl workaround is gone. The result must be a
        // plain file at REMOTE_OUTPUT_PATH fetched by `colab download`.
        assert!(!script.contains("openssl"), "openssl encryption must be removed");
        assert!(!script.contains("catbox"), "catbox upload must be removed");
        assert!(!script.contains("litterbox"), "litterbox upload must be removed");
        assert!(!script.contains("enc_path"), "the encrypted payload path must be removed");
        assert!(
            !script.contains("TRANSFER_OK"),
            "the uploader-chain protocol must be removed"
        );
        // S2: a client crash must not leave the GPU allocated.
        assert!(script.contains("_arm_watchdog(WATCHDOG_ABSOLUTE_SECS)"));
        assert!(script.contains("_arm_watchdog(WATCHDOG_GRACE_SECS)"));
        assert!(script.contains("kill -9"));
    }

    #[test]
    fn test_generated_script_uses_fp16_and_batched_inference() {
        let script = generate_remote_script(
            VocalSeparationMode::Instrumental,
            &INSTRUMENTAL_MODELS,
            12,
            8,
            "pw",
            "/content/in.wav",
        );

        // Measured on the production GPU (RTX PRO 6000 Blackwell): the same
        // model takes 24.8s without these flags and 5.7s with them, while the
        // vocal leak stays identical (corr(instrumental, vocals) 0.0632 vs
        // 0.0631). Dropping them silently costs 4.5x the wall time on every
        // separation, and that time is paid GPU time.
        assert!(
            script.contains("--use_native_fp16"),
            "half precision is the main speed lever; without it every separation runs 4.5x slower for the same audio quality"
        );
        assert!(
            script.contains("--mdxc_batch_size"),
            "batch size must be raised above the default of 1 to keep the GPU busy"
        );
    }
    #[test]
    fn test_generate_remote_script_coros() {
        let script = generate_remote_script(
            VocalSeparationMode::Coros,
            &COROS_MODELS,
            8,
            8,
            "pass_coros",
            "/content/song.wav",
        );

        assert!(script.contains("MODE = \"chorus\""));
        assert!(script.contains("OVERLAP = 8"));
        for m in COROS_MODELS {
            assert!(script.contains(m));
        }
    }

    #[test]
    fn test_generated_script_is_valid_python() {
        let script = generate_remote_script(
            VocalSeparationMode::Instrumental,
            &INSTRUMENTAL_MODELS,
            12,
            8,
            "pw",
            "/content/in.wav",
        );
        // The runtime script is Python: a template typo must fail here instead of
        // on a paid GPU, after the session has already started.
        let path =
            std::env::temp_dir().join(format!("colab_remote_script_{}.py", std::process::id()));
        std::fs::write(&path, &script).unwrap();

        let output = std::process::Command::new("python3")
            .args(["-m", "py_compile"])
            .arg(&path)
            .output();

        let _ = std::fs::remove_file(&path);

        match output {
            Ok(o) => assert!(
                o.status.success(),
                "generated script is not valid Python: {}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(_) => eprintln!("skipping: python3 is not available"),
        }
    }
}
