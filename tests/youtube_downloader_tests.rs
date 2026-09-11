use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

use shirley_show::domain::youtube_url::YouTubeUrl;
use shirley_show::infra::youtube::{ProcessYoutubeDownloader, ProcessYoutubeDownloaderConfig};
use shirley_show::ports::youtube_downloader::{
    YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};

fn create_executable_script(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = File::create(&path).expect("failed to create script");
    file.write_all(content.as_bytes()).expect("failed to write script");
    let mut perms = file.metadata().expect("failed to get metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).expect("failed to set script executable");
    path
}

struct TestEnvironment {
    pub _temp_dir: TempDir,
    pub bin_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub log_file: PathBuf,
    pub yt_dlp_path: PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create tempdir");
        let bin_dir = temp_dir.path().join("bin");
        let staging_dir = temp_dir.path().join("staging");
        fs::create_dir_all(&bin_dir).expect("failed to create bin dir");
        fs::create_dir_all(&staging_dir).expect("failed to create staging dir");
        let log_file = temp_dir.path().join("args.log");

        // Stub for ffmpeg
        create_executable_script(
            &bin_dir,
            "ffmpeg",
            "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then echo \"ffmpeg version 7.0\"; exit 0; fi\nexit 0\n",
        );

        // Stub for ffprobe
        create_executable_script(
            &bin_dir,
            "ffprobe",
            "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then echo \"ffprobe version 7.0\"; exit 0; fi\necho '{\"format\":{\"duration\":\"212.0\"}}'\nexit 0\n",
        );

        // Stub for yt-dlp default behavior
        let yt_dlp_script = format!(
            r#"#!/bin/sh
# Log all args
printf '%s\n' "$@" > "{log_path}"

# Find output argument
output_arg=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
    output_arg="$2"
    shift 2
  else
    shift 1
  fi
done

# Create mp3 file in current working dir
stage_id=$(echo "$output_arg" | sed 's/\.%(ext)s//')
mp3_file="${{stage_id}}.mp3"
printf 'ID3\03\00\00\00\00\00\00\00\00' > "$mp3_file"
# Append padding to reach ~1000 bytes
head -c 1000 /dev/zero >> "$mp3_file"

# Output JSON line
echo '{{"title": "Never Gonna Give You Up", "duration": 212.0, "is_live": false, "live_status": "not_live"}}'
exit 0
"#,
            log_path = log_file.display()
        );

        let yt_dlp_path = create_executable_script(&bin_dir, "yt-dlp", &yt_dlp_script);

        Self {
            _temp_dir: temp_dir,
            bin_dir,
            staging_dir,
            log_file,
            yt_dlp_path,
        }
    }

    fn config(&self) -> ProcessYoutubeDownloaderConfig {
        ProcessYoutubeDownloaderConfig {
            yt_dlp_bin: Some(self.yt_dlp_path.clone()),
            timeout: Duration::from_secs(90),
        }
    }
}

#[tokio::test]
async fn test_process_downloader_exact_structured_arguments_and_320k_mp3() {
    let env = TestEnvironment::new();
    let downloader = ProcessYoutubeDownloader::new(env.config());

    let stage_id = "test-stage-uuid-001";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLignore")
        .expect("valid url");

    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let result = downloader
        .download(url, target)
        .await
        .expect("download should succeed");

    assert_eq!(result.title, "Never Gonna Give You Up");
    assert_eq!(result.duration_secs, 212);
    assert!(result.size_bytes > 0);

    // Verify .part exists and .mp3 does not exist (renamed)
    let part_path = env.staging_dir.join(format!("{stage_id}.part"));
    let mp3_path = env.staging_dir.join(format!("{stage_id}.mp3"));
    assert!(part_path.exists(), "staged .part file must exist");
    assert!(!mp3_path.exists(), ".mp3 file should have been renamed to .part");

    // Verify exact arguments passed to yt-dlp
    let logged_args = fs::read_to_string(&env.log_file).expect("should read logged args");
    let args: Vec<&str> = logged_args.lines().collect();

    let expected_args = [
        "--no-config",
        "--no-playlist",
        "--extract-audio",
        "--audio-format",
        "mp3",
        "--audio-quality",
        "320K",
        "--max-filesize",
        "100M",
        "--match-filter",
        "!is_live & duration <= 1200",
        "--no-simulate",
        "--no-progress",
        "--print-json",
        "--output",
        &format!("{stage_id}.%(ext)s"),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    ];

    assert_eq!(
        args, expected_args,
        "yt-dlp arguments must exactly match structured specifications in order"
    );
}

#[tokio::test]
async fn test_rejects_duration_exceeding_1200_seconds() {
    let env = TestEnvironment::new();

    // Replace ffprobe with one reporting 1205.0 seconds
    create_executable_script(
        &env.bin_dir,
        "ffprobe",
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then echo \"ffprobe 7.0\"; exit 0; fi\necho '{\"format\":{\"duration\":\"1205.0\"}}'\nexit 0\n",
    );

    let downloader = ProcessYoutubeDownloader::new(env.config());
    let stage_id = "test-stage-duration-fail";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let err = downloader.download(url, target).await.unwrap_err();
    assert_eq!(err, YoutubeDownloadError::DurationExceeded);
    assert_eq!(err.code(), "duration_exceeded");

    // Ensure partial files are purged
    let part_path = env.staging_dir.join(format!("{stage_id}.part"));
    let mp3_path = env.staging_dir.join(format!("{stage_id}.mp3"));
    assert!(!part_path.exists(), "part file must be purged");
    assert!(!mp3_path.exists(), "mp3 file must be purged");
}

#[tokio::test]
async fn test_rejects_file_size_exceeding_100_mib() {
    let env = TestEnvironment::new();

    let yt_dlp_script = r#"#!/bin/sh
stage_id=$(echo "$@" | grep -o '[^ ]*\.%(ext)s' | sed 's/\.%(ext)s//')
mp3_file="${stage_id}.mp3"
truncate -s 105000000 "$mp3_file"
printf 'ID3\03\00\00\00\00\00\00\00\00' | dd of="$mp3_file" conv=notrunc 2>/dev/null
echo '{"title": "Heavy Audio", "duration": 300.0, "is_live": false, "live_status": "not_live"}'
exit 0
"#;
    create_executable_script(&env.bin_dir, "yt-dlp", yt_dlp_script);

    let downloader = ProcessYoutubeDownloader::new(env.config());
    let stage_id = "test-stage-size-fail";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let err = downloader.download(url, target).await.unwrap_err();
    assert_eq!(err, YoutubeDownloadError::FileSizeExceeded);
    assert_eq!(err.code(), "file_size_exceeded");

    let part_path = env.staging_dir.join(format!("{stage_id}.part"));
    let mp3_path = env.staging_dir.join(format!("{stage_id}.mp3"));
    assert!(!part_path.exists(), "part file must be purged");
    assert!(!mp3_path.exists(), "mp3 file must be purged");
}

#[tokio::test]
async fn test_process_downloader_timeout_terminates_and_purges() {
    let env = TestEnvironment::new();

    // Stub for yt-dlp that sleeps longer than timeout and touches a partial file
    let slow_script = r#"#!/bin/sh
output_arg=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
    output_arg="$2"
    shift 2
  else
    shift 1
  fi
done
stage_id=$(echo "$output_arg" | sed 's/\.%(ext)s//')
touch "${stage_id}.part"
touch "${stage_id}.mp3"
sleep 5
exit 0
"#;
    create_executable_script(&env.bin_dir, "yt-dlp", slow_script);

    let mut config = env.config();
    config.timeout = Duration::from_millis(150);
    let downloader = ProcessYoutubeDownloader::new(config);

    let stage_id = "test-stage-timeout";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let err = downloader.download(url, target).await.unwrap_err();
    assert_eq!(err, YoutubeDownloadError::DownloadTimeout);
    assert_eq!(err.code(), "download_timeout");

    // Verify partial files purged
    let part_path = env.staging_dir.join(format!("{stage_id}.part"));
    let mp3_path = env.staging_dir.join(format!("{stage_id}.mp3"));
    assert!(!part_path.exists(), "part file must be purged on timeout");
    assert!(!mp3_path.exists(), "mp3 file must be purged on timeout");
}

#[tokio::test]
async fn test_process_downloader_future_cancellation_reaps_child_process() {
    let env = TestEnvironment::new();
    let pid_file = env.bin_dir.join("child.pid");

    let hang_script = format!(
        r#"#!/bin/sh
echo $$ > "{pid_path}"
# Create partial file to check purge on cancel
output_arg=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
    output_arg="$2"
    shift 2
  else
    shift 1
  fi
done
stage_id=$(echo "$output_arg" | sed 's/\.%(ext)s//')
touch "${{stage_id}}.part"
sleep 30
exit 0
"#,
        pid_path = pid_file.display()
    );
    create_executable_script(&env.bin_dir, "yt-dlp", &hang_script);

    let downloader = ProcessYoutubeDownloader::new(env.config());
    let stage_id = "test-stage-cancel";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let download_fut = downloader.download(url, target);

    // Cancel future after child has started and written pid
    tokio::select! {
        _ = download_fut => panic!("should not complete"),
        _ = async {
            for _ in 0..50 {
                if pid_file.exists() && fs::read_to_string(&pid_file).map(|s| !s.trim().is_empty()).unwrap_or(false) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        } => {}
    }

    let pid_str = fs::read_to_string(&pid_file).expect("pid file should exist");
    let pid: i32 = pid_str.trim().parse().expect("valid pid");

    // Wait a brief moment for async drop reaper
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify child process was reaped (no longer in /proc)
    let is_running = Path::new(&format!("/proc/{pid}")).exists();
    assert!(!is_running, "child process with pid {pid} must be terminated when future is dropped");

    // Verify partial files were purged
    let part_path = env.staging_dir.join(format!("{stage_id}.part"));
    assert!(!part_path.exists(), "partial file must be purged after future cancellation");
}

#[tokio::test]
async fn test_process_downloader_nonexistent_yt_dlp_bin_returns_controlled_error_without_fallback() {
    let env = TestEnvironment::new();

    let invalid_config = ProcessYoutubeDownloaderConfig {
        yt_dlp_bin: Some(PathBuf::from("/nonexistent/bin/yt-dlp-custom")),
        timeout: Duration::from_secs(90),
    };

    let downloader = ProcessYoutubeDownloader::new(invalid_config);
    let stage_id = "test-stage-no-bin";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let err = downloader.download(url, target).await.unwrap_err();
    assert_eq!(err, YoutubeDownloadError::DownloaderNotConfigured);
    assert_eq!(err.code(), "downloader_not_configured");
}

#[tokio::test]
async fn test_process_downloader_rejects_live_streams() {
    let env = TestEnvironment::new();

    let live_script = r#"#!/bin/sh
echo '{"title": "Live Stream", "duration": null, "is_live": true, "live_status": "is_live"}'
exit 0
"#;
    create_executable_script(&env.bin_dir, "yt-dlp", live_script);

    let downloader = ProcessYoutubeDownloader::new(env.config());
    let stage_id = "test-stage-live";
    let url = YouTubeUrl::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let target = YoutubeDownloadTarget {
        stage_id: stage_id.to_string(),
        staging_dir: env.staging_dir.clone(),
    };

    let err = downloader.download(url, target).await.unwrap_err();
    assert_eq!(err, YoutubeDownloadError::DownloadFailed);
    assert_eq!(err.code(), "download_failed");
}
