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
    trigger_fail_exec: PathBuf,
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
        let trigger_fail_exec = temp_dir.path().join("fail_exec");
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
        let fail_exec_str = trigger_fail_exec.display().to_string();
        let bad_sha_str = trigger_bad_sha.display().to_string();
        let bad_dur_str = trigger_bad_duration.display().to_string();
        let stop_not_found_str = trigger_stop_not_found.display().to_string();
        let temp_dir_str = temp_dir.path().display().to_string();

        let script = format!(
            r#"#!/bin/sh
cmd="$1"
if [ "$cmd" = "new" ]; then
    if [ -f "{fail_start_str}" ]; then
        exit 1
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
    if [ -f "{fail_exec_str}" ]; then
        exit 1
    fi
    echo "exec $@" >> "{log_str}"

    # Extract the script file (last argument)
    for last; do true; done
    script_file="$last"

    # The upload is judged by what the runtime holds, so the fake answers the
    # size probe when it sees one. The probe is itself a script executed with
    # `-f`, so it is detected by content, not by flag shape.
    if grep -q "SIZE=" "$script_file" 2>/dev/null; then
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
            trigger_fail_exec,
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
/// outright. The fallback is tried, and the run must land on it.
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
        starts[0].contains("--gpu L4"),
        "the run must land on the fallback GPU, log was: {log}"
    );
    assert_eq!(gpu.available_permits(), 1, "the GPU permit must be released");
}

/// Negative control for the test above: with the fallback disabled the same
/// broken primary must still surface as ColabUnavailable. Without this, the
/// fallback test could pass for the wrong reason.
#[tokio::test]
async fn test_start_without_fallback_still_reports_colab_unavailable() {
    let fixture = MockColabEnv::new_fixture();
    std::fs::write(&fixture.trigger_fail_gpu, b"g4").unwrap();

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
    std::fs::write(&fixture.trigger_fail_gpu, b"g4\na100\n").unwrap();

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
        assert_eq!(el[1..], ["exec", "-s", session_name, "-f", el[5]]);
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
