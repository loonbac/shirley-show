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
fn test_colab_config_model_ensembles() {
    assert_eq!(INSTRUMENTAL_MODELS.len(), 4);
    assert_eq!(
        INSTRUMENTAL_MODELS[0],
        "model_bs_roformer_ep_317_sdr_12.9755.ckpt"
    );
    assert_eq!(
        INSTRUMENTAL_MODELS[1],
        "model_bs_roformer_ep_368_sdr_12.9628.ckpt"
    );
    assert_eq!(INSTRUMENTAL_MODELS[2], "melband_roformer_inst_v1e.ckpt");
    assert_eq!(
        INSTRUMENTAL_MODELS[3],
        "mel_band_roformer_instrumental_becruily.ckpt"
    );

    assert_eq!(COROS_MODELS.len(), 4);
    assert_eq!(COROS_MODELS[0], "bs_roformer_karaoke_frazer_becruily.ckpt");
    assert_eq!(COROS_MODELS[1], "bs_roformer_karaoke_anvuew.ckpt");
    assert_eq!(COROS_MODELS[2], "mel_band_roformer_karaoke_gabox_v2.ckpt");
    assert_eq!(
        COROS_MODELS[3],
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
        "super_secret_pw",
        "/content/my_input.wav",
    );

    // Check models
    for m in INSTRUMENTAL_MODELS {
        assert!(script.contains(m), "Script missing model {m}");
    }
    // Check overlap
    assert!(script.contains("OVERLAP = 12") || script.contains("--mdxc_overlap"));
    // Check password
    assert!(script.contains("super_secret_pw"));
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
    // Check openssl encryption and curl upload to litterbox
    assert!(script.contains("openssl"));
    assert!(script.contains("-aes-256-cbc"));
    assert!(script.contains("-pbkdf2"));
    assert!(script.contains("litterbox.catbox.moe"));
    // Check single JSON output line
    assert!(script.contains("json.dumps"));
}

#[test]
fn test_remote_script_generation_coros_mode() {
    let script = generate_remote_script(
        VocalSeparationMode::Coros,
        &COROS_MODELS,
        8,
        "another_pass",
        "/content/audio.mp3",
    );

    for m in COROS_MODELS {
        assert!(script.contains(m), "Script missing coros model {m}");
    }
    assert!(script.contains("another_pass"));
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
    trigger_fail_exec: PathBuf,
    trigger_bad_sha: PathBuf,
    trigger_bad_duration: PathBuf,
    sample_wav: PathBuf,
    token_file: PathBuf,
}

impl MockColabFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let colab_bin = temp_dir.path().join("mock_colab.sh");
        let log_file = temp_dir.path().join("colab_calls.log");
        let trigger_fail_start = temp_dir.path().join("fail_start");
        let trigger_fail_exec = temp_dir.path().join("fail_exec");
        let trigger_bad_sha = temp_dir.path().join("bad_sha");
        let trigger_bad_duration = temp_dir.path().join("bad_duration");

        let sample_wav = temp_dir.path().join("sample_input.wav");
        write_silence_wav(&sample_wav, 1);

        let token_file = temp_dir.path().join("token.json");
        std::fs::write(&token_file, r#"{"refresh_token":"valid_token_123"}"#).unwrap();

        let log_str = log_file.display().to_string();
        let fail_start_str = trigger_fail_start.display().to_string();
        let fail_exec_str = trigger_fail_exec.display().to_string();
        let bad_sha_str = trigger_bad_sha.display().to_string();
        let bad_dur_str = trigger_bad_duration.display().to_string();
        let temp_dir_str = temp_dir.path().display().to_string();

        let script = format!(
            r#"#!/bin/sh
cmd="$1"
if [ "$cmd" = "start" ]; then
    if [ -f "{fail_start_str}" ]; then
        exit 1
    fi
    echo "start $@" >> "{log_str}"
    echo '{{"session":"sess-mock-42","proxyUrl":"http://127.0.0.1:8080"}}'
    exit 0
elif [ "$cmd" = "upload" ]; then
    echo "upload $@" >> "{log_str}"
    # Remember what actually arrived so the size probe can answer truthfully.
    # The upload is judged on the runtime contents, never on the exit code.
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

    # The upload is judged by what the runtime holds, not by the CLI exit
    # code, so the fake must answer the size probe. Without this the probe is
    # mistaken for the separation script and every lifecycle test fails.
    case "$*" in
        *"SIZE="*)
            if [ -f "{temp_dir_str}/uploaded" ]; then
                echo "SIZE=$(wc -c < "{temp_dir_str}/uploaded" | tr -d ' ')"
            else
                echo "SIZE=absent"
            fi
            exit 0
            ;;
    esac

    # Extract script file (last argument)
    for last; do true; done
    script_file="$last"

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
elif [ "$cmd" = "stop" ]; then
    echo "stop $@" >> "{log_str}"
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
            trigger_fail_exec,
            trigger_bad_sha,
            trigger_bad_duration,
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

    // Lifecycle on success is now start -> upload -> exec, and the stop is
    // deferred: the machine is kept for the retention window so a second
    // request reuses it. Asserting "stop happened" here would pin the old
    // contract, in which every request paid a cold start.
    let log = fixture.read_log();
    assert!(log.contains("start"), "Must call colab start");
    assert!(log.contains("upload"), "Must call colab upload");
    assert!(log.contains("exec"), "Must call colab exec");
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
    assert!(log.contains("start"));
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

    // Verify colab start was NEVER called
    let log = fixture.read_log();
    assert!(
        !log.contains("start"),
        "colab start must not be called when duration > 600s"
    );
    assert_eq!(gpu.available_permits(), 1);
}

#[test]
fn test_remote_script_treats_no_vocals_as_instrumental() {
    let script = generate_remote_script(
        VocalSeparationMode::Instrumental,
        &INSTRUMENTAL_MODELS,
        12,
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
async fn test_start_with_unparseable_output_stops_every_runtime() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("calls.log");
    let colab_bin = temp_dir.path().join("mock_colab.sh");
    let sample_wav = temp_dir.path().join("in.wav");
    write_silence_wav(&sample_wav, 1);
    let token_file = temp_dir.path().join("token.json");
    std::fs::write(&token_file, r#"{"refresh_token":"t"}"#).unwrap();

    // start reports success but prints something that is not JSON: the GPU is
    // already allocated, so the adapter must issue a session-less stop.
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{}"
if [ "$1" = "start" ]; then echo "not-json-at-all"; exit 0; fi
exit 0
"#,
        log_file.display()
    );
    std::fs::write(&colab_bin, script).unwrap();
    let mut perms = std::fs::metadata(&colab_bin).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&colab_bin, perms).unwrap();

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
    let stop_line = log
        .lines()
        .find(|l| l.starts_with("stop"))
        .expect("a stop must be issued after an unparseable start");
    assert!(
        !stop_line.contains("--session"),
        "the stop must be session-less when no id could be parsed, got: {stop_line}"
    );
}
