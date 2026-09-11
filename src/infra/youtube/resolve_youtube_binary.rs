use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use crate::ports::youtube_downloader::YoutubeDownloadError;

#[derive(Debug, Clone)]
pub struct ResolvedYoutubeBinary {
    pub program: OsString,
    pub prepended_path: Option<PathBuf>,
}

pub fn resolve_youtube_binary(
    configured_bin: Option<&Path>,
) -> Result<ResolvedYoutubeBinary, YoutubeDownloadError> {
    match configured_bin {
        Some(path) => {
            if path.as_os_str().is_empty() {
                return Err(YoutubeDownloadError::DownloaderNotConfigured);
            }

            let metadata =
                fs::metadata(path).map_err(|_| YoutubeDownloadError::DownloaderNotConfigured)?;
            if !metadata.is_file() {
                return Err(YoutubeDownloadError::DownloaderNotConfigured);
            }

            // Must have executable permission on Unix
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(YoutubeDownloadError::DownloaderNotConfigured);
            }

            let prepended_path = if path.is_absolute() {
                path.parent().map(|p| p.to_path_buf())
            } else {
                None
            };

            Ok(ResolvedYoutubeBinary {
                program: path.as_os_str().to_os_string(),
                prepended_path,
            })
        }
        None => Ok(ResolvedYoutubeBinary {
            program: OsString::from("yt-dlp"),
            prepended_path: None,
        }),
    }
}

pub async fn run_preflight_check(
    tool_name: &str,
    prepended_path: Option<&Path>,
) -> Result<(), YoutubeDownloadError> {
    let mut cmd = tokio::process::Command::new(tool_name);
    cmd.arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(prepend) = prepended_path {
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let mut new_path = prepend.as_os_str().to_os_string();
        new_path.push(":");
        new_path.push(&current_path);
        cmd.env("PATH", new_path);
    }

    let spawn_result = tokio::time::timeout(Duration::from_secs(5), cmd.status()).await;
    match spawn_result {
        Ok(Ok(status)) if status.success() => Ok(()),
        _ => Err(YoutubeDownloadError::DownloaderNotConfigured),
    }
}
