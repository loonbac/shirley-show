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
/// 6. SHA-256 calculation, openssl AES-256-CBC encryption with PBKDF2 and salt, and upload to litterbox.catbox.moe.
/// 7. Emits exactly ONE line to stdout containing JSON: `{"url":..., "sha256":..., "duration":..., "size":...}`.
pub fn generate_remote_script(
    mode: VocalSeparationMode,
    models: &[&str],
    overlap: u32,
    password: &str,
    remote_input_path: &str,
) -> String {
    let mode_str = serde_json::to_string(mode.as_str()).unwrap_or_else(|_| "\"instrumental\"".to_string());
    let models_json = serde_json::to_string(models).unwrap_or_else(|_| "[]".to_string());
    let password_json = serde_json::to_string(password).unwrap_or_else(|_| "\"\"".to_string());
    let input_path_json = serde_json::to_string(remote_input_path).unwrap_or_else(|_| "\"/content/input_audio\"".to_string());

    format!(
r#"#!/usr/bin/env python3
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

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
PASSWORD = {password_json}
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

# 6. Encrypt with openssl (AES-256-CBC, PBKDF2, salt)
enc_path = work_dir / "output.mp3.enc"
openssl_cmd = [
    "openssl", "enc", "-aes-256-cbc", "-pbkdf2", "-salt",
    "-pass", f"pass:{{PASSWORD}}",
    "-in", str(output_mp3),
    "-out", str(enc_path)
]
subprocess.run(openssl_cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

# 7. Upload to litterbox with curl
curl_cmd = [
    "curl", "-s",
    "-F", "reqtype=fileupload",
    "-F", "time=1h",
    "-F", f"fileToUpload=@{{enc_path}}",
    "https://litterbox.catbox.moe/resources/internals/api.php"
]
curl_res = subprocess.run(curl_cmd, capture_output=True, text=True, check=True)
url = curl_res.stdout.strip()
if not url.startswith("http"):
    raise RuntimeError(f"Litterbox upload failed: {{url}}")

# 8. Output strictly ONE small JSON line
payload = {{
    "url": url,
    "sha256": sha256_hex,
    "duration": duration_secs,
    "size": size_bytes
}}
print(json.dumps(payload))
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
            "secret_pass",
            "/content/audio.mp3",
        );

        assert!(script.contains("MODE = \"instrumental\""));
        assert!(script.contains("OVERLAP = 12"));
        assert!(script.contains("PASSWORD = \"secret_pass\""));
        assert!(script.contains("INPUT_AUDIO = \"/content/audio.mp3\""));
        for m in INSTRUMENTAL_MODELS {
            assert!(script.contains(m));
        }
        assert!(script.contains(r"\(([^)]+)\)"));
        assert!(script.contains("audio-separator[cpu]"));
        assert!(script.contains("np.mean"));
        assert!(script.contains("libmp3lame"));
        assert!(script.contains("320k"));
        assert!(script.contains("litterbox.catbox.moe"));
        assert!(script.contains("json.dumps(payload)"));
    }

    #[test]
    fn test_generate_remote_script_coros() {
        let script = generate_remote_script(
            VocalSeparationMode::Coros,
            &COROS_MODELS,
            8,
            "pass_coros",
            "/content/song.wav",
        );

        assert!(script.contains("MODE = \"chorus\""));
        assert!(script.contains("OVERLAP = 8"));
        assert!(script.contains("PASSWORD = \"pass_coros\""));
        for m in COROS_MODELS {
            assert!(script.contains(m));
        }
    }
}
