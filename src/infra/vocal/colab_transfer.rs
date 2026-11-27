//! Transport of the separated audio back from the Colab runtime.
//!
//! The old transport arc is gone: AES-256-CBC encryption with openssl, upload of
//! the ciphertext to litterbox/catbox, and download back over curl. The remote
//! script now leaves the plain result at `REMOTE_OUTPUT_PATH` and the official
//! `colab download -s NAME REMOTE LOCAL` command fetches it straight from the
//! runtime.
//!
//! What remains here is the verification of the *input* upload (the audio the
//! CLI uploads to the runtime before separating) and the session retention
//! window. `upload_verdict` decides an upload by asking the runtime what file it
//! actually holds, never by the CLI exit code alone.

use std::time::Duration;

/// How long the runtime is kept alive after a separation finishes.
///
/// A second request inside this window reuses the running machine instead of
/// paying for another cold start. It is a product decision, stated explicitly:
/// ten minutes after the last separation, and only then is the GPU stopped. Every
/// reuse restarts the ten minutes, so a series of separations keeps one machine
/// warm instead of stopping and paying the cold start between them.
pub const SESSION_IDLE_RETENTION: Duration = Duration::from_secs(600);

/// What to do about a `colab upload` attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadVerdict {
    /// The file is on the runtime and intact: carry on.
    Accept,
    /// The transfer may well succeed on a second attempt.
    Retry,
    /// The attempt definitively failed.
    Reject,
}

/// Decides an upload by checking the runtime, never by the CLI exit code alone.
///
/// `colab upload` exits 1 with `kernel write failed` and still leaves the file
/// byte-perfect on the runtime. Measured in production against a 289.689-byte
/// MP3: local and remote sha256 matched exactly while the CLI reported failure.
///
/// Trusting the exit status aborted every separation with `separation_failed`
/// even though the audio was already there, so the exit code cannot be the
/// authority. The runtime file is: a matching byte count proves the payload
/// arrived, and the checksum confirms the caller gets what they sent.
///
/// `remote_size` is `None` when the file is absent from the runtime.
pub fn upload_verdict(
    exit_success: bool,
    remote_size: Option<u64>,
    expected_size: u64,
) -> UploadVerdict {
    match remote_size {
        // A complete payload is accepted whatever the CLI claimed. This is the
        // case that used to fail every request.
        Some(size) if size == expected_size => UploadVerdict::Accept,
        // A present but wrong-size file is a truncated write. Retrying a
        // partial file would quietly separate half an audio, so reject.
        Some(_) => UploadVerdict::Reject,
        // Nothing landed. Retry only when the CLI also reported a failure;
        // an exit 0 with no file means the runtime is not where we think it is.
        None if exit_success => UploadVerdict::Reject,
        None => UploadVerdict::Retry,
    }
}
