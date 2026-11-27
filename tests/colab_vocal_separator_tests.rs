use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Semaphore;

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::colab_remote_script::generate_remote_script;
use shirley_show::infra::vocal::colab_vocal_separator::{ColabSessionGuard, ColabVocalSeparator};
use shirley_show::infra::vocal::colab_vocal_separator_config::{
    COROS_MODELS, ColabVocalSeparatorConfig, INSTRUMENTAL_MODELS,
};
use shirley_show::ports::vocal_separator::{
    VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn set_env(key: &str, val: &str) {
    unsafe {
        env::set_var(key, val);
    }
}

fn remove_env(key: &str) {
    unsafe {
        env::remove_var(key);
    }
}

#[test]
fn test_colab_config_defaults_and_env_parsing() {
    let _lock = ENV_MUTEX.lock().unwrap();
    remove_env("SHIRLEY_COLAB_BIN");
    remove_env("SHIRLEY_COLAB_TOKEN");
    remove_env("SHIRLEY_COLAB_GPU");
    remove_env("SHIRLEY_COLAB_OVERLAP");
    remove_env("SHIRLEY_COLAB_TIMEOUT_SECS");
    remove_env("SHIRLEY_COLAB_WORK_DIR");

    let gpu = Arc::new(Semaphore::new(1));
    let temp_work = PathBuf::from("/tmp/shirley_test_work");
    let cfg = ColabVocalSeparatorConfig::from_env(temp_work.clone(), gpu.clone());

    assert_eq!(cfg.colab_bin, PathBuf::from("colab"));
    assert_eq!(cfg.gpu_type, "g4");
    assert_eq!(cfg.overlap, 12);
    assert_eq!(cfg.timeout_secs, 1800);
    assert_eq!(cfg.work_dir, temp_work);

    // Custom env overrides
    set_env("SHIRLEY_COLAB_BIN", "/custom/colab");
    set_env("SHIRLEY_COLAB_TOKEN", "/custom/token.json");
    set_env("SHIRLEY_COLAB_GPU", "a100");
    set_env("SHIRLEY_COLAB_OVERLAP", "16");
    set_env("SHIRLEY_COLAB_TIMEOUT_SECS", "3600");
    set_env("SHIRLEY_COLAB_WORK_DIR", "/custom/work");

    let cfg_custom = ColabVocalSeparatorConfig::from_env(temp_work, gpu);
    assert_eq!(cfg_custom.colab_bin, PathBuf::from("/custom/colab"));
    assert_eq!(cfg_custom.token_path, PathBuf::from("/custom/token.json"));
    assert_eq!(cfg_custom.gpu_type, "a100");
    assert_eq!(cfg_custom.overlap, 16);
    assert_eq!(cfg_custom.timeout_secs, 3600);
    assert_eq!(cfg_custom.work_dir, PathBuf::from("/custom/work"));

    // Clean up
    remove_env("SHIRLEY_COLAB_BIN");
    remove_env("SHIRLEY_COLAB_TOKEN");
    remove_env("SHIRLEY_COLAB_GPU");
    remove_env("SHIRLEY_COLAB_OVERLAP");
    remove_env("SHIRLEY_COLAB_TIMEOUT_SECS");
    remove_env("SHIRLEY_COLAB_WORK_DIR");
}

#[test]
fn test_colab_config_model_per_mode() {
    // One model per mode. Measured on the production GPU: averaging four
    // models gave leak 0.0904 against 0.0632 for the single instrument
    // model, so the ensemble made separation worse for four times the GPU
    // time. This test pins the outcome of that measurement, not a preference.
    assert_eq!(INSTRUMENTAL_MODELS.len(), 1);
    assert_eq!(
        INSTRUMENTAL_MODELS[0],
        "model_bs_roformer_ep_317_sdr_12.9755.ckpt"
    );

    assert_eq!(COROS_MODELS.len(), 1);
    assert_eq!(
        COROS_MODELS[0],
        "mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt"
    );

    let inst = ColabVocalSeparatorConfig::models_for_mode(VocalSeparationMode::Instrumental);
    assert_eq!(inst, &INSTRUMENTAL_MODELS);

    let coros = ColabVocalSeparatorConfig::models_for_mode(VocalSeparationMode::Coros);
    assert_eq!(coros, &COROS_MODELS);
}

#[test]
fn test_colab_config_token_validation() {
    let temp = TempDir::new().unwrap();
    let token_path = temp.path().join("token.json");
    let gpu = Arc::new(Semaphore::new(1));

    let mut cfg = ColabVocalSeparatorConfig::from_env(temp.path().to_path_buf(), gpu);
    cfg.token_path = token_path.clone();

    // 1. Non-existent file
    assert_eq!(
        cfg.validate_token().unwrap_err(),
        VocalSeparationError::ColabTokenMissing
    );

    // 2. Invalid JSON
    std::fs::write(&token_path, "not-json").unwrap();
    assert_eq!(
        cfg.validate_token().unwrap_err(),
        VocalSeparationError::ColabTokenMissing
    );

    // 3. JSON missing refresh_token
    std::fs::write(
        &token_path,
        r#"{"access_token":"abc","expiry":"2025-01-01"}"#,
    )
    .unwrap();
    assert_eq!(
        cfg.validate_token().unwrap_err(),
        VocalSeparationError::ColabTokenMissing
    );

    // 4. JSON with empty refresh_token
    std::fs::write(
        &token_path,
        r#"{"access_token":"abc","refresh_token":"  ","expiry":"2025-01-01"}"#,
    )
    .unwrap();
    assert_eq!(
        cfg.validate_token().unwrap_err(),
        VocalSeparationError::ColabTokenMissing
    );

    // 5. Valid refresh_token
    std::fs::write(
        &token_path,
        r#"{"access_token":"abc","refresh_token":"valid_refresh","expiry":"2025-01-01"}"#,
    )
    .unwrap();
    assert!(cfg.validate_token().is_ok());
}

#[test]
fn test_remote_script_generation_content() {
    let script = generate_remote_script(
        VocalSeparationMode::Instrumental,
        &INSTRUMENTAL_MODELS,
        12,
        8,
        "super_secret_pw",
        "/content/my_input.wav",
    );

    // Check models
    for m in INSTRUMENTAL_MODELS {
        assert!(script.contains(m), "Script missing model {m}");
    }
    // Check overlap
    assert!(script.contains("OVERLAP = 12") || script.contains("--mdxc_overlap"));
    // Check no password leaks into the script
    assert!(!script.contains("super_secret_pw"));
    assert!(!script.contains("password"));
    // Check conditional import of audio-separator
    assert!(script.contains("audio_separator"));
    assert!(script.contains("ImportError"));
    // Check stem classification by parenthetical content
    assert!(script.contains(r"\(([^)]+)\)"));
    // Check averaging using numpy and soundfile
    assert!(script.contains("soundfile") || script.contains("sf."));
    assert!(script.contains("np.mean") || script.contains("numpy"));
    // Check ffmpeg mp3 320k encoding
    assert!(script.contains("libmp3lame"));
    assert!(script.contains("320k"));
    // Check no openssl encryption, catbox or upload URL in the script
    assert!(!script.contains("openssl"));
    assert!(!script.contains("catbox"));
    assert!(!script.contains("litterbox"));
    assert!(!script.contains("http"));
    // Check single JSON output line
    assert!(script.contains("json.dumps"));
    assert!(script.contains("\"url\""));
    assert!(script.contains("\"sha256\""));
    assert!(script.contains("\"duration\""));
    assert!(script.contains("\"size\""));
}

#[test]
fn test_remote_script_generation_coros_mode() {
    let script = generate_remote_script(
        VocalSeparationMode::Coros,
        &COROS_MODELS,
        8,
        8,
        "another_pass",
        "/content/audio.mp3",
    );

    for m in COROS_MODELS {
        assert!(script.contains(m), "Script missing coros model {m}");
    }
    assert!(!script.contains("another_pass"));
    assert!(!script.contains("password"));
    assert!(!script.contains("openssl"));
    assert!(!script.contains("catbox"));
    assert!(!script.contains("litterbox"));
    assert!(!script.contains("http"));
    assert!(script.contains("\"url\""));
    assert!(script.contains("\"sha256\""));
    assert!(script.contains("\"duration\""));
    assert!(script.contains("\"size\""));
}

#[test]
fn test_colab_vocal_separator_is_remote() {
    let temp = TempDir::new().unwrap();
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = ColabVocalSeparatorConfig::from_env(temp.path().to_path_buf(), gpu);
    let sep = ColabVocalSeparator::new(cfg);
    assert!(sep.is_remote());
}

#[tokio::test]
async fn test_colab_vocal_separator_missing_source_fails_fast() {
    let temp = TempDir::new().unwrap();
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = ColabVocalSeparatorConfig::from_env(temp.path().to_path_buf(), gpu);
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: PathBuf::from("/non/existent/audio.mp3"),
        output_path: temp.path().join("output.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::InvalidAudioDuration);
}

#[tokio::test]
async fn test_colab_vocal_separator_missing_token_fails_without_gpu() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("input.mp3");
    std::fs::write(&source_path, b"dummy audio content").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = ColabVocalSeparatorConfig::from_env(temp.path().to_path_buf(), gpu.clone());
    cfg.token_path = temp.path().join("nonexistent_token.json");

    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path,
        output_path: temp.path().join("output.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::ColabTokenMissing);
    // Permit was NOT acquired / was preserved
    assert_eq!(gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_colab_vocal_separator_gpu_busy_when_semaphore_occupied() {
    let temp = TempDir::new().unwrap();
    let source_path = temp.path().join("input.mp3");
    std::fs::write(&source_path, b"dummy audio content").unwrap();

    let token_path = temp.path().join("token.json");
    std::fs::write(&token_path, r#"{"refresh_token":"abc"}"#).unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    // Acquire the permit externally
    let _held_permit = gpu.clone().try_acquire_owned().unwrap();

    let mut cfg = ColabVocalSeparatorConfig::from_env(temp.path().to_path_buf(), gpu.clone());
    cfg.token_path = token_path;

    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path,
        output_path: temp.path().join("output.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::GpuBusy);
}

#[test]
fn test_colab_session_guard_drop_spawns_stop_when_active() {
    let temp = TempDir::new().unwrap();
    let log_file = temp.path().join("stop_invoked.log");

    // Create a mock colab script that logs when stop is invoked
    let mock_colab = temp.path().join("mock_colab.sh");
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = "stop" ]; then
    echo "stopped $3" >> "{}"
fi
"#,
        log_file.display()
    );
    std::fs::write(&mock_colab, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&mock_colab).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&mock_colab, perms).unwrap();
    }

    // When dropped while active, guard must spawn colab stop
    {
        let _guard = ColabSessionGuard::new(mock_colab.clone(), "test-sess-123".to_string());
    }

    // Give process a moment to run
    std::thread::sleep(Duration::from_millis(50));
    let content = std::fs::read_to_string(&log_file).unwrap_or_default();
    assert!(
        content.contains("stopped test-sess-123"),
        "Expected colab stop to be called on drop"
    );

    // When deactivated, guard must NOT call colab stop
    let log_file2 = temp.path().join("stop_invoked_2.log");
    let mock_colab2 = temp.path().join("mock_colab_2.sh");
    let script2 = format!(
        r#"#!/bin/sh
if [ "$1" = "stop" ]; then
    echo "stopped $3" >> "{}"
fi
"#,
        log_file2.display()
    );
    std::fs::write(&mock_colab2, script2).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&mock_colab2).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&mock_colab2, perms).unwrap();
    }

    {
        let mut guard2 = ColabSessionGuard::new(mock_colab2, "test-sess-456".to_string());
        guard2.deactivate();
    }
    std::thread::sleep(Duration::from_millis(50));
    assert!(
        !log_file2.exists() || std::fs::read_to_string(&log_file2).unwrap().is_empty(),
        "Deactivated guard must not spawn colab stop"
    );
}

fn write_silence_wav(path: &Path, duration_secs: u32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    let num_samples = 44100 * duration_secs * 2;
    for _ in 0..num_samples {
        writer.write_sample(0i16).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

struct MockColabFixture {
    temp_dir: TempDir,
    colab_bin: PathBuf,
    log_file: PathBuf,
    trigger_fail_start: PathBuf,
    /// Holds a GPU name; `start --gpu <name>` fails only for that accelerator,
    /// which is how a per-GPU assignment outage is simulated.
    trigger_fail_gpu: PathBuf,
    /// Holds a GPU name; `new --gpu <name>` for it never answers until the
    /// client's per-attempt timeout cuts it off, and then assigns late (exit 0).
    /// This is the production shape: the endpoint was still working after the
    /// client gave up.
    trigger_hang_gpu: PathBuf,
    /// When present, `new` refuses every accelerator the CLI accepts, which is a
    /// chain-wide capacity outage rather than a per-GPU one.
    trigger_all_gpus_busy: PathBuf,
    trigger_fail_exec: PathBuf,
    /// Holds the exact stderr the failing `exec` must print, so a test can pin
    /// what the one-line diagnostic keeps (last line, single line, truncated).
    trigger_fail_exec_stderr: PathBuf,
    /// When present, `exec` fails exactly like the real CLI does on a silent
    /// kernel: exit 1 and `TimeoutError: ... no message from the kernel for
    /// 30s` on stderr, unless a silence budget above the CLI default was passed
    /// with `--timeout N`. This is the production failure, reproduced.
    trigger_require_cli_timeout: PathBuf,
    trigger_bad_sha: PathBuf,
    trigger_bad_duration: PathBuf,
    /// When present, `stop` exits 1 with "Session not found" on stderr, which
    /// the adapter must treat as a successful cleanup.
    trigger_stop_not_found: PathBuf,
    sample_wav: PathBuf,
    token_file: PathBuf,
}

impl MockColabFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let colab_bin = temp_dir.path().join("mock_colab.sh");
        let log_file = temp_dir.path().join("colab_calls.log");
        let trigger_fail_start = temp_dir.path().join("fail_start");
        let trigger_fail_gpu = temp_dir.path().join("fail_gpu");
        let trigger_hang_gpu = temp_dir.path().join("hang_gpu");
        let trigger_all_gpus_busy = temp_dir.path().join("all_gpus_busy");
        let trigger_fail_exec = temp_dir.path().join("fail_exec");
        let trigger_fail_exec_stderr = temp_dir.path().join("fail_exec_stderr");
        let trigger_require_cli_timeout = temp_dir.path().join("require_cli_timeout");
        let trigger_bad_sha = temp_dir.path().join("bad_sha");
        let trigger_bad_duration = temp_dir.path().join("bad_duration");
        let trigger_stop_not_found = temp_dir.path().join("stop_not_found");

        let sample_wav = temp_dir.path().join("sample_input.wav");
        write_silence_wav(&sample_wav, 1);

        let token_file = temp_dir.path().join("token.json");
        std::fs::write(&token_file, r#"{"refresh_token":"valid_token_123"}"#).unwrap();

        // Keep the official CLI's legacy token copy inside this scratch dir, not
        // the real user config, when `ensure_legacy_token` runs during `separate`.
        unsafe {
            std::env::set_var(
                "SHIRLEY_COLAB_LEGACY_TOKEN",
                temp_dir.path().join("colab-cli/token.json"),
            );
        }

        let log_str = log_file.display().to_string();
        let fail_start_str = trigger_fail_start.display().to_string();
        let fail_gpu_str = trigger_fail_gpu.display().to_string();
        let hang_gpu_str = trigger_hang_gpu.display().to_string();
        let all_gpus_busy_str = trigger_all_gpus_busy.display().to_string();
        let fail_exec_str = trigger_fail_exec.display().to_string();
        let fail_exec_stderr_str = trigger_fail_exec_stderr.display().to_string();
        let require_timeout_str = trigger_require_cli_timeout.display().to_string();
        let bad_sha_str = trigger_bad_sha.display().to_string();
        let bad_dur_str = trigger_bad_duration.display().to_string();
        let stop_not_found_str = trigger_stop_not_found.display().to_string();
        let temp_dir_str = temp_dir.path().display().to_string();

        let script = format!(
            r#"#!/bin/sh
cmd="$1"
if [ "$cmd" = "new" ]; then
    # Every assignment attempt is logged before any decision, so a test can pin
    # the ORDER in which the accelerators are asked without depending on which
    # one eventually assigns.
    echo "attempt $@" >> "{log_str}"
    if [ -f "{fail_start_str}" ]; then
        exit 1
    fi
    # The real CLI rejects values it does not know: "unsupported value '14'.
    # Supported: T4, L4, G4, H100, A100." An attempt spent on one is an attempt
    # wasted, so the mock refuses them exactly like the CLI does.
    prev=""
    gpu_req=""
    for a in "$@"; do
        if [ "$prev" = "--gpu" ]; then gpu_req="$a"; fi
        prev="$a"
    done
    case "$gpu_req" in
        T4|L4|G4|H100|A100) ;;
        *)
            echo "badgpu $@" >> "{log_str}"
            echo "unsupported value '$gpu_req'. Supported: T4, L4, G4, H100, A100." >&2
            exit 1
            ;;
    esac
    if [ -f "{all_gpus_busy_str}" ]; then
        case "$*" in
            *"--gpu T4"*|*"--gpu L4"*|*"--gpu G4"*|*"--gpu H100"*|*"--gpu A100"*) exit 1 ;;
        esac
    fi
    if [ -f "{hang_gpu_str}" ]; then
        while IFS= read -r hung || [ -n "$hung" ]; do
            [ -z "$hung" ] && continue
            hung_upper=$(printf '%s' "$hung" | tr 'a-z' 'A-Z')
            case "$*" in
                *"--gpu $hung_upper"*)
                    # Recorded before hanging so the test can name the session
                    # that must be stopped before the chain rotates.
                    echo "hangnew $@" >> "{log_str}"
                    sleep 10
                    exit 0
                    ;;
            esac
        done < "{hang_gpu_str}"
    fi
    if [ -f "{fail_gpu_str}" ]; then
        while IFS= read -r broken || [ -n "$broken" ]; do
            [ -z "$broken" ] && continue
            broken_upper=$(printf '%s' "$broken" | tr 'a-z' 'A-Z')
            case "$*" in
                *"--gpu $broken_upper"*) exit 1 ;;
            esac
        done < "{fail_gpu_str}"
    fi
    echo "new $@" >> "{log_str}"
    exit 0
elif [ "$cmd" = "upload" ]; then
    echo "upload $@" >> "{log_str}"
    for arg in "$@"; do
        if [ -f "$arg" ]; then
            cp "$arg" "{temp_dir_str}/uploaded"
        fi
    done
    # Exit 1 on purpose: the real CLI reports "kernel write failed" while still
    # leaving a byte-perfect file. The pipeline must accept that.
    exit 1
elif [ "$cmd" = "exec" ]; then
    # Extract the script file (last argument)
    for last; do true; done
    script_file="$last"

    # The upload is judged by what the runtime holds, so the fake answers the
    # size probe when it sees one. The probe is itself a script executed with
    # `-f`, so it is detected by content, not by flag shape.
    is_probe=0
    if grep -q "SIZE=" "$script_file" 2>/dev/null; then is_probe=1; fi

    # The real CLI aborts a call whose kernel prints nothing for `--timeout`
    # seconds (default 30) and reports it on stderr. The production run died
    # exactly that way while the script was installing packages and loading the
    # models, so the mock refuses the same calls unless a larger budget was
    # requested.
    silence=""
    prev=""
    for a in "$@"; do
        if [ "$prev" = "--timeout" ]; then silence="$a"; fi
        prev="$a"
    done
    too_short=1
    case "$silence" in
        ""|*[!0-9]*) too_short=1 ;;
        *) if [ "$silence" -gt 30 ]; then too_short=0; fi ;;
    esac
    if [ -f "{require_timeout_str}" ] && [ "$too_short" = "1" ]; then
        echo "execfail $@" >> "{log_str}"
        echo "Traceback (most recent call last):" >&2
        echo "TimeoutError: Timeout waiting for output: no message from the kernel for 30s. Use --timeout to allow longer silences." >&2
        exit 1
    fi
    if [ "$is_probe" = "0" ] && [ -f "{fail_exec_str}" ]; then
        # A real exec failure carries a traceback on stderr; the adapter must
        # leave a diagnosable trace of it without leaking the token.
        if [ -f "{fail_exec_stderr_str}" ]; then
            cat "{fail_exec_stderr_str}" >&2
        else
            echo "Traceback (most recent call last):" >&2
            echo "TimeoutError: Timeout waiting for output: no message from the kernel for 30s. Use --timeout to allow longer silences." >&2
        fi
        exit 1
    fi
    echo "exec $@" >> "{log_str}"

    if [ "$is_probe" = "1" ]; then
        if [ -f "{temp_dir_str}/uploaded" ]; then
            echo "SIZE=$(wc -c < "{temp_dir_str}/uploaded" | tr -d ' ')"
        else
            echo "SIZE=absent"
        fi
        exit 0
    fi

    PASSWORD=$(python3 -c 'import re, sys; m=re.search(r"PASSWORD\s*=\s*\"([^\"]+)\"", open(sys.argv[1]).read()); print(m.group(1) if m else "")' "$script_file")

    mock_mp3="{temp_dir_str}/mock_generated.mp3"
    ffmpeg -y -v error -f lavfi -i anullsrc=r=44100:cl=stereo -t 1 -codec:a libmp3lame -b:a 320k "$mock_mp3"

    mock_enc="{temp_dir_str}/mock_generated.mp3.enc"
    openssl enc -aes-256-cbc -pbkdf2 -salt -pass "pass:$PASSWORD" -in "$mock_mp3" -out "$mock_enc"

    if [ -f "{bad_sha_str}" ]; then
        SHA="0000000000000000000000000000000000000000000000000000000000000000"
    else
        SHA=$(openssl dgst -sha256 -r "$mock_mp3" | awk '{{print $1}}')
    fi

    if [ -f "{bad_dur_str}" ]; then
        DUR="999.0"
    else
        DUR="1.0"
    fi

    SIZE=$(wc -c < "$mock_mp3" | awk '{{print $1}}')

    echo "{{\"url\":\"file://$mock_enc\",\"sha256\":\"$SHA\",\"duration\":$DUR,\"size\":$SIZE}}"
    exit 0
elif [ "$cmd" = "status" ]; then
    echo "status $@" >> "{log_str}"
    exit 0
elif [ "$cmd" = "download" ]; then
    echo "download $@" >> "{log_str}"
    for last; do true; done
    dest="$last"
    cp "{temp_dir_str}/mock_generated.mp3" "$dest"
    exit 0
elif [ "$cmd" = "stop" ]; then
    echo "stop $@" >> "{log_str}"
    if [ -f "{stop_not_found_str}" ]; then
        echo "Session not found" >&2
        exit 1
    fi
    exit 0
else
    echo "unknown command $cmd" >> "{log_str}"
    exit 1
fi
"#
        );

        std::fs::write(&colab_bin, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&colab_bin).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&colab_bin, perms).unwrap();
        }

        Self {
            temp_dir,
            colab_bin,
            log_file,
            trigger_fail_start,
            trigger_fail_gpu,
            trigger_hang_gpu,
            trigger_all_gpus_busy,
            trigger_fail_exec,
            trigger_fail_exec_stderr,
            trigger_require_cli_timeout,
            trigger_bad_sha,
            trigger_bad_duration,
            trigger_stop_not_found,
            sample_wav,
            token_file,
        }
    }

    fn make_config(&self, gpu: Arc<Semaphore>) -> ColabVocalSeparatorConfig {
        let mut cfg = ColabVocalSeparatorConfig::from_env(self.temp_dir.path().to_path_buf(), gpu);
        cfg.colab_bin = self.colab_bin.clone();
        cfg.token_path = self.token_file.clone();
        cfg
    }

    fn read_log(&self) -> String {
        std::fs::read_to_string(&self.log_file).unwrap_or_default()
    }
}

#[tokio::test]
async fn test_colab_vocal_separator_probe() {
    let fixture = MockColabEnv::new_fixture();
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu);
    let sep = ColabVocalSeparator::new(cfg);

    assert!(sep.probe(Duration::from_secs(5)).await);

    // Corrupt token
    std::fs::write(&fixture.token_file, "not-json").unwrap();
    assert!(!sep.probe(Duration::from_secs(5)).await);
}

type MockColabEnv = MockColabFixture;
impl MockColabFixture {
    fn new_fixture() -> Self {
        Self::new()
    }
}

#[tokio::test]
async fn test_colab_vocal_separator_full_successful_pipeline_lifecycle() {
    let fixture = MockColabEnv::new_fixture();
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3.clone(),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await.expect("separation should succeed");
    assert_eq!(res.duration_secs, 1);
    assert!(res.size_bytes > 0);
    assert!(output_mp3.is_file());

    // Lifecycle on success is now new -> upload -> exec -> download, and the
    // stop is deferred: the machine is kept for the retention window so a
    // second request reuses it. Asserting "stop happened" here would pin the
    // old contract, in which every request paid a cold start.
    let log = fixture.read_log();
    assert!(log.contains("new"), "Must call colab new");
    assert!(log.contains("upload"), "Must call colab upload");
    assert!(log.contains("exec"), "Must call colab exec");
    assert!(log.contains("download"), "Must call colab download");
    assert!(
        !log.contains("stop"),
        "a successful run must keep the machine for the retention window, not stop it"
    );

    // Ensure permit was released
    assert_eq!(gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_colab_vocal_separator_start_failure_returns_colab_unavailable() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_start, b"fail").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::ColabUnavailable);
    assert_eq!(gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_colab_vocal_separator_exec_failure_still_runs_stop() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_exec, b"fail").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::SeparationFailed);

    let log = fixture.read_log();
    assert!(log.contains("new"));
    assert!(log.contains("stop"), "Must call stop even on exec failure");
    assert_eq!(gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_colab_vocal_separator_sha256_mismatch_fails_and_runs_stop() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_bad_sha, b"bad").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(
        res.unwrap_err(),
        VocalSeparationError::InvalidSeparatorOutput
    );

    let log = fixture.read_log();
    assert!(log.contains("stop"), "Must call stop on sha256 mismatch");
    assert_eq!(gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_colab_vocal_separator_duration_mismatch_fails_and_runs_stop() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_bad_duration, b"bad").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(
        res.unwrap_err(),
        VocalSeparationError::InvalidSeparatorOutput
    );

    let log = fixture.read_log();
    assert!(log.contains("stop"), "Must call stop on duration mismatch");
    assert_eq!(gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_colab_vocal_separator_duration_exceeded_before_gpu() {
    let fixture = MockColabEnv::new_fixture();
    let long_wav = fixture.temp_dir.path().join("long_audio.wav");
    // Write 605 seconds silence wav (> 600s)
    write_silence_wav(&long_wav, 605);

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: long_wav,
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::DurationExceeded);

    // Verify colab new was NEVER called
    let log = fixture.read_log();
    assert!(
        !log.contains("new"),
        "colab new must not be called when duration > 600s"
    );
    assert_eq!(gpu.available_permits(), 1);
}

#[test]
fn test_remote_script_treats_no_vocals_as_instrumental() {
    let script = generate_remote_script(
        VocalSeparationMode::Instrumental,
        &INSTRUMENTAL_MODELS,
        12,
        8,
        "pw",
        "/content/input.wav",
    );
    // "(no_vocals)" contains the substring "vocal"; it must still be accepted
    // as a backing stem, in both the primary check and the fallback.
    assert!(
        script.contains("if label in (\"no_vocals\", \"accompaniment\")"),
        "no_vocals must be classified before the vocal substring check"
    );
    assert!(
        script.contains("label != \"no_vocals\""),
        "the fallback must not discard no_vocals stems"
    );
}

#[tokio::test]
async fn test_start_failure_does_not_issue_stop() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("calls.log");
    let colab_bin = temp_dir.path().join("mock_colab.sh");
    let sample_wav = temp_dir.path().join("in.wav");
    write_silence_wav(&sample_wav, 1);
    let token_file = temp_dir.path().join("token.json");
    std::fs::write(&token_file, r#"{"refresh_token":"t"}"#).unwrap();

    // The session name is generated client-side, so a failing `new` means no
    // session was allocated: the adapter must return ColabUnavailable and must
    // NOT issue any stop (there is nothing to free, and bare `colab stop` is
    // forbidden by the official CLI contract).
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{}"
if [ "$1" = "new" ]; then exit 1; fi
exit 0
"#,
        log_file.display()
    );
    std::fs::write(&colab_bin, script).unwrap();
    let mut perms = std::fs::metadata(&colab_bin).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&colab_bin, perms).unwrap();

    // Keep the legacy token copy inside the scratch dir.
    unsafe {
        std::env::set_var(
            "SHIRLEY_COLAB_LEGACY_TOKEN",
            temp_dir.path().join("colab-cli/token.json"),
        );
    }

    let mut cfg = ColabVocalSeparatorConfig::from_env(
        temp_dir.path().to_path_buf(),
        Arc::new(Semaphore::new(1)),
    );
    cfg.colab_bin = colab_bin;
    cfg.token_path = token_file;
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: sample_wav,
        output_path: temp_dir.path().join("out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(30),
    };

    let result = sep.separate(req).await;
    assert!(matches!(
        result,
        Err(VocalSeparationError::ColabUnavailable)
    ));

    let log = std::fs::read_to_string(&log_file).unwrap_or_default();
    assert!(
        !log.contains("stop"),
        "a session-less stop must never run when `new` failed, got: {log}"
    );
}

/// Colab can list an accelerator as available while its assignment endpoint
/// hangs or refuses, so the primary GPU failing must not fail the separation
/// outright. The next candidate in the performance ranking is tried, and the run
/// must land on it.
#[tokio::test]
async fn test_start_falls_back_to_secondary_gpu_when_primary_fails_to_assign() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_gpu, b"g4").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = vec!["l4".to_string()];
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("fallback_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(120),
    };

    let res = sep.separate(req).await;
    assert!(
        res.is_ok(),
        "the fallback GPU must carry the separation, got: {res:?}"
    );

    let log = fixture.read_log();
    let starts: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with("new "))
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "the broken GPU must not be logged as started, log was: {log}"
    );
    assert!(
        starts[0].contains("--gpu H100"),
        "the run must land on the next fastest GPU, log was: {log}"
    );
    assert_eq!(gpu.available_permits(), 1, "the GPU permit must be released");
}

/// Negative control for the test above: when every accelerator in the ranking
/// refuses, the request must still surface as ColabUnavailable. Without this,
/// the fallback test could pass for the wrong reason.
#[tokio::test]
async fn test_start_without_fallback_still_reports_colab_unavailable() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_gpu, b"g4\nh100\na100\nl4").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = Vec::new();
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("no_fallback_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(120),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::ColabUnavailable);
    assert_eq!(gpu.available_permits(), 1);
}

/// Which accelerators answer changes within the hour: G4 and A100 both refused
/// in the same window that L4 answered. A single fallback only moves the
/// failure one step, so the chain must keep going until one assigns.
#[tokio::test]
async fn test_start_walks_the_whole_gpu_chain_until_one_assigns() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_gpu, b"g4\nh100\na100\n").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = vec!["a100".to_string(), "l4".to_string()];
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("chain_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(120),
    };

    let res = sep.separate(req).await;
    assert!(res.is_ok(), "the chain must reach a working GPU, got: {res:?}");

    let log = fixture.read_log();
    let starts: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with("new "))
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "only the GPU that assigned may be logged as started, log was: {log}"
    );
    assert!(
        starts[0].contains("--gpu L4"),
        "the chain must end on the working GPU, log was: {log}"
    );
    assert_eq!(gpu.available_permits(), 1);
}

/// The official CLI contract, pinned through the mock log: `new -s NAME --gpu
/// GPU` (uppercase GPU), `upload -s`, `exec -s -f`, `download -s`, and stop
/// never used without `-s`.
#[tokio::test]
async fn test_command_arguments_match_official_cli() {
    let fixture = MockColabEnv::new_fixture();
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu);
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    sep.separate(req).await.expect("separation should succeed");

    let log = fixture.read_log();

    let new_line = log
        .lines()
        .find(|l| l.starts_with("new "))
        .expect("new called");
    assert!(new_line.contains("-s"), "new must pass -s, got: {new_line}");
    assert!(
        new_line.contains("shirley-"),
        "session name must be generated as shirley-<hex>, got: {new_line}"
    );
    assert!(
        new_line.contains("--gpu G4"),
        "gpu must be uppercase when passed to the CLI, got: {new_line}"
    );

    let upload_line = log
        .lines()
        .find(|l| l.starts_with("upload "))
        .expect("upload called");
    assert!(upload_line.contains("-s"), "upload must pass -s, got: {upload_line}");

    let exec_line = log
        .lines()
        .find(|l| l.starts_with("exec ") && l.contains("colab_script_"))
        .expect("exec of the separation script called");
    assert!(exec_line.contains("-s"), "exec must pass -s, got: {exec_line}");
    assert!(exec_line.contains("-f"), "exec must pass the script with -f, got: {exec_line}");
    assert!(exec_line.contains("colab_script_"), "exec must run the generated script, got: {exec_line}");
    assert!(!exec_line.contains("--json"), "exec must not use --json, got: {exec_line}");
    assert!(!exec_line.contains("--session"), "no legacy --session flag, got: {exec_line}");
    let tokens: Vec<&str> = exec_line.split_whitespace().collect();
    assert!(
        !tokens.contains(&"-c"),
        "no inline -c flag, got: {exec_line}"
    );

    let download_line = log
        .lines()
        .find(|l| l.starts_with("download "))
        .expect("download called");
    assert!(download_line.contains("-s"), "download must pass -s, got: {download_line}");
}

/// A `colab stop` outcome of exit 0, or of exit 1 with a "not found" message
/// (the session the already gone), is a successful cleanup. Anything else is a
/// real failure.
#[cfg(unix)]
#[test]
fn test_stop_ok_treats_not_found_as_success() {
    use std::os::unix::process::ExitStatusExt;
    use shirley_show::infra::vocal::colab_orphan_reaper::stop_ok;

    let ok = std::process::ExitStatus::from_raw(0);
    assert!(stop_ok(&ok, b""));

    let not_found = std::process::ExitStatus::from_raw(1 << 8);
    assert!(stop_ok(&not_found, b"Session not found"));
    assert!(stop_ok(&not_found, b"nothing here; session not found"));

    assert!(!stop_ok(&not_found, b"Permission denied"));
    let other = std::process::ExitStatus::from_raw(2 << 8);
    assert!(!stop_ok(&other, b""));
}

/// When `stop` reports the session is already gone (exit 1, "not found"), the
/// adapter treats it as a successful cleanup: the pipeline returns the real
/// separation error (InvalidSeparatorOutput), not a masked SeparationFailed.
#[tokio::test]
async fn test_stop_on_missing_session_is_treated_as_success() {
    let fixture = MockColabEnv::new_fixture();
    // The separation itself fails on sha mismatch (InvalidSeparatorOutput), and
    // the subsequent stop reports the session is gone.
    std::fs::write(&fixture.trigger_bad_sha, b"bad").unwrap();
    std::fs::write(&fixture.trigger_stop_not_found, b"gone").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("final_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::InvalidSeparatorOutput);
    assert_eq!(gpu.available_permits(), 1);

    let log = fixture.read_log();
    assert!(log.contains("stop"), "stop must be attempted on error");
}

/// The legacy token copy must never overwrite a newer destination: the CLI
/// rewrites its token file when it refreshes the access token.
#[test]
fn test_token_sync_does_not_overwrite_newer_destination() {
    use std::time::{Duration, SystemTime};
    use shirley_show::infra::vocal::colab_vocal_separator_config::sync_token_to_legacy;

    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src_token.json");
    let dst = temp.path().join("colab-cli/token.json");
    std::fs::write(&src, r#"{"refresh_token":"source"}"#).unwrap();
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::write(&dst, r#"{"refresh_token":"destination"}"#).unwrap();

    // Make the destination newer than the source.
    let src_file = std::fs::File::open(&src).unwrap();
    src_file
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();

    sync_token_to_legacy(&src, &dst).unwrap();

    let dst_contents = std::fs::read_to_string(&dst).unwrap();
    assert!(
        dst_contents.contains("destination"),
        "a newer destination must not be overwritten: {dst_contents}"
    );
}

/// The legacy token is copied when the destination is missing or older, and it
/// lands with 0600 permissions.
#[test]
fn test_token_sync_copies_when_destination_missing_or_older() {
    use std::time::{Duration, SystemTime};
    use shirley_show::infra::vocal::colab_vocal_separator_config::sync_token_to_legacy;

    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src_token.json");
    std::fs::write(&src, r#"{"refresh_token":"source"}"#).unwrap();

    // Missing destination -> copy.
    let dst = temp.path().join("colab-cli/token.json");
    sync_token_to_legacy(&src, &dst).unwrap();
    assert_eq!(
        std::fs::read_to_string(&dst).unwrap(),
        r#"{"refresh_token":"source"}"#
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&dst).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "legacy token must be 0600");
    }

    // Source newer than destination -> overwrite.
    std::fs::write(&src, r#"{"refresh_token":"source-v2"}"#).unwrap();
    let dst_file = std::fs::File::open(&dst).unwrap();
    dst_file
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    sync_token_to_legacy(&src, &dst).unwrap();
    assert_eq!(
        std::fs::read_to_string(&dst).unwrap(),
        r#"{"refresh_token":"source-v2"}"#
    );
}

/// The boot/periodic reaper enumerates session names from the official CLI's
/// sessions.json and stops each by name.
#[test]
fn test_reaper_enumerates_sessions_from_json() {
    use shirley_show::infra::vocal::colab_orphan_reaper::enumerate_session_names;

    let temp = TempDir::new().unwrap();
    let sessions = temp.path().join("sessions.json");
    std::fs::write(&sessions, r#"{"alpha":{},"beta":{},"gamma":{}}"#).unwrap();

    let names = enumerate_session_names(&sessions);
    assert_eq!(names.len(), 3);
    for expected in ["alpha", "beta", "gamma"] {
        assert!(names.contains(&expected.to_string()), "missing {expected}: {names:?}");
    }

    // A missing file yields an empty list rather than a panic.
    assert!(enumerate_session_names(&temp.path().join("nope.json")).is_empty());
}

/// Exact argument-shape test for the official CLI contract, including session
/// reuse: the first run creates a session with `new -s shirley-<hex> --gpu
/// G4`, stores a second run inside the retention window reuses the same id
/// (checking liveness with `status -s`), and no forbidden flags (`--json`,
/// `--public-api`, `--session`, bare `stop`) ever appear.
#[tokio::test]
async fn test_official_cli_arguments_exact_shape_and_session_reuse() {
    let fixture = MockColabEnv::new_fixture();
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu);
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("final_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    // First run creates a session.
    sep.separate(req.clone()).await.expect("first separation should succeed");

    // Second run inside the retention window must reuse the same session id.
    let req2 = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("final_out2.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };
    sep.separate(req2).await.expect("second separation should succeed");

    let log = fixture.read_log();

    // The mock logs `echo "<cmd> $@"`, so the echoed prefix and the full
    // argument list both start with the subcommand: a line is
    // `new new -s shirley-<hex> --gpu G4`. `parts[0]` is the echo prefix,
    // `parts[1]` is the real subcommand, and `parts[2..]` are the arguments.
    let new_lines: Vec<&str> = log.lines().filter(|l| l.starts_with("new ")).collect();
    assert_eq!(new_lines.len(), 1, "the session must be created exactly once, log: {log}");
    let new_parts: Vec<&str> = new_lines[0].split_whitespace().collect();
    assert_eq!(new_parts[1..], ["new", "-s", new_parts[3], "--gpu", "G4"]);
    let session_name = new_parts[3];
    assert!(session_name.starts_with("shirley-"), "session name must be shirley-<hex>, got {session_name}");
    let hex = &session_name["shirley-".len()..];
    assert_eq!(hex.len(), 8, "hex id must be exactly 8 chars, got {hex}");
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "id must be hex, got {hex}");

    // The second run reused the id: its liveness was checked with `status -s <id>`.
    let status_lines: Vec<Vec<&str>> = log
        .lines()
        .filter(|l| l.starts_with("status "))
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(status_lines.len(), 1, "exactly one liveness status, log: {log}");
    assert_eq!(status_lines[0][1..], ["status", "-s", session_name]);

    let upload_lines: Vec<Vec<&str>> = log
        .lines()
        .filter(|l| l.starts_with("upload "))
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(upload_lines.len(), 2, "two uploads (one per run), log: {log}");
    for up in &upload_lines {
        assert_eq!(up[1..], ["upload", "-s", session_name, up[4], "/content/input_audio"]);
    }

    let exec_lines: Vec<Vec<&str>> = log
        .lines()
        .filter(|l| l.starts_with("exec ") && l.contains("colab_script_"))
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(exec_lines.len(), 2, "two separation execs (one per run), log: {log}");
    for el in &exec_lines {
        // PRESERVE: the official contract is still `exec -s NAME -f SCRIPT`, with
        // the CLI silence budget added and nothing else. `--timeout` must come
        // before `-f` (a value the CLI's float parser accepts).
        assert_eq!(
            el[1..],
            ["exec", "-s", session_name, "--timeout", el[5], "-f", el[7]],
            "exec argument shape changed: {el:?}"
        );
    }

    let download_lines: Vec<Vec<&str>> = log
        .lines()
        .filter(|l| l.starts_with("download "))
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(download_lines.len(), 2, "two downloads (one per run), log: {log}");
    for dl in &download_lines {
        assert_eq!(dl[1..], ["download", "-s", session_name, dl[4], dl[5]]);
    }

    // Forbidden forms must never appear.
    assert!(
        log.lines().all(|l| !l.starts_with("stop") || l.contains("-s")),
        "every stop must carry -s, log: {log}"
    );
    assert!(!log.contains("--json"), "--json is forbidden, log: {log}");
    assert!(!log.contains("--public-api"), "--public-api is forbidden, log: {log}");
    assert!(!log.contains("--session"), "--session is forbidden, log: {log}");
}

/// `sync_token_to_legacy` must not overwrite the destination when the mtimes are
/// equal (the destination is neither newer nor older).
#[test]
fn test_token_sync_does_not_overwrite_on_equal_mtime() {
    use std::time::{Duration, SystemTime};
    use shirley_show::infra::vocal::colab_vocal_separator_config::sync_token_to_legacy;

    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src_token.json");
    let dst = temp.path().join("colab-cli/token.json");
    std::fs::write(&src, r#"{"refresh_token":"source"}"#).unwrap();
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::write(&dst, r#"{"refresh_token":"destination"}"#).unwrap();

    // Both files share the exact same mtime: the destination is neither newer
    // nor older, so it must be left untouched.
    let mtime = SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::open(&src).unwrap().set_modified(mtime).unwrap();
    std::fs::File::open(&dst).unwrap().set_modified(mtime).unwrap();

    sync_token_to_legacy(&src, &dst).unwrap();

    assert_eq!(
        std::fs::read_to_string(&dst).unwrap(),
        r#"{"refresh_token":"destination"}"#,
        "equal mtime must not overwrite the destination"
    );
}

/// `sync_token_to_legacy` with source == destination is a no-op: it returns Ok
/// and leaves the file untouched.
#[test]
fn test_token_sync_source_eq_dest_is_noop() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::sync_token_to_legacy;

    let temp = TempDir::new().unwrap();
    let path = temp.path().join("token.json");
    std::fs::write(&path, r#"{"refresh_token":"v"}"#).unwrap();

    assert!(sync_token_to_legacy(&path, &path).is_ok());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        r#"{"refresh_token":"v"}"#
    );
}

/// Boot lease: a live pid in `<data>/colab-runtime-owner.pid` makes
/// `claim_runtime_lease` return true (a foreign owner), which drives
/// `boot_reap_decision` to Keep.
#[cfg(unix)]
#[test]
fn test_boot_lease_live_sibling_claims_and_keeps() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{
        ReapDecision, boot_reap_decision, claim_runtime_lease,
    };

    let temp = TempDir::new().unwrap();
    // pid 1 is always running on unix, so it stands in for a live sibling.
    std::fs::write(temp.path().join("colab-runtime-owner.pid"), "1").unwrap();

    assert!(claim_runtime_lease(temp.path()), "a live pid must claim a foreign owner");
    assert_eq!(boot_reap_decision(true), ReapDecision::Keep);
    assert_eq!(boot_reap_decision(false), ReapDecision::Stop);
}

/// `enumerate_session_names` must parse a sessions.json whose array mixes string
/// entries and object entries carrying a name/session/id field, in addition to
/// the object-keyed form.
#[test]
fn test_reaper_enumerates_object_and_string_entries() {
    use shirley_show::infra::vocal::colab_orphan_reaper::enumerate_session_names;

    let temp = TempDir::new().unwrap();

    // Object form: keys are session names.
    let obj = temp.path().join("sessions_obj.json");
    std::fs::write(&obj, r#"{"alpha":{},"beta":{},"gamma":{}}"#).unwrap();
    let names = enumerate_session_names(&obj);
    assert_eq!(names.len(), 3);
    for expected in ["alpha", "beta", "gamma"] {
        assert!(names.contains(&expected.to_string()), "missing {expected}: {names:?}");
    }

    // Array form mixing strings and objects carrying name/session/id.
    let arr = temp.path().join("sessions_arr.json");
    std::fs::write(
        &arr,
        r#"["sess-one", {"name":"sess-two"}, {"session":"sess-three"}, {"id":"sess-four"}]"#,
    )
    .unwrap();
    let names = enumerate_session_names(&arr);
    assert_eq!(names.len(), 4);
    for expected in ["sess-one", "sess-two", "sess-three", "sess-four"] {
        assert!(names.contains(&expected.to_string()), "missing {expected}: {names:?}");
    }
}

// ---------------------------------------------------------------------------
// Intelligent GPU selection (S5 / L7)
//
// Production evidence behind these tests: the assignment endpoint answered 503
// intermittently for G4, A100 and L4 while `colab usage` still reported 233.09
// units and 0.00/hr, so capacity on one accelerator — not credit — was the
// failure. The old code gave the first candidate `request.timeout - elapsed`,
// so a hanging G4 or A100 spent the whole 600s and the fallbacks never ran; a
// configured value of `14` was rejected by the CLI as unsupported; and an L4
// session that was created after a 120s client timeout turned the next attempt
// into `412 Precondition Failed` because a session was already active.
// ---------------------------------------------------------------------------

/// The chain is normalized to what the CLI accepts, unsupported values (`14`)
/// are skipped, and no accelerator is ever attempted twice.
#[test]
fn test_gpu_chain_normalizes_and_skips_unsupported_values() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::{
        DEFAULT_GPU_CHAIN, SUPPORTED_COLAB_GPUS, gpu_chain, normalize_gpu,
    };

    // The exact list the CLI prints for an invalid value.
    assert_eq!(SUPPORTED_COLAB_GPUS, ["T4", "L4", "G4", "H100", "A100"]);
    assert_eq!(normalize_gpu("g4"), Some("G4".to_string()));
    assert_eq!(normalize_gpu(" A100 "), Some("A100".to_string()));
    assert_eq!(normalize_gpu("h100"), Some("H100".to_string()));
    // `14` is not `L4`: the typo that burned an attempt in production.
    assert_eq!(normalize_gpu("14"), None);
    assert_eq!(normalize_gpu("v100"), None);
    assert_eq!(normalize_gpu(""), None);

    let fallbacks = vec!["14".to_string(), "l4".to_string(), "G4".to_string()];
    let chain = gpu_chain("g4", &fallbacks);
    assert_eq!(chain, vec!["G4", "H100", "A100", "L4"]);
    assert!(!chain.iter().any(|g| g == "14"), "the typo must never be attempted");
    let mut sorted = chain.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), chain.len(), "no accelerator may be attempted twice");

    // When nothing configured is usable the documented chain is used instead of
    // failing a paid request on a configuration typo.
    assert_eq!(gpu_chain("14", &["v100".to_string()]), vec!["G4", "H100", "A100", "L4"]);

    // A configured accelerator the CLI does not accept must not disable the
    // chain either: `14` alone still rotates through the fastest options.
    assert_eq!(gpu_chain("14", &[]), vec!["G4", "H100", "A100", "L4"]);

    // The default chain is the measured performance ranking, T4 excluded.
    assert_eq!(
        DEFAULT_GPU_CHAIN.to_vec(),
        vec!["G4", "H100", "A100", "L4"],
        "the default chain must be the fast-first ranking"
    );
}

/// The candidate order is a PERFORMANCE ranking, never the configuration order.
///
/// Production carries `SHIRLEY_COLAB_GPU=a100` from an old mitigation, and
/// honouring that order would put the slowest usable device first and make every
/// separation wait for it while a faster machine is free.
#[test]
fn test_gpu_chain_is_ordered_by_performance_not_by_configuration() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::gpu_chain;

    // The production configuration today.
    assert_eq!(
        gpu_chain("a100", &["l4".to_string()]),
        vec!["G4", "H100", "A100", "L4"],
        "a configured primary must not outrank a faster accelerator"
    );

    // Any other configuration order still yields the same attempt order.
    assert_eq!(
        gpu_chain("l4", &["a100".to_string(), "g4".to_string()]),
        vec!["G4", "H100", "A100", "L4"]
    );
    // A restricted configuration cannot demote the fastest device either.
    assert_eq!(gpu_chain("h100", &[]), vec!["G4", "H100", "A100", "L4"]);
    // T4 is excluded by default: these models do not fit in the pipeline
    // reserve on it. Naming it explicitly is the only way to reach it, and even
    // then it goes last because it is the slowest device.
    assert_eq!(
        gpu_chain("t4", &["l4".to_string()]),
        vec!["G4", "H100", "A100", "L4", "T4"]
    );
    // Unsupported configured values neither break the chain nor add an attempt.
    assert_eq!(gpu_chain("14", &["v100".to_string()]), vec!["G4", "H100", "A100", "L4"]);
    // Every extra supported accelerator is reached even when it is configured
    // nowhere: a busy configured pair must not be the whole rotation.
    assert_eq!(gpu_chain("g4", &[]), vec!["G4", "H100", "A100", "L4"]);
}

/// An end-to-end pin of the attempt order, driven by the environment exactly as
/// production has it (`SHIRLEY_COLAB_GPU=a100`): the first `new` must ask for G4,
/// then H100, then A100, then L4. The default fixture has every other GPU
/// refusing, so all four are really attempted and the run lands on L4.
#[tokio::test]
async fn test_assignment_asks_g4_then_h100_then_a100_then_l4_with_a100_configured() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_gpu, b"g4\nh100\na100").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "a100".to_string();
    cfg.gpu_fallbacks = vec!["l4".to_string()];
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("order_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert!(res.is_ok(), "the chain must reach a working GPU, got: {res:?}");

    let log = fixture.read_log();
    let attempts: Vec<String> = log
        .lines()
        .filter(|l| l.starts_with("attempt "))
        .map(|l| {
            let mut it = l.split_whitespace();
            let gpu = it
                .by_ref()
                .skip_while(|tok| *tok != "--gpu")
                .nth(1)
                .unwrap_or("<missing>")
                .to_string();
            gpu
        })
        .collect();
    assert_eq!(
        attempts,
        vec!["G4", "H100", "A100", "L4"],
        "attempts must follow the performance ranking, log was: {log}"
    );
    assert_eq!(gpu.available_permits(), 1, "the GPU permit must be released");
}

/// A configured fallback value the CLI does not accept must not be treated as an
/// empty chain: production carries `14`, which left the primary as the only
/// option. Unusable values fall back to the documented chain.
#[test]
fn test_config_treats_unusable_fallback_value_as_unset() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let temp = TempDir::new().unwrap();

    remove_env("SHIRLEY_COLAB_GPU");
    set_env("SHIRLEY_COLAB_GPU_FALLBACK", "14");
    let cfg = ColabVocalSeparatorConfig::from_env(
        temp.path().to_path_buf(),
        Arc::new(Semaphore::new(1)),
    );
    assert_eq!(
        cfg.gpu_fallbacks,
        vec!["a100".to_string(), "l4".to_string()],
        "an unusable value must not disable the chain"
    );

    // A mix keeps the usable part and drops the typo.
    set_env("SHIRLEY_COLAB_GPU_FALLBACK", "14, l4");
    let cfg2 = ColabVocalSeparatorConfig::from_env(
        temp.path().to_path_buf(),
        Arc::new(Semaphore::new(1)),
    );
    assert_eq!(cfg2.gpu_fallbacks, vec!["14".to_string(), "l4".to_string()]);
    assert!(
        !shirley_show::infra::vocal::colab_vocal_separator_config::gpu_chain(
            &cfg2.gpu_type,
            &cfg2.gpu_fallbacks
        )
        .iter()
        .any(|gpu| gpu == "14"),
        "the typo must never become an attempt"
    );

    remove_env("SHIRLEY_COLAB_GPU_FALLBACK");
}

/// The budget is split: assignment may not consume what upload/exec/download
/// still need, and each attempt is bounded well below the whole request.
#[test]
fn test_gpu_attempt_budget_leaves_a_pipeline_reserve() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::{
        GPU_ATTEMPT_TIMEOUT_DEFAULT, MIN_USEFUL_GPU_ATTEMPT, gpu_assign_window,
        gpu_attempt_timeout, gpu_start_budget, pipeline_reserve,
    };

    let request = Duration::from_secs(600);
    assert_eq!(pipeline_reserve(request), Duration::from_secs(300));
    assert_eq!(gpu_start_budget(request), Duration::from_secs(300));
    assert_eq!(gpu_assign_window(request), Duration::from_secs(300));

    // The whole old failure: one candidate may never take the whole request.
    let window = gpu_assign_window(request);
    let attempt = gpu_attempt_timeout(window, GPU_ATTEMPT_TIMEOUT_DEFAULT);
    assert_eq!(attempt, GPU_ATTEMPT_TIMEOUT_DEFAULT);
    assert!(
        attempt < request,
        "a single accelerator attempt must not own the request budget"
    );

    // An attempt is capped by the window still left, never by the whole request.
    let tight = Duration::from_secs(40);
    assert_eq!(gpu_attempt_timeout(tight, GPU_ATTEMPT_TIMEOUT_DEFAULT), Duration::from_secs(40));
    assert_eq!(gpu_attempt_timeout(Duration::from_secs(7), GPU_ATTEMPT_TIMEOUT_DEFAULT), Duration::from_secs(7));

    // Below the minimum the window cannot pay for a meaningful attempt, so none
    // is started; at exactly the minimum one still is.
    assert_eq!(
        gpu_attempt_timeout(
            MIN_USEFUL_GPU_ATTEMPT - Duration::from_millis(1),
            GPU_ATTEMPT_TIMEOUT_DEFAULT
        ),
        Duration::ZERO
    );
    assert_eq!(
        gpu_attempt_timeout(MIN_USEFUL_GPU_ATTEMPT, GPU_ATTEMPT_TIMEOUT_DEFAULT),
        MIN_USEFUL_GPU_ATTEMPT
    );
    assert_eq!(gpu_attempt_timeout(Duration::ZERO, GPU_ATTEMPT_TIMEOUT_DEFAULT), Duration::ZERO);
}

/// The per-attempt bound must be barely more than one assignment POST.
///
/// In production each attempt was a 30s 503, a 10s wait and a 30s 503 before the
/// 90s cap fired, so 90s bought nothing Colab was not already retrying on its
/// own: it just made every busy accelerator cost a minute and a half. The bound
/// only has to cover the POST and its margin, so a busy best GPU is skipped
/// quickly and retried in the next round.
#[test]
fn test_per_attempt_cap_covers_one_assign_post_plus_margin() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::GPU_ATTEMPT_TIMEOUT_DEFAULT;

    let cap = GPU_ATTEMPT_TIMEOUT_DEFAULT;
    assert!(
        cap >= Duration::from_secs(30),
        "a cap under the observed 30s 503 answer would cut working attempts short, got {cap:?}"
    );
    assert!(
        cap <= Duration::from_secs(45),
        "the cap must not sit inside the CLI's internal retry loop, got {cap:?}"
    );
}

/// RED-able regression: the assignment window is derived from the request
/// deadline the caller really holds, not from
/// `SHIRLEY_COLAB_TIMEOUT_SECS`-derived defaults.
///
/// Production ran with `SHIRLEY_COLAB_TIMEOUT_SECS` unset (default 1800s) while
/// the agent turn cut the request at 600s, so the half of the budget reserved
/// for assignment was computed from 1800s: the window became ~502s instead of
/// ~300s and the upload/exec/download arc was left nothing before the turn was
/// cancelled.
#[test]
fn test_assignment_window_comes_from_the_real_request_deadline() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::{
        gpu_assign_window, pipeline_reserve, request_timeout_for,
    };

    // Production shape: a 1800s configured separation timeout, an agent turn
    // that really ends at 600s.
    let effective = request_timeout_for(
        Duration::from_secs(1800),
        Some(Duration::from_secs(600)),
    );
    assert_eq!(
        effective,
        Duration::from_secs(600),
        "the deadline the request will be cut at must win over the larger default"
    );
    assert_eq!(gpu_assign_window(effective), Duration::from_secs(300));
    assert_eq!(pipeline_reserve(effective), Duration::from_secs(300));
    assert!(
        gpu_assign_window(effective) <= Duration::from_secs(300),
        "assignment may never claim half of a budget the request does not have"
    );

    // A tighter deadline is honoured as-is, and with no deadline the configured
    // timeout is the only bound there is.
    assert_eq!(
        request_timeout_for(
            Duration::from_secs(1800),
            Some(Duration::from_secs(240))
        ),
        Duration::from_secs(240)
    );
    assert_eq!(
        request_timeout_for(Duration::from_secs(900), None),
        Duration::from_secs(900)
    );
}

/// RED-able regression: the primary GPU hangs past the per-attempt bound and a
/// later candidate must carry the separation.
///
/// Under the old code the first candidate was given `request.timeout - elapsed`
/// and the whole run ended in SeparationTimeout without ever asking another GPU.
#[tokio::test]
async fn test_hanging_first_gpu_rotates_to_a_later_candidate() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::MIN_USEFUL_GPU_ATTEMPT;

    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_hang_gpu, b"g4\n").unwrap();
    // The two accelerators between the hanging G4 and L4 refuse outright, so the
    // rotation really walks the ranking and lands on the last candidate.
    std::fs::write(&fixture.trigger_fail_gpu, b"h100\na100\n").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = vec!["a100".to_string(), "l4".to_string()];
    // 12s request: assignment gets 6s, split across four attempts. Two seconds
    // per attempt lets the hanging G4 be cut off inside the assignment budget
    // and still leaves room for the three remaining candidates.
    cfg.gpu_attempt_timeout = MIN_USEFUL_GPU_ATTEMPT;
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("hang_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(12),
    };

    let started = std::time::Instant::now();
    let res = sep.separate(req).await;
    let elapsed = started.elapsed();

    assert!(
        res.is_ok(),
        "a hanging accelerator must not fail the separation, got: {res:?}"
    );
    assert!(
        elapsed < Duration::from_secs(12),
        "the request must finish inside its own timeout, took {elapsed:?}"
    );

    let log = fixture.read_log();
    let hangs: Vec<&str> = log.lines().filter(|l| l.starts_with("hangnew ")).collect();
    assert_eq!(hangs.len(), 1, "the hanging GPU must be asked once, log was: {log}");
    assert!(hangs[0].contains("--gpu G4"), "the fastest must be asked first, log was: {log}");

    let starts: Vec<&str> = log.lines().filter(|l| l.starts_with("new ")).collect();
    assert_eq!(starts.len(), 1, "only the assigning GPU counts as started, log was: {log}");
    assert!(
        starts[0].contains("--gpu L4"),
        "the chain must rotate past a hang to a later candidate, log was: {log}"
    );
    assert_eq!(gpu.available_permits(), 1, "the GPU permit must be released");
}

/// A timed-out attempt is a live session nobody will use, and leaving it active
/// makes the next `new` fail with 412 Precondition Failed. It must be stopped
/// before the chain rotates.
#[tokio::test]
async fn test_timed_out_gpu_attempt_is_stopped_before_rotation() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::MIN_USEFUL_GPU_ATTEMPT;

    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_hang_gpu, b"g4\n").unwrap();
    // The two accelerators between the hanging G4 and L4 refuse outright, so the
    // rotation really reaches L4 and the assertion below names the last attempt.
    std::fs::write(&fixture.trigger_fail_gpu, b"h100\na100\n").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = vec!["l4".to_string()];
    cfg.gpu_attempt_timeout = MIN_USEFUL_GPU_ATTEMPT;
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("stop_rotate_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(10),
    };

    sep.separate(req).await.expect("the fallback GPU must carry the separation");

    let log = fixture.read_log();
    let hang_line = log
        .lines()
        .find(|l| l.starts_with("hangnew "))
        .unwrap_or_else(|| panic!("the hanging GPU must have been asked, log was: {log}"));
    // The mock logs `hangnew new -s shirley-<hex> --gpu G4`.
    let hung_session = hang_line
        .split_whitespace()
        .nth(3)
        .expect("the hung session name must be logged");
    assert!(hung_session.starts_with("shirley-"), "got {hung_session}");

    let stops: Vec<&str> = log.lines().filter(|l| l.starts_with("stop ")).collect();
    assert!(
        stops.iter().any(|l| l.contains(hung_session)),
        "the timed-out session {hung_session} must be stopped before rotating, log was: {log}"
    );

    // The stop happens BEFORE the next accelerator is asked, so the account is
    // not left with an active session that would earn a 412.
    let stop_pos = log
        .lines()
        .position(|l| l.starts_with("stop ") && l.contains(hung_session))
        .expect("stop of the hung session");
    let next_start_pos = log
        .lines()
        .position(|l| l.starts_with("new ") && l.contains("--gpu L4"))
        .expect("the fallback attempt");
    assert!(
        stop_pos < next_start_pos,
        "the hung session must be stopped before the next candidate, log was: {log}"
    );
    assert_eq!(gpu.available_permits(), 1);
}

/// When every accelerator the CLI accepts refuses, the request ends as
/// ColabUnavailable, inside its own timeout, having really tried the chain.
#[tokio::test]
async fn test_all_gpus_failing_reports_colab_unavailable_within_the_request_timeout() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::MIN_USEFUL_GPU_ATTEMPT;

    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_all_gpus_busy, b"busy").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = vec!["a100".to_string(), "l4".to_string()];
    cfg.gpu_attempt_timeout = MIN_USEFUL_GPU_ATTEMPT;
    cfg.gpu_max_rounds = 2;
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("all_busy_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(14),
    };

    let started = std::time::Instant::now();
    let res = sep.separate(req).await;
    let elapsed = started.elapsed();

    assert_eq!(res.unwrap_err(), VocalSeparationError::ColabUnavailable);
    assert!(
        elapsed < Duration::from_secs(14),
        "failure must respect the request budget, took {elapsed:?}"
    );
    assert_eq!(gpu.available_permits(), 1, "the GPU permit must be released");

    // Nothing assigned, so nothing may be stopped: there is no session to free.
    let log = fixture.read_log();
    assert!(
        !log.lines().any(|l| l.starts_with("stop ")),
        "no session was created, so no stop may run, log was: {log}"
    );
}

/// A configured value the CLI does not accept must be skipped, not attempted:
/// `14` is the value production carried, and the CLI answers it with
/// `unsupported value '14'. Supported: T4, L4, G4, H100, A100.` Spending an
/// attempt on it wastes the only thing the rotation is short of.
#[tokio::test]
async fn test_unsupported_configured_gpu_is_never_attempted() {
    let fixture = MockColabEnv::new_fixture();
    // Only L4 answers, so the run has to reach the end of the ranking, and the
    // unsupported configured value must not be one of the attempts on the way.
    std::fs::write(&fixture.trigger_fail_gpu, b"g4\nh100\na100\n").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "14".to_string();
    cfg.gpu_fallbacks = vec!["l4".to_string()];
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("bad_gpu_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert!(
        res.is_ok(),
        "an unusable configured value must not fail the request, got: {res:?}"
    );

    let log = fixture.read_log();
    assert!(
        !log.contains("badgpu"),
        "an unsupported GPU value must never reach the CLI, log was: {log}"
    );
    let starts: Vec<&str> = log.lines().filter(|l| l.starts_with("new ")).collect();
    assert_eq!(starts.len(), 1, "only the usable accelerator may be asked, log was: {log}");
    assert!(starts[0].contains("--gpu L4"), "got: {}", starts[0]);
    assert_eq!(gpu.available_permits(), 1);
}

/// Regression for the shrinking reserve: the assignment window is fixed when
/// assignment starts, so retrying can never eat the half of the budget kept for
/// upload/exec/download.
///
/// Before the fix each attempt recomputed `remaining - remaining/2`, so with a
/// 4 x 3 x 90s cap the rotation kept going until barely 2s of the budget was
/// left, and the pipeline got what remained. Here every candidate hangs for the
/// whole attempt cap, so the run must still end close to the assignment window
/// (half the request) instead of near the full request timeout.
#[tokio::test]
async fn test_gpu_assignment_window_stays_fixed_while_every_candidate_hangs() {
    let fixture = MockColabEnv::new_fixture();
    // Every accelerator the chain can try hangs for the full attempt bound.
    std::fs::write(&fixture.trigger_hang_gpu, b"g4\na100\nl4\nt4\nh100\n").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    cfg.gpu_type = "g4".to_string();
    cfg.gpu_fallbacks = vec!["a100".to_string(), "l4".to_string()];
    // Cap each attempt below the window so several attempts really run; the
    // point is what the rotation does with many attempts, not one long one.
    cfg.gpu_attempt_timeout = Duration::from_secs(2);
    cfg.gpu_max_rounds = 4;
    let sep = ColabVocalSeparator::new(cfg);

    let request_budget = Duration::from_secs(20);
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("all_hang_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: request_budget,
    };

    let started = std::time::Instant::now();
    let res = sep.separate(req).await;
    let elapsed = started.elapsed();

    assert_eq!(res.unwrap_err(), VocalSeparationError::ColabUnavailable);
    assert_eq!(gpu.available_permits(), 1, "the GPU permit must be released");

    // Generous margins for slow CI: the assignment window is 10s of the 20s
    // request, so ending anywhere below 15s proves the reserve survived the
    // retries. The shrinking-reserve bug ran until ~18s or past the deadline.
    assert!(
        elapsed < Duration::from_secs(15),
        "the pipeline reserve must survive every retry: assignment+teardown took \
         {elapsed:?} of a {request_budget:?} request"
    );

    // More than one attempt really ran, so this is the rotation being measured
    // and not a single early failure.
    let log = fixture.read_log();
    let hangs: Vec<&str> = log.lines().filter(|l| l.starts_with("hangnew ")).collect();
    assert!(
        hangs.len() >= 2,
        "the rotation must have tried more than one candidate, log was: {log}"
    );

    // Each abandoned attempt was stopped before the next one was asked, so the
    // account is never left with an active session (the 412 case).
    let stops: Vec<&str> = log.lines().filter(|l| l.starts_with("stop ")).collect();
    assert!(
        stops.len() >= hangs.len(),
        "every timed-out attempt must be stopped, log was: {log}"
    );
}

/// Production failure, reproduced: the official CLI aborts a `colab exec` whose
/// kernel prints nothing for 30s and reports
/// `TimeoutError: Timeout waiting for output: no message from the kernel for
/// 30s. Use --timeout to allow longer silences.` A real separation is silent for
/// longer than that while it installs packages and loads the separation models,
/// so the CLI's own watchdog killed every request at ~36s with an empty stdout.
///
/// The adapter must hand the step's remaining budget to `--timeout` instead of
/// leaving the CLI on its default. Both exec calls are covered: the separation
/// script and the upload size probe.
#[tokio::test]
async fn test_exec_passes_remaining_budget_as_cli_silence_timeout() {
    let fixture = MockColabEnv::new_fixture();
    // From here on the mock behaves like the real CLI: a silent kernel with the
    // default 30s budget is a failure.
    std::fs::write(&fixture.trigger_require_cli_timeout, b"strict").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let cfg = fixture.make_config(gpu.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let output_mp3 = fixture.temp_dir.path().join("silence_out.mp3");
    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: output_mp3.clone(),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    let res = sep.separate(req).await;
    assert!(
        res.is_ok(),
        "a silent kernel must not abort the separation, got: {res:?}"
    );
    assert!(output_mp3.is_file(), "the separated audio must be written");

    let log = fixture.read_log();
    assert!(
        !log.contains("execfail"),
        "no exec may reach the CLI without a silence budget above its default, log: {log}"
    );

    let exec_lines: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with("exec "))
        .collect();
    assert!(
        exec_lines.len() >= 2,
        "the size probe and the separation script are both execs, log: {log}"
    );
    for line in &exec_lines {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let idx = tokens
            .iter()
            .position(|t| *t == "--timeout")
            .unwrap_or_else(|| panic!("exec must pass --timeout, got: {line}"));
        let value: u64 = tokens[idx + 1]
            .parse()
            .unwrap_or_else(|_| panic!("--timeout must be whole seconds, got: {line}"));
        assert!(
            value > 30,
            "the CLI default of 30s aborts a silent kernel, got --timeout {value} in: {line}"
        );
        assert!(
            value <= 60,
            "--timeout must stay inside the request budget (60s), got {value} in: {line}"
        );
        assert!(
            tokens.contains(&"-f"),
            "the script must still be passed with -f, got: {line}"
        );
    }
}

/// The CLI's silence budget is derived from the budget the step really has, in
/// whole seconds rounded up, and never `0` — the CLI reads `--timeout 0` as "no
/// limit", which would remove the bound instead of raising it.
#[test]
fn test_cli_silence_timeout_rounds_up_and_never_disables_the_bound() {
    use shirley_show::infra::vocal::colab_vocal_separator_config::cli_silence_timeout_secs;

    assert_eq!(cli_silence_timeout_secs(Duration::from_secs(600)), 600);
    // Rounded up: 599.2s of budget must not become 599s, which would cut the
    // call before our own deadline does.
    assert_eq!(
        cli_silence_timeout_secs(Duration::from_millis(599_200)),
        600
    );
    assert_eq!(cli_silence_timeout_secs(Duration::from_secs(31)), 31);
    // Never zero: `0` means "no limit" to the CLI.
    assert_eq!(cli_silence_timeout_secs(Duration::from_millis(1)), 1);
    assert_eq!(cli_silence_timeout_secs(Duration::ZERO), 1);
}

/// A failed `colab exec` must always leave one diagnosable line behind.
///
/// The product error is a fixed message that hides stderr on purpose, so the
/// only channel is the process log. Before this, a production failure was
/// completely silent: empty stdout, no trace of the CLI's own
/// `TimeoutError: ... no message from the kernel for 30s`, and nothing to read
/// from `docker logs`.
#[tokio::test]
async fn test_exec_failure_always_emits_a_sanitized_diagnostic() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_exec, b"fail").unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu.clone());
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    cfg.diagnostic_sink = Some(captured.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("diag_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };

    // The user-facing error is unchanged: still SeparationFailed, still no
    // stderr leaked into the product message.
    let res = sep.separate(req).await;
    assert_eq!(res.unwrap_err(), VocalSeparationError::SeparationFailed);

    let lines = captured.lock().unwrap().clone();
    assert_eq!(
        lines.len(),
        1,
        "exactly one diagnostic line per failed exec, got: {lines:?}"
    );
    let line = &lines[0];

    // Exit code, so a timeout (1) can be told from a signal kill.
    assert!(
        line.contains("exit=1"),
        "the diagnostic must carry the exit code, got: {line}"
    );
    // The last stderr line is the one that names the cause.
    assert!(
        line.contains("TimeoutError: Timeout waiting for output"),
        "the diagnostic must carry the CLI's own error, got: {line}"
    );
    // One line only: a traceback would otherwise flood the log.
    assert!(
        !line.contains('\n'),
        "the diagnostic must be a single line, got: {line:?}"
    );
    // No credential may reach the log.
    let token_file = std::fs::read_to_string(&fixture.token_file).unwrap();
    let refresh = token_file
        .split("\"refresh_token\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("fixture token");
    assert!(
        !line.contains(refresh),
        "the diagnostic must never carry the OAuth token, got: {line}"
    );
}

/// The sanitizer keeps one bounded line: the last non-empty stderr line, with
/// whitespace collapsed, truncated to ~300 characters. Checked through the
/// process-level diagnostic rather than by exposing the helper: `colab exec`
/// printing a traceback must leave exactly one line naming the real cause.
#[tokio::test]
async fn test_diagnostic_keeps_only_the_last_stderr_line_bounded() {
    let fixture = MockColabEnv::new_fixture();
    // A long last line, so the truncation is observable in the emitted line.
    let long_tail = format!("RuntimeError: {}", "y".repeat(400));
    let stderr = format!(
        "Traceback (most recent call last):\n  File \"<stdin>\", line 4, in <module>\n{long_tail}\n"
    );
    std::fs::write(&fixture.trigger_fail_exec, b"fail").unwrap();
    std::fs::write(&fixture.trigger_fail_exec_stderr, stderr).unwrap();

    let gpu = Arc::new(Semaphore::new(1));
    let mut cfg = fixture.make_config(gpu);
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    cfg.diagnostic_sink = Some(captured.clone());
    let sep = ColabVocalSeparator::new(cfg);

    let req = VocalSeparationRequest {
        source_path: fixture.sample_wav.clone(),
        output_path: fixture.temp_dir.path().join("bounded_out.mp3"),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(60),
    };
    assert_eq!(
        sep.separate(req).await.unwrap_err(),
        VocalSeparationError::SeparationFailed
    );

    let lines = captured.lock().unwrap().clone();
    assert_eq!(lines.len(), 1, "one line per failed exec, got: {lines:?}");
    let line = &lines[0];
    assert!(
        line.contains("RuntimeError:"),
        "the last stderr line names the cause, got: {line}"
    );
    assert!(
        !line.contains("Traceback (most recent call last)"),
        "the traceback header must not be the reported cause, got: {line}"
    );
    assert!(
        line.chars().count() <= 380,
        "the diagnostic must be truncated to ~300 chars of stderr, got {} chars: {line}",
        line.chars().count()
    );
}
