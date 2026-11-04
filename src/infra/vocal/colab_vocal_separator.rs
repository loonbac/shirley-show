use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::Command;
use uuid::Uuid;

use super::colab_remote_script::generate_remote_script;
use super::colab_vocal_separator_config::ColabVocalSeparatorConfig;
use crate::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

#[derive(serde::Deserialize, Debug)]
struct ColabStartOutput {
    session: Option<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ColabExecutionResult {
    pub url: String,
    pub sha256: String,
    pub duration: f64,
    pub size: u64,
}

/// The shape `colab exec --json` prints: it does not forward the script's
/// stdout, it wraps it.
#[derive(serde::Deserialize)]
struct ColabExecEnvelope {
    output: Option<String>,
}

/// Finds the payload in whatever `colab exec` printed.
///
/// Two shapes have to be handled, and only one of them was:
///
/// 1. A bare JSON line, which is what the script itself prints.
/// 2. The CLI envelope `{"duration":..,"gpu":..,"output":"<script stdout>","status":"success"}`.
///
/// Handling only the first meant every real run was reported as "invalid
/// output", because the payload was nested one level down inside `output`.
/// That is why a working separation looked identical to a broken one.
///
/// The envelope's own `status` field is not proof of success: the kernel runs
/// fine even when the script raises, so `status` stays `"success"` while
/// `output` holds a traceback. Success is only the presence of a parseable
/// payload.
pub fn parse_execution_result(stdout: &str) -> Option<ColabExecutionResult> {
    // Pass 1: look for the payload at the top level of every JSON line.
    if let Some(found) = find_payload(stdout) {
        return Some(found);
    }

    // Pass 2: unwrap the CLI envelope and look inside its `output`.
    //
    // The envelope is pretty-printed across several lines, so scanning for a
    // single line that starts with '{' and ends with '}' never matched it. The
    // whole stdout has to be parsed as one JSON document, taking the first '{'
    // as the start so leading noise does not defeat it.
    if let Some(start) = stdout.find('{') {
        if let Ok(envelope) = serde_json::from_str::<ColabExecEnvelope>(&stdout[start..]) {
            if let Some(inner) = envelope.output {
                if let Some(found) = find_payload(&inner) {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn find_payload(text: &str) -> Option<ColabExecutionResult> {
    for line in text.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(res) = serde_json::from_str::<ColabExecutionResult>(trimmed) {
                return Some(res);
            }
        }
    }
    None
}

/// Drop guard ensuring that "colab stop" is executed even in the event of panics,
/// thread drops, or future cancellation while a remote Colab session is active.
pub struct ColabSessionGuard {
    colab_bin: PathBuf,
    session_id: Option<String>,
    active: bool,
}

impl ColabSessionGuard {
    pub fn new(colab_bin: PathBuf, session_id: String) -> Self {
        Self {
            colab_bin,
            session_id: Some(session_id),
            active: true,
        }
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

impl Drop for ColabSessionGuard {
    fn drop(&mut self) {
        if self.active {
            if let Some(session_id) = self.session_id.take() {
                let _ = std::process::Command::new(&self.colab_bin)
                    .args(["stop", "--session", &session_id])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ColabVocalSeparator {
    config: ColabVocalSeparatorConfig,
    /// Remembers the recent session so a second request inside the retention
    /// window reuses the running machine instead of paying a cold start.
    sessions: Arc<tokio::sync::Mutex<crate::infra::vocal::colab_session_cache::ColabSessionCache>>,
}

impl ColabVocalSeparator {
    pub fn new(config: ColabVocalSeparatorConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(tokio::sync::Mutex::new(
                crate::infra::vocal::colab_session_cache::ColabSessionCache::new(),
            )),
        }
    }

    /// Returns a session id that is still inside the retention window, if any.
    async fn reusable_session(&self) -> Option<String> {
        let guard = self.sessions.lock().await;
        guard.reusable(crate::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION)
    }

    async fn remember_session(&self, session_id: &str) {
        let mut guard = self.sessions.lock().await;
        guard.store(session_id.to_string());
    }

    async fn forget_session(&self) {
        let mut guard = self.sessions.lock().await;
        guard.invalidate();
    }

    /// Confirms the cached session is still alive before trusting it.
    ///
    /// A runtime can disappear on its own: Colab reclaims idle machines and can
    /// preempt a GPU. Handing a dead id to `colab exec` produces a confusing
    /// error far from the cause, so liveness is checked first and a dead id is
    /// dropped.
    async fn session_is_alive(&self, session_id: &str) -> bool {
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["exec", "--session", session_id, "-c", "print(1)"]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        matches!(
            tokio::time::timeout(Duration::from_secs(45), cmd.status()).await,
            Ok(Ok(st)) if st.success()
        )
    }

    /// Schedules the release of the session after the retention window.
    ///
    /// The task is detached on purpose: the request has already finished and the
    /// user has their audio. Holding the response open for three minutes to keep
    /// a machine alive would be worse than the cold start it saves.
    fn schedule_release(&self, session_id: String, colab_bin: PathBuf) {
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            // Claim the release: this task owns it only while its generation is
            // still current. A later request that reuses the session bumps the
            // generation, and this task then stops nothing. Without that check a
            // second request could be killed mid-pipeline by the first timer.
            let generation = {
                let mut guard = sessions.lock().await;
                guard.arm_release(
                    Instant::now() + crate::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION,
                )
            };

            tokio::time::sleep(crate::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION).await;

            // Re-check ownership after sleeping.
            {
                let guard = sessions.lock().await;
                if !guard.should_release(generation) {
                    return;
                }
            }

            let mut cmd = Command::new(colab_bin);
            cmd.kill_on_drop(true);
            cmd.args(["stop", "--session", &session_id]);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            let ok = matches!(
                tokio::time::timeout(Duration::from_secs(60), cmd.status()).await,
                Ok(Ok(st)) if st.success()
            );
            if !ok && std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                eprintln!("[release] could not stop {session_id}; rerun colab stop");
            }
        });
    }

    async fn run_ffprobe_duration(
        &self,
        path: &Path,
        timeout: Duration,
    ) -> Result<f64, VocalSeparationError> {
        let mut cmd = Command::new(&self.config.ffprobe_bin);
        cmd.kill_on_drop(true);
        cmd.args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "format=duration:stream=codec_type,codec_name,duration",
            "-of",
            "json",
        ]);
        cmd.arg(path);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(o)) if o.status.success() => o,
            _ => return Err(VocalSeparationError::InvalidAudioDuration),
        };

        let parsed: FfprobeJsonOutput = serde_json::from_slice(&output.stdout)
            .map_err(|_| VocalSeparationError::InvalidAudioDuration)?;

        let has_audio_stream = parsed
            .streams
            .as_ref()
            .map(|streams| {
                streams
                    .iter()
                    .any(|s| s.codec_type.as_deref() == Some("audio") || s.codec_name.is_some())
            })
            .unwrap_or(false);

        if !has_audio_stream {
            return Err(VocalSeparationError::InvalidAudioDuration);
        }

        let duration_val = parsed.format.and_then(|f| f.duration).or_else(|| {
            parsed
                .streams
                .as_ref()
                .and_then(|s| s.first().and_then(|st| st.duration.clone()))
        });

        let duration_num = match duration_val {
            Some(serde_json::Value::Number(n)) => n
                .as_f64()
                .ok_or(VocalSeparationError::InvalidAudioDuration)?,
            Some(serde_json::Value::String(s)) => s
                .parse::<f64>()
                .map_err(|_| VocalSeparationError::InvalidAudioDuration)?,
            _ => return Err(VocalSeparationError::InvalidAudioDuration),
        };

        if duration_num <= 0.0 {
            return Err(VocalSeparationError::InvalidAudioDuration);
        }

        Ok(duration_num)
    }

    async fn compute_sha256(
        &self,
        path: &Path,
        timeout: Duration,
    ) -> Result<String, VocalSeparationError> {
        let mut cmd = Command::new(&self.config.openssl_bin);
        cmd.kill_on_drop(true);
        cmd.args(["dgst", "-sha256", "-r"]);
        cmd.arg(path);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(o)) if o.status.success() => o,
            _ => return Err(VocalSeparationError::InvalidSeparatorOutput),
        };

        let text = String::from_utf8_lossy(&output.stdout);
        let hash = text
            .split_whitespace()
            .next()
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;
        Ok(hash.to_lowercase())
    }

    async fn stop_session(&self, session_id: &str) -> bool {
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["stop", "--session", session_id]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        matches!(
            tokio::time::timeout(Duration::from_secs(30), cmd.status()).await,
            Ok(Ok(status)) if status.success()
        )
    }

    /// Stops every runtime owned by this account. Used when `colab start`
    /// succeeded but we could not identify the session it created: the GPU is
    /// already allocated, so it must be released before returning an error.
    async fn stop_all_sessions(&self) {
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["stop"]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        let _ = tokio::time::timeout(Duration::from_secs(30), cmd.status()).await;
    }

    /// Resolves the URL that actually downloads bytes.
    ///
    /// Uploaders answer in two shapes. Some return the file URL directly, which
    /// is passed through untouched. Others, tmpfiles among them, return a
    /// landing page whose download anchor carries a tokenised `/dl/` link; that
    /// page must be fetched and the anchor extracted, because rewriting the URL
    /// by hand just redirects back to the page and the "file" ends up being
    /// HTML.
    ///
    /// A URL that yields no link is not silently accepted: the caller would
    /// then try to decrypt an HTML page. It reports a transfer failure, which is
    /// what actually happened.
    async fn resolve_direct_url(
        &self,
        url: &str,
        timeout: Duration,
    ) -> Result<String, VocalSeparationError> {
        if let Some(stripped) = url.strip_prefix("file://") {
            return Ok(format!("file://{stripped}"));
        }

        let probe = self
            .config
            .work_dir
            .join(format!("colab_page_{}.txt", std::process::id()));

        // Two attempts: the landing page is small, and a single transient
        // refusal must not decide the whole transfer.
        let body = crate::infra::vocal::colab_transfer::fetch_with_retry(url, &probe, timeout, 3)
            .await
            .map_err(|e| {
                if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                    eprintln!("[resolve_direct_url] {e}");
                }
                VocalSeparationError::TransferFailed
            })?;
        let _ = tokio::fs::remove_file(&probe).await;

        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
            eprintln!(
                "[resolve_direct_url] body {} bytes, looks_like_html={}",
                body.len(),
                crate::infra::vocal::colab_transfer::looks_like_html(&body)
            );
        }

        // The body is unambiguous: either the payload itself, or a page whose
        // anchor holds the real link.
        if !crate::infra::vocal::colab_transfer::looks_like_html(&body) {
            return Ok(url.to_string());
        }

        let text = String::from_utf8_lossy(&body);
        crate::infra::vocal::colab_transfer::extract_direct_link(&text)
            .ok_or(VocalSeparationError::TransferFailed)
    }

    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        timeout: Duration,
    ) -> Result<(), VocalSeparationError> {
        if let Some(stripped) = url.strip_prefix("file://") {
            tokio::fs::copy(stripped, dest)
                .await
                .map_err(|_| VocalSeparationError::TransferFailed)?;
            return Ok(());
        }

        let bytes = crate::infra::vocal::colab_transfer::fetch_with_retry(url, dest, timeout, 4)
            .await
            .map_err(|e| {
                if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                    eprintln!("[download_file] {e}");
                }
                VocalSeparationError::TransferFailed
            })?;

        // A 200 with an HTML landing page is the failure that hid here before:
        // the transfer "succeeded", a few kilobytes of page were written as if
        // they were audio, and the user got "invalid output" from decryption.
        if crate::infra::vocal::colab_transfer::looks_like_html(&bytes) {
            return Err(VocalSeparationError::TransferFailed);
        }

        // fetch_with_retry already rejected an empty body, so a real payload is
        // guaranteed to be on disk here.
        Ok(())
    }

    async fn execute_pipeline(
        &self,
        session_id: &str,
        request: &VocalSeparationRequest,
        remaining_timeout: Duration,
    ) -> Result<SeparatedAudio, VocalSeparationError> {
        let start = Instant::now();

        // 1. Generate random encryption password
        let password = generate_random_password();

        // 2. Upload source audio to Colab runtime
        let upload_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let mut upload_cmd = Command::new(&self.config.colab_bin);
        upload_cmd.kill_on_drop(true);
        upload_cmd.args([
            "upload",
            "--session",
            session_id,
            request
                .source_path
                .to_str()
                .ok_or(VocalSeparationError::SeparationFailed)?,
            "/content/input_audio",
        ]);
        upload_cmd.stdin(Stdio::null());
        upload_cmd.stdout(Stdio::null());
        upload_cmd.stderr(Stdio::null());

        let upload_status = match tokio::time::timeout(upload_timeout, upload_cmd.status()).await {
            Ok(Ok(st)) if st.success() => st,
            Ok(Ok(_)) => return Err(VocalSeparationError::SeparationFailed),
            Ok(Err(_)) => return Err(VocalSeparationError::SeparationFailed),
            Err(_) => return Err(VocalSeparationError::SeparationTimeout),
        };
        let _ = upload_status;

        // 3. Generate remote Python script and save to local work directory
        tokio::fs::create_dir_all(&self.config.work_dir)
            .await
            .map_err(|_| VocalSeparationError::SeparationFailed)?;

        let script_id = Uuid::new_v4();
        let script_path = self
            .config
            .work_dir
            .join(format!("colab_script_{}.py", script_id));

        let models = ColabVocalSeparatorConfig::models_for_mode(request.mode);
        let python_code = generate_remote_script(
            request.mode,
            models,
            self.config.overlap,
            &password,
            "/content/input_audio",
        );

        tokio::fs::write(&script_path, &python_code)
            .await
            .map_err(|_| VocalSeparationError::SeparationFailed)?;

        // 4. Execute remote script via colab exec
        let exec_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let mut exec_cmd = Command::new(&self.config.colab_bin);
        exec_cmd.kill_on_drop(true);
        exec_cmd.args([
            "exec",
            "--session",
            session_id,
            "--json",
            script_path
                .to_str()
                .ok_or(VocalSeparationError::SeparationFailed)?,
        ]);
        exec_cmd.stdin(Stdio::null());
        exec_cmd.stdout(Stdio::piped());
        exec_cmd.stderr(Stdio::piped());

        let exec_output = match tokio::time::timeout(exec_timeout, exec_cmd.output()).await {
            Ok(Ok(o)) if o.status.success() => o,
            Ok(Ok(_)) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                return Err(VocalSeparationError::SeparationFailed);
            }
            Ok(Err(_)) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                return Err(VocalSeparationError::SeparationFailed);
            }
            Err(_) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                return Err(VocalSeparationError::SeparationTimeout);
            }
        };

        let _ = tokio::fs::remove_file(&script_path).await;

        let exec_stdout = String::from_utf8_lossy(&exec_output.stdout);
        // Diagnostic breadcrumb. When the payload cannot be parsed, the only
        // way to tell "the script failed" from "we misread its output" is to
        // have the raw text, and the product error deliberately hides it.
        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
            eprintln!("--- colab exec stdout ({} bytes) ---", exec_stdout.len());
            eprintln!("{exec_stdout}");
            let exec_stderr = String::from_utf8_lossy(&exec_output.stderr);
            if !exec_stderr.trim().is_empty() {
                eprintln!("--- colab exec stderr ---");
                eprintln!("{exec_stderr}");
            }
        }
        let exec_res = parse_execution_result(&exec_stdout)
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;

        // 5. Resolve the real download link.
        //
        // Uploaders answer in two shapes: the file URL itself, or a landing
        // page that carries a tokenised direct link. Asking for the page and
        // following its download anchor is what makes tmpfiles usable; a naive
        // rewrite of the URL returns the page again.
        let dl_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let direct_url = self.resolve_direct_url(&exec_res.url, dl_timeout).await?;

        let enc_path = self
            .config
            .work_dir
            .join(format!("colab_download_{}.enc", script_id));

        self.download_file(&direct_url, &enc_path, dl_timeout)
            .await?;

        // 6. Decrypt with openssl
        if let Some(parent) = request.output_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let decrypt_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let mut dec_cmd = Command::new(&self.config.openssl_bin);
        dec_cmd.kill_on_drop(true);
        dec_cmd.args([
            "enc",
            "-d",
            "-aes-256-cbc",
            "-pbkdf2",
            "-salt",
            "-pass",
            &format!("pass:{}", password),
            "-in",
            enc_path
                .to_str()
                .ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
            "-out",
            request
                .output_path
                .to_str()
                .ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
        ]);
        dec_cmd.stdin(Stdio::null());
        dec_cmd.stdout(Stdio::null());
        dec_cmd.stderr(Stdio::null());

        let dec_status = match tokio::time::timeout(decrypt_timeout, dec_cmd.status()).await {
            Ok(Ok(st)) if st.success() => st,
            _ => {
                let _ = tokio::fs::remove_file(&enc_path).await;
                return Err(VocalSeparationError::InvalidSeparatorOutput);
            }
        };
        let _ = dec_status;
        let _ = tokio::fs::remove_file(&enc_path).await;

        // 7. Verify SHA-256 against runtime reported hash (S3)
        let verify_sha_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let local_sha = self
            .compute_sha256(&request.output_path, verify_sha_timeout)
            .await?;

        if local_sha != exec_res.sha256.to_lowercase() {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        // 8. Verify duration against runtime reported duration (S3)
        let verify_dur_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let local_duration = self
            .run_ffprobe_duration(&request.output_path, verify_dur_timeout)
            .await?;

        if (local_duration - exec_res.duration).abs() > 2.0 {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        // 9. Inspect output file size
        let size_bytes = tokio::fs::metadata(&request.output_path)
            .await
            .map(|m| m.len())
            .unwrap_or(exec_res.size);

        Ok(SeparatedAudio {
            duration_secs: local_duration.round() as u32,
            size_bytes,
        })
    }
}

fn generate_random_password() -> String {
    use rand::Rng;
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

#[derive(serde::Deserialize, Debug)]
struct FfprobeJsonOutput {
    streams: Option<Vec<FfprobeStreamEntry>>,
    format: Option<FfprobeFormatEntry>,
}

#[derive(serde::Deserialize, Debug)]
struct FfprobeStreamEntry {
    codec_type: Option<String>,
    codec_name: Option<String>,
    duration: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, Debug)]
struct FfprobeFormatEntry {
    duration: Option<serde_json::Value>,
}

impl VocalSeparator for ColabVocalSeparator {
    fn is_remote(&self) -> bool {
        true
    }

    fn probe(&self, _timeout: Duration) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let this = self.clone();
        Box::pin(async move { this.config.validate_token().is_ok() })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            // 1. Source existence check
            if !request.source_path.is_file() {
                return Err(VocalSeparationError::InvalidAudioDuration);
            }

            // 2. Validate token without starting GPU (S4)
            this.config.validate_token()?;

            // 3. Acquire GPU mutual exclusion permit (S7)
            let permit = match this.config.gpu.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => return Err(VocalSeparationError::GpuBusy),
            };

            // 4. Preflight ffprobe duration check (S7: <= 600s) before starting remote GPU session
            let preflight_timeout = request.timeout.min(Duration::from_secs(5));
            let preflight_duration = this
                .run_ffprobe_duration(&request.source_path, preflight_timeout)
                .await?;

            if preflight_duration > 600.0 {
                return Err(VocalSeparationError::DurationExceeded);
            }

            let start = Instant::now();

            // 5. Reuse a recent session, or start one when there is work (S2).
            //
            // Reuse is the product rule: after a request the machine stays up
            // for three minutes, and anything asked inside that window runs on
            // the same runtime. A cached id is only trusted after proving it is
            // still alive, because Colab reclaims idle machines and preempts
            // GPUs on its own.
            let start_timeout = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;

            let mut reused = false;
            let session_id = match this.reusable_session().await {
                Some(cached) if this.session_is_alive(&cached).await => {
                    reused = true;
                    cached
                }
                _ => {
                    this.forget_session().await;

                    let mut start_cmd = Command::new(&this.config.colab_bin);
                    start_cmd.kill_on_drop(true);
                    start_cmd.args(["start", "--gpu", &this.config.gpu_type, "--json"]);
                    start_cmd.stdin(Stdio::null());
                    start_cmd.stdout(Stdio::piped());
                    start_cmd.stderr(Stdio::null());

                    let start_output =
                        match tokio::time::timeout(start_timeout, start_cmd.output()).await {
                            Ok(Ok(o)) if o.status.success() => o,
                            _ => return Err(VocalSeparationError::ColabUnavailable),
                        };

                    // The runtime is already allocated at this point. When the
                    // output cannot be parsed there is no id to stop, so every
                    // runtime is torn down before returning: leaving one running
                    // would burn quota with nobody holding the handle.
                    match serde_json::from_slice::<ColabStartOutput>(&start_output.stdout)
                        .ok()
                        .and_then(|parsed| parsed.session)
                        .filter(|s| !s.trim().is_empty())
                    {
                        Some(id) => id,
                        None => {
                            this.stop_all_sessions().await;
                            return Err(VocalSeparationError::ColabUnavailable);
                        }
                    }
                }
            };

            // The id is recorded now so a concurrent request can find it; its age is
            // refreshed when the work finishes, because the window the product asks
            // for counts from completion, not from when the machine was chosen.
            this.remember_session(&session_id).await;
            if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                eprintln!("[separate] using session {session_id} (reused={reused})");
            }

            // 6. Wrap session in Drop guard to handle panic/cancellation
            let mut session_guard =
                ColabSessionGuard::new(this.config.colab_bin.clone(), session_id.clone());

            // 7. Execute the separation pipeline
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;

            let pipeline_result = this
                .execute_pipeline(&session_id, &request, remaining)
                .await;

            // 8. Teardown.
            //
            // On success the machine is NOT stopped here: it is kept for the
            // retention window so a second request reuses it, and a detached
            // task stops it when the window expires. Holding this response open
            // for three minutes to keep a GPU alive would be worse than the
            // cold start it saves, so the release is a separate task.
            //
            // On failure nothing is reused: the session is stopped now and the
            // cache is cleared, because a failed runtime is not worth keeping.
            match &pipeline_result {
                Ok(_) => {
                    // The age of the cached session is refreshed HERE, when the
                    // work finishes. Stamping it when the session was chosen made
                    // a separation longer than the window expire before it
                    // completed, so the next request paid a cold start while the
                    // previous machine was still alive.
                    {
                        let mut guard = this.sessions.lock().await;
                        guard.refresh_at(Instant::now());
                    }

                    // The Drop guard must not fire while the machine is meant
                    // to stay alive; the scheduled task owns the release now.
                    session_guard.deactivate();
                    this.schedule_release(session_id.clone(), this.config.colab_bin.clone());
                    if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                        eprintln!(
                            "[separate] keeping {session_id} for {:?}",
                            crate::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION
                        );
                    }
                }
                Err(_) => {
                    this.forget_session().await;
                    let stopped = this.stop_session(&session_id).await;
                    if stopped {
                        session_guard.deactivate();
                    }
                    if !stopped {
                        // Leave the guard armed: its Drop runs one more stop.
                        drop(permit);
                        return Err(VocalSeparationError::SeparationFailed);
                    }
                }
            }
            drop(permit);
            pipeline_result
        })
    }
}
