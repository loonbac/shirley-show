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

/// Transcodes an input WAV file to CBR 320 kbps MP3 using ffmpeg.
///
/// Forces MP3 format output (-f mp3), strips video (-vn), strips path/metadata (-map_metadata -1),
/// and encodes using libmp3lame at 320 kbps. If the process fails or produces an empty file,
/// the output is deterministically purged from disk.
pub async fn encode_mp3_320k(
    ffmpeg_bin: Option<&Path>,
    input: &Path,
    output: &Path,
) -> Result<(), EncodeMp3Error> {
    if !input.is_file() {
        return Err(EncodeMp3Error::InputNotFound);
    }

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

    let output_status = cmd.output().await?;

    if !output_status.status.success() || !output.is_file() {
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
