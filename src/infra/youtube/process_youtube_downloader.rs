use std::fs::{self, File};
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{self, AsyncReadExt};
use tokio::process::Command;

use crate::domain::youtube_url::YouTubeUrl;
use crate::ports::youtube_downloader::{
    DownloadedYoutubeAudio, YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};

use super::purge_youtube_attempt::purge_youtube_attempt;
use super::resolve_youtube_binary::{resolve_youtube_binary, run_preflight_check};
use super::youtube_process_guard::YoutubeProcessGuard;

#[derive(Debug, Clone)]
pub struct ProcessYoutubeDownloaderConfig {
    pub yt_dlp_bin: Option<PathBuf>,
    pub timeout: Duration,
}

impl Default for ProcessYoutubeDownloaderConfig {
    fn default() -> Self {
        Self {
            yt_dlp_bin: None,
            timeout: Duration::from_secs(90),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessYoutubeDownloader {
    config: ProcessYoutubeDownloaderConfig,
}

impl ProcessYoutubeDownloader {
    pub fn new(config: ProcessYoutubeDownloaderConfig) -> Self {
        Self { config }
    }
}

#[derive(serde::Deserialize)]
struct YtDlpMetadata {
    title: Option<String>,
    duration: Option<f64>,
    is_live: Option<bool>,
    live_status: Option<String>,
}

#[derive(serde::Deserialize)]
struct FfprobeFormat {
    duration: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct FfprobeOutput {
    format: Option<FfprobeFormat>,
}

impl YoutubeDownloader for ProcessYoutubeDownloader {
    fn download(
        &self,
        url: YouTubeUrl,
        target: YoutubeDownloadTarget,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadedYoutubeAudio, YoutubeDownloadError>> + Send>>
    {
        let config = self.config.clone();

        Box::pin(async move {
            let _ = fs::create_dir_all(&target.staging_dir);

            // 1. Resolve binary
            let binary = resolve_youtube_binary(config.yt_dlp_bin.as_deref())?;

            // 2. Preflight checks for ffmpeg and ffprobe
            run_preflight_check("ffmpeg", binary.prepended_path.as_deref()).await?;
            run_preflight_check("ffprobe", binary.prepended_path.as_deref()).await?;

            // 3. Build command
            let mut cmd = Command::new(&binary.program);
            cmd.current_dir(&target.staging_dir);

            if let Some(prepend) = &binary.prepended_path {
                let current_path = std::env::var_os("PATH").unwrap_or_default();
                let mut new_path = prepend.as_os_str().to_os_string();
                new_path.push(":");
                new_path.push(&current_path);
                cmd.env("PATH", new_path);
            }

            cmd.args([
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
                &format!("{}.%(ext)s", target.stage_id),
                url.canonical_url(),
            ]);

            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let child = cmd.spawn().map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => YoutubeDownloadError::DownloaderNotConfigured,
                _ => YoutubeDownloadError::DownloadFailed,
            })?;

            let mut guard = YoutubeProcessGuard::new(
                child,
                target.staging_dir.clone(),
                target.stage_id.clone(),
            );

            let mut stdout_pipe = guard
                .child_mut()
                .and_then(|c| c.stdout.take())
                .ok_or(YoutubeDownloadError::DownloadFailed)?;
            let mut stderr_pipe = guard
                .child_mut()
                .and_then(|c| c.stderr.take())
                .ok_or(YoutubeDownloadError::DownloadFailed)?;

            // Drain stdout and stderr concurrently with child execution by spawning reader tasks before wait
            let stdout_task = tokio::spawn(async move {
                let mut stdout_buf = Vec::new();
                let mut stdout_take = (&mut stdout_pipe).take(1024 * 1024);
                let _ = stdout_take.read_to_end(&mut stdout_buf).await;
                let mut sink = io::sink();
                let _ = io::copy(&mut stdout_pipe, &mut sink).await;
                stdout_buf
            });

            let stderr_task = tokio::spawn(async move {
                let mut stderr_buf = Vec::new();
                let mut stderr_take = (&mut stderr_pipe).take(1024 * 1024);
                let _ = stderr_take.read_to_end(&mut stderr_buf).await;
                let mut sink = io::sink();
                let _ = io::copy(&mut stderr_pipe, &mut sink).await;
                stderr_buf
            });

            let wait_fut = async {
                let status = match guard.child_mut() {
                    Some(c) => c.wait().await,
                    None => {
                        return Err(YoutubeDownloadError::DownloadFailed);
                    }
                };

                let stdout_buf = stdout_task.await.unwrap_or_default();
                let stderr_buf = stderr_task.await.unwrap_or_default();
                Ok((status, stdout_buf, stderr_buf))
            };

            let execution_result = match tokio::time::timeout(config.timeout, wait_fut).await {
                Ok(res) => res,
                Err(_) => {
                    guard.kill_and_wait().await;
                    return Err(YoutubeDownloadError::DownloadTimeout);
                }
            };

            let (status, stdout_buf, stderr_buf) = match execution_result {
                Ok(data) => data,
                Err(err) => {
                    guard.kill_and_wait().await;
                    return Err(err);
                }
            };

            let stdout_str = String::from_utf8_lossy(&stdout_buf);
            let stderr_str = String::from_utf8_lossy(&stderr_buf);

            let status_ok = match status {
                Ok(s) => s.success(),
                Err(_) => false,
            };

            if !status_ok {
                guard.kill_and_wait().await;
                if stderr_str.contains("duration <= 1200")
                    || stdout_str.contains("duration <= 1200")
                {
                    return Err(YoutubeDownloadError::DurationExceeded);
                }
                if stderr_str.contains("max-filesize") || stdout_str.contains("max-filesize") {
                    return Err(YoutubeDownloadError::FileSizeExceeded);
                }
                return Err(YoutubeDownloadError::DownloadFailed);
            }

            // 4. Parse last JSON line from stdout
            let mut metadata: Option<YtDlpMetadata> = None;
            for line in stdout_str.lines().rev() {
                let trimmed = line.trim();
                if trimmed.starts_with('{') && trimmed.ends_with('}') {
                    if let Ok(meta) = serde_json::from_str::<YtDlpMetadata>(trimmed) {
                        metadata = Some(meta);
                        break;
                    }
                }
            }

            let meta = metadata.ok_or_else(|| {
                // If stdout didn't have valid json
                YoutubeDownloadError::DownloadFailed
            })?;

            if meta.is_live == Some(true)
                || matches!(
                    meta.live_status.as_deref(),
                    Some("is_live" | "is_upcoming" | "post_live")
                )
            {
                guard.kill_and_wait().await;
                return Err(YoutubeDownloadError::DownloadFailed);
            }

            if let Some(dur) = meta.duration {
                if dur > 1200.0 {
                    guard.kill_and_wait().await;
                    return Err(YoutubeDownloadError::DurationExceeded);
                }
            }

            // 5. Verify MP3 output file on disk
            let mp3_path = target.staging_dir.join(format!("{}.mp3", target.stage_id));
            let file_metadata = match fs::metadata(&mp3_path) {
                Ok(m) if m.is_file() => m,
                _ => {
                    guard.kill_and_wait().await;
                    return Err(YoutubeDownloadError::DownloadFailed);
                }
            };

            let file_size = file_metadata.len();
            if file_size == 0 {
                guard.kill_and_wait().await;
                return Err(YoutubeDownloadError::InvalidAudioOutput);
            }
            if file_size > 104_857_600 {
                guard.kill_and_wait().await;
                return Err(YoutubeDownloadError::FileSizeExceeded);
            }

            // Verify magic bytes: MP3 check
            let mut header = [0u8; 128];
            let mut mp3_file = File::open(&mp3_path).map_err(|_| {
                purge_youtube_attempt(&target.staging_dir, &target.stage_id);
                YoutubeDownloadError::InvalidAudioOutput
            })?;
            let bytes_read = mp3_file.read(&mut header).unwrap_or(0);
            drop(mp3_file);

            let is_id3 = bytes_read >= 3 && &header[0..3] == b"ID3";
            let is_sync = bytes_read >= 2 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0;
            if !is_id3 && !is_sync {
                guard.kill_and_wait().await;
                return Err(YoutubeDownloadError::InvalidAudioOutput);
            }

            // 6. Real duration verification using ffprobe
            let ffprobe_duration = run_ffprobe_duration(
                &target.staging_dir,
                &target.stage_id,
                binary.prepended_path.as_deref(),
            )
            .await;
            let duration_secs = match ffprobe_duration {
                Ok(dur) if dur > 0.0 && dur <= 1200.0 => dur.ceil() as u32,
                _ => {
                    guard.kill_and_wait().await;
                    return Err(YoutubeDownloadError::DurationExceeded);
                }
            };

            // 7. Atomic rename to .part
            let part_path = target.staging_dir.join(format!("{}.part", target.stage_id));
            if fs::rename(&mp3_path, &part_path).is_err() {
                guard.kill_and_wait().await;
                return Err(YoutubeDownloadError::DownloadFailed);
            }

            // 8. Disarm guard on success
            guard.disarm();

            let title = meta
                .title
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| url.video_id().to_string());

            Ok(DownloadedYoutubeAudio {
                title,
                duration_secs,
                size_bytes: file_size,
            })
        })
    }
}

async fn run_ffprobe_duration(
    staging_dir: &Path,
    stage_id: &str,
    prepended_path: Option<&Path>,
) -> Result<f64, ()> {
    let mut cmd = Command::new("ffprobe");
    cmd.current_dir(staging_dir);

    if let Some(prepend) = prepended_path {
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let mut new_path = prepend.as_os_str().to_os_string();
        new_path.push(":");
        new_path.push(&current_path);
        cmd.env("PATH", new_path);
    }

    cmd.args([
        "-v",
        "error",
        "-select_streams",
        "a:0",
        "-show_entries",
        "format=duration",
        "-of",
        "json",
        &format!("{stage_id}.mp3"),
    ]);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let output = match tokio::time::timeout(Duration::from_secs(5), cmd.output()).await {
        Ok(Ok(o)) if o.status.success() => o,
        _ => return Err(()),
    };

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout).map_err(|_| ())?;
    let duration_val = parsed.format.and_then(|f| f.duration).ok_or(())?;

    match duration_val {
        serde_json::Value::Number(n) => n.as_f64().ok_or(()),
        serde_json::Value::String(s) => s.parse::<f64>().map_err(|_| ()),
        _ => Err(()),
    }
}
