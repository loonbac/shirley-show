use std::fs;
use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum EncodeMp3Error {
    #[error("input audio file not found")]
    InputNotFound,
    #[error("mp3 encoding process failed")]
    TranscodeFailed,
    #[error("io error during encoding")]
    IoError(#[from] std::io::Error),
}

/// Builds the ffmpeg Command for transcoding an input WAV file to CBR 320 kbps MP3.
///
/// Forces MP3 format output (-f mp3), strips video (-vn), strips path/metadata (-map_metadata -1),
/// and encodes using libmp3lame at 320 kbps.
pub fn build_encode_mp3_cmd(ffmpeg_bin: Option<&Path>, input: &Path, output: &Path) -> Command {
    let bin = ffmpeg_bin.unwrap_or_else(|| Path::new("ffmpeg"));
    let mut cmd = Command::new(bin);
    cmd.args(["-y", "-i"]);
    cmd.arg(input);
    cmd.args([
        "-vn",
        "-codec:a",
        "libmp3lame",
        "-b:a",
        "320k",
        "-map_metadata",
        "-1",
        "-f",
        "mp3",
    ]);
    cmd.arg(output);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

struct EncodeChildGuard(Option<tokio::process::Child>);

impl Drop for EncodeChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.start_kill();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = child.wait().await;
                });
            }
        }
    }
}

/// Transcodes an input WAV file to CBR 320 kbps MP3 using ffmpeg.
/// If the process fails, is cancelled, or produces an empty file,
/// the output is deterministically purged from disk and all subprocesses reaped.
pub async fn encode_mp3_320k(
    ffmpeg_bin: Option<&Path>,
    input: &Path,
    output: &Path,
) -> Result<(), EncodeMp3Error> {
    if !input.is_file() {
        return Err(EncodeMp3Error::InputNotFound);
    }

    let mut cmd = build_encode_mp3_cmd(ffmpeg_bin, input, output);
    cmd.kill_on_drop(true);
    let child = cmd.spawn()?;
    let mut guard = EncodeChildGuard(Some(child));

    let status = match guard.0.as_mut().unwrap().wait().await {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_file(output);
            return Err(EncodeMp3Error::IoError(e));
        }
    };
    guard.0 = None;

    if !status.success() || !output.is_file() {
        let _ = fs::remove_file(output);
        return Err(EncodeMp3Error::TranscodeFailed);
    }

    let meta = fs::metadata(output)?;
    if meta.len() == 0 {
        let _ = fs::remove_file(output);
        return Err(EncodeMp3Error::TranscodeFailed);
    }

    Ok(())
}
