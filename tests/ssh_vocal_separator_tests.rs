use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use shirley_show::config::{Config, JoshuaPcConfig};
use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::remote_directory_guard::RemoteDirectoryGuard;
use shirley_show::infra::vocal::ssh_command_builder::SshCommandBuilder;
use shirley_show::infra::vocal::ssh_vocal_separator::SshVocalSeparator;
use shirley_show::infra::vocal::ssh_vocal_separator_config::SshVocalSeparatorConfig;
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
fn test_joshua_pc_config_defaults_and_env_parsing() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Clean environment
    remove_env("JOSHUA_PC_HOST");
    remove_env("JOSHUA_PC_USER");
    remove_env("JOSHUA_PC_SSH_KEY");
    remove_env("JOSHUA_PC_PORT");
    remove_env("JOSHUA_PC_WAKE_TIMEOUT_MINS");
    remove_env("SHIRLEY_SEPARATION_TIMEOUT_SECS");
    remove_env("SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS");

    // 1. Unset JOSHUA_PC_HOST -> None
    let cfg = Config::from_env().expect("valid config without joshua pc");
    assert!(cfg.joshua_pc.is_none());
    assert_eq!(cfg.separation_timeout_secs, 600);
    assert_eq!(cfg.remote_separation_timeout_secs, 1500);

    // 2. Empty or whitespace JOSHUA_PC_HOST -> None
    set_env("JOSHUA_PC_HOST", "   ");
    let cfg_empty = Config::from_env().expect("whitespace host should be None");
    assert!(cfg_empty.joshua_pc.is_none());

    // 3. Valid JOSHUA_PC_HOST with defaults
    set_env("JOSHUA_PC_HOST", "100.73.247.39");
    let cfg_remote = Config::from_env().expect("valid joshua pc defaults");
    let jpc = cfg_remote.joshua_pc.expect("joshua_pc must be Some");
    assert_eq!(jpc.host, "100.73.247.39");
    assert_eq!(jpc.user, "loonbac");
    assert_eq!(jpc.key_path, PathBuf::from("/home/shirley/.ssh/id_ed25519"));
    assert_eq!(jpc.port, 22);
    assert_eq!(jpc.wake_timeout, Duration::from_secs(600));

    // 4. Custom JOSHUA_PC_* fields
    set_env("JOSHUA_PC_USER", "custom-user");
    set_env("JOSHUA_PC_SSH_KEY", "/secrets/key.pem");
    set_env("JOSHUA_PC_PORT", "2222");
    set_env("JOSHUA_PC_WAKE_TIMEOUT_MINS", "15");
    set_env("SHIRLEY_SEPARATION_TIMEOUT_SECS", "900");
    set_env("SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS", "2000");

    let cfg_custom = Config::from_env().expect("valid custom joshua pc");
    let jpc_custom = cfg_custom.joshua_pc.expect("custom joshua_pc present");
    assert_eq!(jpc_custom.user, "custom-user");
    assert_eq!(jpc_custom.key_path, PathBuf::from("/secrets/key.pem"));
    assert_eq!(jpc_custom.port, 2222);
    assert_eq!(jpc_custom.wake_timeout, Duration::from_secs(15 * 60));
    assert_eq!(cfg_custom.separation_timeout_secs, 900);
    assert_eq!(cfg_custom.remote_separation_timeout_secs, 2000);

    // Clean up
    remove_env("JOSHUA_PC_HOST");
    remove_env("JOSHUA_PC_USER");
    remove_env("JOSHUA_PC_SSH_KEY");
    remove_env("JOSHUA_PC_PORT");
    remove_env("JOSHUA_PC_WAKE_TIMEOUT_MINS");
    remove_env("SHIRLEY_SEPARATION_TIMEOUT_SECS");
    remove_env("SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS");
}

#[test]
fn test_joshua_pc_config_validation_rejections() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // 1. Host with shell metacharacters or leading hyphen
    set_env("JOSHUA_PC_HOST", "-bad-host");
    assert!(Config::from_env().is_err());

    set_env("JOSHUA_PC_HOST", "host;rm -rf /");
    assert!(Config::from_env().is_err());

    set_env("JOSHUA_PC_HOST", "host with spaces");
    assert!(Config::from_env().is_err());

    // Valid host
    set_env("JOSHUA_PC_HOST", "100.73.247.39");

    // 2. User with invalid characters or leading hyphen
    set_env("JOSHUA_PC_USER", "-user");
    assert!(Config::from_env().is_err());

    set_env("JOSHUA_PC_USER", "user name");
    assert!(Config::from_env().is_err());
    remove_env("JOSHUA_PC_USER");

    // 3. Port invalid
    set_env("JOSHUA_PC_PORT", "0");
    assert!(Config::from_env().is_err());

    set_env("JOSHUA_PC_PORT", "not-a-port");
    assert!(Config::from_env().is_err());
    remove_env("JOSHUA_PC_PORT");

    // 4. Wake timeout 0
    set_env("JOSHUA_PC_WAKE_TIMEOUT_MINS", "0");
    assert!(Config::from_env().is_err());
    remove_env("JOSHUA_PC_WAKE_TIMEOUT_MINS");

    // 5. Remote separation timeout less than wake + separation
    set_env("SHIRLEY_SEPARATION_TIMEOUT_SECS", "600");
    set_env("JOSHUA_PC_WAKE_TIMEOUT_MINS", "10"); // 600s
    // wake (600) + separation (600) = 1200. Setting remote separation to 1000 must fail:
    set_env("SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS", "1000");
    assert!(Config::from_env().is_err());

    // Clean up
    remove_env("JOSHUA_PC_HOST");
    remove_env("JOSHUA_PC_USER");
    remove_env("JOSHUA_PC_SSH_KEY");
    remove_env("JOSHUA_PC_PORT");
    remove_env("JOSHUA_PC_WAKE_TIMEOUT_MINS");
    remove_env("SHIRLEY_SEPARATION_TIMEOUT_SECS");
    remove_env("SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS");
}

#[test]
fn test_vocal_separation_error_remote_variants() {
    assert_eq!(
        VocalSeparationError::PcWakeTimeout.code(),
        "pc_wake_timeout"
    );
    assert_eq!(
        VocalSeparationError::RemoteSudoDenied.code(),
        "remote_sudo_denied"
    );
    assert_eq!(
        VocalSeparationError::SshKeyMissing.code(),
        "ssh_key_missing"
    );
    assert_eq!(
        VocalSeparationError::InvalidSeparatorOutput.code(),
        "invalid_separator_output"
    );

    // Verify messages are non-empty and user-friendly
    assert!(!VocalSeparationError::PcWakeTimeout.message().is_empty());
    assert!(!VocalSeparationError::RemoteSudoDenied.message().is_empty());
    assert!(!VocalSeparationError::SshKeyMissing.message().is_empty());
}

#[test]
fn test_ssh_command_builder_argv_construction() {
    let builder = SshCommandBuilder::new(
        PathBuf::from("/usr/bin/ssh"),
        PathBuf::from("/usr/bin/scp"),
        "loonbac".to_string(),
        "100.73.247.39".to_string(),
        PathBuf::from("/home/shirley/.ssh/id_ed25519"),
        22,
    );

    // 1. Probe
    let probe_cmd = builder.build_probe(3);
    let probe_args: Vec<_> = probe_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        probe_args,
        vec![
            "-p",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=3",
            "loonbac@100.73.247.39",
            "true",
        ]
    );

    // 2. Mkdir
    let mkdir_cmd = builder.build_mkdir("/tmp/shirley-vocal/test-uuid");
    let mkdir_args: Vec<_> = mkdir_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        mkdir_args,
        vec![
            "-p",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "loonbac@100.73.247.39",
            "mkdir",
            "-p",
            "-m",
            "700",
            "/tmp/shirley-vocal/test-uuid",
        ]
    );

    // 3. Upload (SCP: note capital -P for port)
    let upload_cmd = builder.build_upload(
        &PathBuf::from("/tmp/local.mp3"),
        "/tmp/shirley-vocal/test-uuid/input.mp3",
    );
    let upload_args: Vec<_> = upload_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        upload_args,
        vec![
            "-P",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "/tmp/local.mp3",
            "loonbac@100.73.247.39:/tmp/shirley-vocal/test-uuid/input.mp3",
        ]
    );

    // 4. Pipeline
    let (pipeline_cmd, script) =
        builder.build_pipeline("chorus", "/tmp/shirley-vocal/test-uuid", "input.mp3");
    let pipeline_args: Vec<_> = pipeline_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        pipeline_args,
        vec![
            "-p",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "loonbac@100.73.247.39",
            "sh",
            "-s",
            "--",
            "chorus",
            "/tmp/shirley-vocal/test-uuid",
            "input.mp3",
        ]
    );
    assert!(!script.is_empty());
    assert!(script.contains("karaoke-separator"));

    // 5. Download (SCP: note capital -P)
    let download_cmd = builder.build_download(
        "/tmp/shirley-vocal/test-uuid/output.mp3",
        &PathBuf::from("/tmp/local_out.mp3"),
    );
    let download_args: Vec<_> = download_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        download_args,
        vec![
            "-P",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "loonbac@100.73.247.39:/tmp/shirley-vocal/test-uuid/output.mp3",
            "/tmp/local_out.mp3",
        ]
    );

    // 6. Cleanup
    let cleanup_cmd = builder.build_cleanup("/tmp/shirley-vocal/test-uuid");
    let cleanup_args: Vec<_> = cleanup_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        cleanup_args,
        vec![
            "-p",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "loonbac@100.73.247.39",
            "rm",
            "-rf",
            "/tmp/shirley-vocal/test-uuid",
        ]
    );

    // 7. Cancel shutdown
    let cancel_cmd = builder.build_cancel_shutdown();
    let cancel_args: Vec<_> = cancel_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        cancel_args,
        vec![
            "-p",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "loonbac@100.73.247.39",
            "sudo",
            "-n",
            "/run/current-system/sw/bin/shutdown",
            "-c",
        ]
    );

    // 8. Schedule idle shutdown (+60)
    let schedule_cmd = builder.build_schedule_shutdown();
    let schedule_args: Vec<_> = schedule_cmd
        .as_std()
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        schedule_args,
        vec![
            "-p",
            "22",
            "-i",
            "/home/shirley/.ssh/id_ed25519",
            "-o",
            "BatchMode=yes",
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "loonbac@100.73.247.39",
            "sudo",
            "-n",
            "/run/current-system/sw/bin/shutdown",
            "+60",
            "Shirley Show idle shutdown",
        ]
    );
}

#[tokio::test]
async fn test_remote_directory_guard_lifecycle_and_cleanup_on_drop() {
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let permit = sem.clone().acquire_owned().await.unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let local_partial = temp_dir.path().join("partial_output.mp3");
    tokio::fs::write(&local_partial, b"partial-data")
        .await
        .unwrap();
    assert!(local_partial.exists());

    let builder = SshCommandBuilder::new(
        PathBuf::from("/bin/echo"),
        PathBuf::from("/bin/echo"),
        "loonbac".to_string(),
        "100.73.247.39".to_string(),
        PathBuf::from("/nonexistent/key"),
        22,
    );

    // Create guard with armed remote dir and local partial
    {
        let mut guard = RemoteDirectoryGuard::new(Some(local_partial.clone()), permit, builder);
        guard.arm_remote_dir("/tmp/shirley-vocal/op-123");
        assert_eq!(guard.remote_dir(), Some("/tmp/shirley-vocal/op-123"));

        // While guard is alive, semaphore has 0 available permits
        assert_eq!(sem.available_permits(), 0);

        // Dropping guard without disarming triggers cleanup of local partial and releases permit
    }

    // After drop, local partial should be deleted
    // Give async drop task a moment if executed via handle.spawn
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !local_partial.exists(),
        "local partial must be deleted on undisarmed drop"
    );

    // Permit must be released back to semaphore
    assert_eq!(
        sem.available_permits(),
        1,
        "permit must be released back to semaphore"
    );
}

#[test]
fn test_ssh_vocal_separator_config_injectable_bins() {
    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: PathBuf::from("/path/key"),
        port: 22,
        wake_timeout: Duration::from_secs(600),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let cfg = SshVocalSeparatorConfig::with_custom_bins(
        jpc.clone(),
        PathBuf::from("/custom/ssh"),
        PathBuf::from("/custom/scp"),
        PathBuf::from("/custom/ffprobe"),
        sem,
    );
    assert_eq!(cfg.ssh_bin, PathBuf::from("/custom/ssh"));
    assert_eq!(cfg.scp_bin, PathBuf::from("/custom/scp"));
    assert_eq!(cfg.ffprobe_bin, PathBuf::from("/custom/ffprobe"));
    assert_eq!(cfg.joshua_pc.host, "100.73.247.39");
}

fn create_executable_script(path: &std::path::Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, content).unwrap();
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

#[tokio::test]
async fn test_ssh_vocal_separator_instant_connect_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");

    // Stub SSH: exit 0 on all commands
    create_executable_script(
        &ssh_bin,
        r#"#!/bin/sh
exit 0
"#,
    );

    // Stub SCP: on download (destination is local path), write valid MP3 fixture
    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf '%2048s' 'x' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );

    // Stub ffprobe: outputs valid JSON with 120s duration and 320k bitrate
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    // Create a mock key file
    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    // Create a mock source audio file
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();

    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(600),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(
        result.is_ok(),
        "instant connect separation should succeed: {:?}",
        result.err()
    );
    let audio = result.unwrap();
    assert_eq!(audio.duration_secs, 120);
    assert!(audio.size_bytes >= 1024);
    assert!(output_file.exists(), "output file must exist after success");
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_app_state_selects_vocal_separator_based_on_joshua_pc_host() {
    let _lock = ENV_MUTEX.lock().unwrap();

    let pool = shirley_show::infra::sqlite::init_pool(":memory:").unwrap();
    shirley_show::infra::sqlite::migrate_pool(&pool).unwrap();

    let req = VocalSeparationRequest {
        source_path: PathBuf::from("/nonexistent/file.mp3"),
        output_path: PathBuf::from("/nonexistent/out.mp3"),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(5),
    };

    // 1. When JOSHUA_PC_HOST is unset -> ProcessVocalSeparator is selected
    remove_env("JOSHUA_PC_HOST");
    let state_local = shirley_show::state::AppState::new(pool.clone());
    let err_local = state_local
        .vocal_separator
        .separate(req.clone())
        .await
        .unwrap_err();
    assert_eq!(err_local.code(), "invalid_audio_duration");

    // 2. When JOSHUA_PC_HOST is set -> SshVocalSeparator is selected
    set_env("JOSHUA_PC_HOST", "100.73.247.39");
    set_env("JOSHUA_PC_SSH_KEY", "/nonexistent/id_ed25519");
    let state_remote = shirley_show::state::AppState::new(pool);
    let err_remote = state_remote
        .vocal_separator
        .separate(req)
        .await
        .unwrap_err();
    assert_eq!(err_remote.code(), "ssh_key_missing");

    remove_env("JOSHUA_PC_HOST");
    remove_env("JOSHUA_PC_SSH_KEY");
}

#[tokio::test]
async fn test_ssh_vocal_separator_wake_wait_offline_then_online() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");
    let counter_file = temp_dir.path().join("probe_count.txt");

    // Stub SSH: fails first 2 probes, succeeds on 3rd
    let ssh_script = format!(
        r#"#!/bin/sh
counter="{}"
cmd="$@"
case "$cmd" in
    *true)
        val=0
        if [ -f "$counter" ]; then
            val=$(cat "$counter")
        fi
        val=$((val + 1))
        echo "$val" > "$counter"
        if [ "$val" -lt 3 ]; then
            exit 1
        else
            exit 0
        fi
        ;;
    *)
        exit 0
        ;;
esac
"#,
        counter_file.display()
    );
    create_executable_script(&ssh_bin, &ssh_script);

    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf '%2048s' 'x' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );

    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(30),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(
        result.is_ok(),
        "wake wait should succeed when online on probe 3: {:?}",
        result.err()
    );
    let audio = result.unwrap();
    assert_eq!(audio.duration_secs, 120);

    let count_str = std::fs::read_to_string(&counter_file).unwrap();
    assert_eq!(count_str.trim(), "3", "must have probed exactly 3 times");
}

#[tokio::test]
async fn test_ssh_vocal_separator_wake_timeout() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");

    // Stub SSH: always fails probe
    create_executable_script(
        &ssh_bin,
        r#"#!/bin/sh
exit 1
"#,
    );
    create_executable_script(&scp_bin, "#!/bin/sh\nexit 0\n");
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0"
    }
  ],
  "format": {
    "duration": "120.0"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_millis(50),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(10),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "pc_wake_timeout");
}

#[tokio::test]
async fn test_ssh_vocal_separator_missing_ssh_key_fails_fast() {
    let temp_dir = tempfile::tempdir().unwrap();
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: PathBuf::from("/nonexistent/ssh/id_ed25519"),
        port: 22,
        wake_timeout: Duration::from_secs(600),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::new(jpc, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(10),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "ssh_key_missing");
}

#[tokio::test]
async fn test_ssh_vocal_separator_corrupted_output_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");

    create_executable_script(&ssh_bin, "#!/bin/sh\nexit 0\n");

    // Stub SCP: writes only 50 bytes (under the 1024-byte minimum)
    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf 'short-corrupted' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );

    // Stub ffprobe: valid preflight, but post-verify should fail on size
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(10),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "invalid_separator_output");
    assert!(
        !output_file.exists(),
        "corrupted output file must be deleted"
    );
}

#[tokio::test]
async fn test_ssh_vocal_separator_remote_pipeline_failure_cleans_up() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");
    let log_file = temp_dir.path().join("ssh_log.txt");

    // Stub SSH: records all calls; fails if command is "sh -s --"
    let ssh_script = format!(
        r#"#!/bin/sh
log="{}"
echo "$@" >> "$log"
cmd="$@"
case "$cmd" in
    *sh\ -s\ --*)
        exit 1
        ;;
    *)
        exit 0
        ;;
esac
"#,
        log_file.display()
    );
    create_executable_script(&ssh_bin, &ssh_script);
    create_executable_script(&scp_bin, "#!/bin/sh\nexit 0\n");
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0"
    }
  ],
  "format": {
    "duration": "120.0"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(10),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "separation_failed");

    let log_content = std::fs::read_to_string(&log_file).unwrap();
    assert!(
        log_content.contains("rm -rf /tmp/shirley-vocal/"),
        "must trigger remote directory cleanup via rm -rf on failure: {log_content}"
    );
}

#[tokio::test]
async fn test_ssh_vocal_separator_source_duration_exceeded() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");

    create_executable_script(&ssh_bin, "#!/bin/sh\nexit 0\n");
    create_executable_script(&scp_bin, "#!/bin/sh\nexit 0\n");
    // ffprobe returns 605s (exceeds 600s limit)
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "605.0"
    }
  ],
  "format": {
    "duration": "605.0"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(10),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "duration_exceeded");
}

#[tokio::test]
async fn test_ssh_vocal_separator_missing_source_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::new(jpc, sem);

    let separator = SshVocalSeparator::new(config);
    let request = VocalSeparationRequest {
        source_path: PathBuf::from("/nonexistent/source.mp3"),
        output_path: temp_dir.path().join("output.mp3"),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(10),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code(), "invalid_audio_duration");
}

#[tokio::test]
async fn test_ssh_vocal_separator_remote_energy_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");
    let log_file = temp_dir.path().join("ssh_cmds.log");

    let ssh_script = format!(
        r#"#!/bin/sh
cmd="$@"
echo "$cmd" >> "{}"
case "$cmd" in
    *true)
        exit 0
        ;;
    *shutdown*-c*)
        exit 0
        ;;
    *shutdown*+60*)
        exit 0
        ;;
    *)
        exit 0
        ;;
esac
"#,
        log_file.display()
    );
    create_executable_script(&ssh_bin, &ssh_script);

    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf '%2048s' 'x' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );

    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);
    let separator = SshVocalSeparator::new(config);

    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(
        result.is_ok(),
        "separation with energy management should succeed: {:?}",
        result.err()
    );

    let log = std::fs::read_to_string(&log_file).unwrap();
    let lines: Vec<&str> = log.lines().collect();

    // Verify probe happened
    assert!(lines.iter().any(|l| l.contains("true")));

    // Find index of shutdown -c, mkdir, and shutdown +60
    let cancel_idx = lines
        .iter()
        .position(|l| l.contains("shutdown") && l.contains("-c"));
    let mkdir_idx = lines.iter().position(|l| l.contains("mkdir"));
    let schedule_idx = lines
        .iter()
        .position(|l| l.contains("shutdown") && l.contains("+60"));

    assert!(cancel_idx.is_some(), "shutdown -c must be executed");
    assert!(mkdir_idx.is_some(), "mkdir must be executed");
    assert!(schedule_idx.is_some(), "shutdown +60 must be executed");

    assert!(
        cancel_idx.unwrap() < mkdir_idx.unwrap(),
        "shutdown -c must run BEFORE remote mkdir and upload"
    );
    assert!(
        mkdir_idx.unwrap() < schedule_idx.unwrap(),
        "shutdown +60 must run AFTER successful work"
    );
}

#[tokio::test]
async fn test_ssh_vocal_separator_shutdown_c_sudo_denied_does_not_block_processing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");
    let log_file = temp_dir.path().join("ssh_cmds.log");

    let ssh_script = format!(
        r#"#!/bin/sh
cmd="$@"
echo "$cmd" >> "{}"
case "$cmd" in
    *true)
        exit 0
        ;;
    *shutdown*-c*)
        echo "sudo: a password is required" >&2
        exit 1
        ;;
    *)
        exit 0
        ;;
esac
"#,
        log_file.display()
    );
    create_executable_script(&ssh_bin, &ssh_script);
    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf '%2048s' 'x' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);
    let separator = SshVocalSeparator::new(config);

    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(
        result.is_ok(),
        "optional shutdown permissions must not block audio processing: {:?}",
        result.err()
    );

    let log = std::fs::read_to_string(&log_file).unwrap();
    assert!(
        log.contains("mkdir"),
        "the unprivileged pipeline must continue after sudo denial"
    );
    assert!(
        log.contains("+60"),
        "the best-effort shutdown scheduling must still be attempted"
    );
}

#[tokio::test]
async fn test_ssh_vocal_separator_shutdown_c_no_pending_timer_ignored() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");

    create_executable_script(
        &ssh_bin,
        r#"#!/bin/sh
cmd="$@"
case "$cmd" in
    *true)
        exit 0
        ;;
    *shutdown*-c*)
        echo "Cannot find any scheduled shutdown" >&2
        exit 1
        ;;
    *shutdown*+60*)
        exit 0
        ;;
    *)
        exit 0
        ;;
esac
"#,
    );

    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf '%2048s' 'x' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );

    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);
    let separator = SshVocalSeparator::new(config);

    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(
        result.is_ok(),
        "exit code 1 from shutdown -c must be ignored when no shutdown was pending"
    );
}

#[tokio::test]
async fn test_ssh_vocal_separator_pipeline_failure_skips_shutdown_schedule() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");
    let log_file = temp_dir.path().join("ssh_cmds.log");

    let ssh_script = format!(
        r#"#!/bin/sh
cmd="$@"
echo "$cmd" >> "{}"
case "$cmd" in
    *true)
        exit 0
        ;;
    *shutdown*-c*)
        exit 0
        ;;
    *sh*-s*--*)
        exit 1
        ;;
    *)
        exit 0
        ;;
esac
"#,
        log_file.display()
    );
    create_executable_script(&ssh_bin, &ssh_script);
    create_executable_script(&scp_bin, "#!/bin/sh\nexit 0\n");
    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0"
    }
  ],
  "format": {
    "duration": "120.0"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);
    let separator = SshVocalSeparator::new(config);

    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "separation_failed");

    let log = std::fs::read_to_string(&log_file).unwrap();
    assert!(
        !log.contains("+60"),
        "shutdown +60 must NOT be called after pipeline failure"
    );
}

#[tokio::test]
async fn test_ssh_vocal_separator_shutdown_schedule_failure_does_not_fail_audio() {
    let temp_dir = tempfile::tempdir().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    let ssh_bin = bin_dir.join("ssh");
    let scp_bin = bin_dir.join("scp");
    let ffprobe_bin = bin_dir.join("ffprobe");

    create_executable_script(
        &ssh_bin,
        r#"#!/bin/sh
cmd="$@"
case "$cmd" in
    *true)
        exit 0
        ;;
    *shutdown*-c*)
        exit 0
        ;;
    *shutdown*+60*)
        echo "Failed to schedule shutdown" >&2
        exit 1
        ;;
    *)
        exit 0
        ;;
esac
"#,
    );

    create_executable_script(
        &scp_bin,
        r#"#!/bin/sh
dest=""
for arg in "$@"; do
    dest="$arg"
done
case "$dest" in
    *:*) ;;
    *)
        if [ -n "$dest" ] && [ ! -d "$dest" ]; then
            printf '%2048s' 'x' > "$dest"
        fi
        ;;
esac
exit 0
"#,
    );

    create_executable_script(
        &ffprobe_bin,
        r#"#!/bin/sh
cat <<'EOF'
{
  "streams": [
    {
      "codec_type": "audio",
      "codec_name": "mp3",
      "duration": "120.0",
      "bit_rate": "320000"
    }
  ],
  "format": {
    "duration": "120.0",
    "bit_rate": "320000"
  }
}
EOF
exit 0
"#,
    );

    let key_file = temp_dir.path().join("id_ed25519");
    std::fs::write(&key_file, b"mock-key").unwrap();
    let source_file = temp_dir.path().join("source.mp3");
    std::fs::write(&source_file, b"mock-source-audio").unwrap();
    let output_file = temp_dir.path().join("output.mp3");

    let jpc = JoshuaPcConfig {
        host: "100.73.247.39".to_string(),
        user: "loonbac".to_string(),
        key_path: key_file,
        port: 22,
        wake_timeout: Duration::from_secs(10),
    };
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
    let config = SshVocalSeparatorConfig::with_custom_bins(jpc, ssh_bin, scp_bin, ffprobe_bin, sem);
    let separator = SshVocalSeparator::new(config);

    let request = VocalSeparationRequest {
        source_path: source_file,
        output_path: output_file,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(request).await;
    assert!(
        result.is_ok(),
        "failure to schedule shutdown +60 must not invalidate valid audio: {:?}",
        result.err()
    );
}
