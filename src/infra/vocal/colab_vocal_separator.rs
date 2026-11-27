use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::process::Command;
use uuid::Uuid;

use super::colab_orphan_reaper::stop_ok;
use super::colab_remote_script::generate_remote_script;
use super::colab_vocal_separator_config::{
    ColabVocalSeparatorConfig, cli_silence_timeout_secs, gpu_assign_window, gpu_attempt_timeout,
    gpu_chain, pipeline_reserve,
};
use crate::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

/// The path on the runtime where the remote script writes the separated MP3.
///
/// The script keeps `output.mp3` at this path before encrypting it for the old
/// catbox transfer arc; the official CLI downloads that plain file directly. The
/// path is shared with `colab_remote_script` (which T3 will simplify next).
pub const REMOTE_OUTPUT_PATH: &str = "/content/colab_sep_work/output.mp3";

/// Generates a session name in the `shirley-<8 hex>` shape the official CLI
/// expects. The name is generated client-side with the existing `uuid`
/// dependency; it is never parsed back out of command output.
fn generate_session_name() -> String {
    format!("shirley-{}", &Uuid::new_v4().simple().to_string()[..8])
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

/// Longest slice of the CLI's stderr kept in a diagnostic line.
///
/// The line goes to `docker logs` and has to stay one line: a Python traceback
/// is many, and the interesting part (`TimeoutError: ...`) is its last one.
const DIAGNOSTIC_MAX_CHARS: usize = 300;

/// Trims and flattens a fragment of CLI stderr into a single bounded line.
///
/// The last non-empty line is the one that names the failure, so that is what is
/// kept; whitespace is collapsed because a wrapped traceback would otherwise
/// break the "one line" contract of the log entry.
fn sanitize_diagnostic(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let last = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .next_back()
        .unwrap_or("");
    let flattened = last.split_whitespace().collect::<Vec<_>>().join(" ");
    flattened.chars().take(DIAGNOSTIC_MAX_CHARS).collect()
}

/// Exit code of a finished process, or the name of the signal that killed it.
fn describe_exit(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => code.to_string(),
        None => format!("signal {}", status),
    }
}

/// One-line failure diagnostic for `colab exec`, always written to stderr.
///
/// This is the only channel that survives production. The user-facing error is a
/// fixed product message ("Falló el proceso de separación vocal"), so a failure
/// inside `colab exec` used to leave nothing behind: stdout was empty and the
/// stderr that named the cause was dropped with the child process. The exit code
/// and the CLI's own last stderr line are enough to tell a silence timeout
/// (`TimeoutError: ... no message from the kernel for 30s`) from a missing
/// package or a dead runtime, and neither carries the OAuth token.
fn report_exec_failure(
    sink: Option<&Arc<std::sync::Mutex<Vec<String>>>>,
    session_id: &str,
    status: Option<&std::process::ExitStatus>,
    stderr: &[u8],
) {
    let exit = status
        .map(describe_exit)
        .unwrap_or_else(|| "none (timed out)".to_string());
    let detail = sanitize_diagnostic(stderr);
    let detail = if detail.is_empty() {
        "<no stderr>".to_string()
    } else {
        detail
    };
    let line = format!("[colab exec] fallo session={session_id} exit={exit} stderr={detail}");
    eprintln!("{line}");
    if let Some(sink) = sink {
        if let Ok(mut lines) = sink.lock() {
            lines.push(line);
        }
    }
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
                    .args(["stop", "-s", &session_id])
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

    /// Reads the size of a file on the runtime, or `None` when it is absent.
    ///
    /// This is the evidence the upload is judged on. The CLI status cannot be
    /// used for that: it reports failure on transfers that arrived intact.
    async fn remote_file_size(
        &self,
        session_id: &str,
        remote_path: &str,
        timeout: Duration,
    ) -> Option<u64> {
        let probe = format!(
            "import os\nprint('SIZE=%d' % os.path.getsize({remote_path:?}) if os.path.exists({remote_path:?}) else 'SIZE=absent')"
        );
        // The official CLI only runs scripts from a file (`-f`), so the probe
        // is written to the local work dir and executed with `-f`.
        let probe_path = self
            .config
            .work_dir
            .join(format!("colab_probe_{}.py", Uuid::new_v4()));
        tokio::fs::write(&probe_path, &probe).await.ok()?;
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        // A probe answers in milliseconds, but the CLI's own silence watchdog is
        // 30s and does not know that: a cold kernel that is still starting would
        // be reported as a timeout while the file may well be there. The bound
        // that matters is ours, so the CLI is given the whole step budget.
        let silence_secs = cli_silence_timeout_secs(timeout);
        let silence_arg = silence_secs.to_string();
        cmd.args([
            "exec",
            "-s",
            session_id,
            "--timeout",
            silence_arg.as_str(),
            "-f",
            probe_path.to_str()?,
        ]);
        cmd.stdin(Stdio::null());
        cmd.stderr(Stdio::null());
        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(o)) => o,
            _ => {
                let _ = tokio::fs::remove_file(&probe_path).await;
                return None;
            }
        };
        let _ = tokio::fs::remove_file(&probe_path).await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().find_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("SIZE=")?;
            rest.parse::<u64>().ok()
        })
    }

    async fn session_is_alive(&self, session_id: &str) -> bool {
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["status", "-s", session_id]);
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
    /// user has their audio. Holding the response open for ten minutes to keep
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
                let owns = guard.should_release(generation);
                if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                    eprintln!("[release] generation {generation} owns={owns}");
                }
                if !owns {
                    return;
                }
            }

            let mut cmd = Command::new(colab_bin);
            cmd.kill_on_drop(true);
            cmd.args(["stop", "-s", &session_id]);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::piped());
            let ok = match tokio::time::timeout(Duration::from_secs(60), cmd.output()).await {
                Ok(Ok(o)) => stop_ok(&o.status, &o.stderr),
                _ => false,
            };
            if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                eprintln!("[release] stop {session_id} ok={ok}");
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

    /// Arranca el vigilante que libera runtimes huérfanos.
    ///
    /// Cubre el caso en que el release falló en silencio. Se le pasa el semáforo
    /// de GPU como señal de trabajo en curso: si no quedan permisos, hay una
    /// separación corriendo y apagar mataría un trabajo ya pagado.
    pub fn spawn_reaper(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let gpu = self.config.gpu.clone();
        let sessions = self.sessions.clone();
        let retention = crate::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION;

        let has_active_work = std::sync::Arc::new(move || gpu.available_permits() == 0);
        let inside_retention = std::sync::Arc::new(move || {
            sessions
                .try_lock()
                .map(|guard| {
                    guard.release_deadline().is_some() && guard.reusable(retention).is_some()
                })
                .unwrap_or(true)
        });

        crate::infra::vocal::colab_orphan_reaper::spawn_periodic_reaper(
            self.config.colab_bin.clone(),
            self.config.token_path.clone(),
            interval,
            has_active_work,
            inside_retention,
        )
    }

    async fn stop_session(&self, session_id: &str) -> bool {
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["stop", "-s", session_id]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        let output = match tokio::time::timeout(Duration::from_secs(30), cmd.output()).await {
            Ok(Ok(o)) => o,
            _ => return false,
        };
        stop_ok(&output.status, &output.stderr)
    }

    /// Walks the accelerator chain until one assigns, cycling while the request
    /// budget remains.
    ///
    /// The product rule is that a paid request must not fail because one
    /// accelerator's assignment endpoint answered 503, and that the fastest
    /// device is always preferred: these models are heavy, so the GPU with the
    /// most throughput finishes a separation soonest. The chain is therefore
    /// always ordered by performance (G4, H100, A100, L4) regardless of the
    /// order the configuration names; configuration only widens the set. Each
    /// attempt is bounded by a cap barely above one assignment POST, so a busy
    /// best GPU is abandoned, stopped and retried in the next round instead of
    /// queueing behind it.
    ///
    /// The assignment window is computed ONCE from the budget available when
    /// assignment starts (half of it), and the other half is reserved for
    /// upload/exec/download. Each attempt is bounded by what is left of that fixed
    /// window; re-halving the remaining budget per attempt let 4 rounds of 3 GPUs
    /// spend nearly everything and leave the pipeline seconds.
    ///
    /// A timed-out attempt stops its session before the next candidate so the
    /// account is not left with an active session that turns the following
    /// request into a 412.
    ///
    /// Every candidate the configuration names but the CLI does not accept is
    /// skipped without an attempt (`14` was one).
    async fn assign_session(&self, deadline: Instant) -> Result<String, VocalSeparationError> {
        let candidates = gpu_chain(&self.config.gpu_type, &self.config.gpu_fallbacks);

        // Fixed once: the pipeline reserve never shrinks because we retried.
        let initial_budget = deadline.saturating_duration_since(Instant::now());
        let assign_deadline = Instant::now() + gpu_assign_window(initial_budget);
        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
            eprintln!(
                "[separate] ventana de asignacion {:?} (reserva {:?}), cadena {candidates:?}",
                gpu_assign_window(initial_budget),
                pipeline_reserve(initial_budget)
            );
        }

        for round in 0..self.config.gpu_max_rounds {
            for gpu in &candidates {
                let remaining_window = assign_deadline.saturating_duration_since(Instant::now());
                let attempt_timeout =
                    gpu_attempt_timeout(remaining_window, self.config.gpu_attempt_timeout);
                if attempt_timeout.is_zero() {
                    if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                        eprintln!("[separate] sin presupuesto para asignar GPU ({gpu} pendiente)");
                    }
                    return Err(VocalSeparationError::ColabUnavailable);
                }

                // The official CLI names the session client-side (`-s`), and it
                // only accepts uppercase GPU names (`G4`, `A100`, `L4`).
                let session_name = generate_session_name();
                let mut start_cmd = Command::new(&self.config.colab_bin);
                start_cmd.kill_on_drop(true);
                start_cmd.args(["new", "-s", &session_name, "--gpu", gpu]);
                start_cmd.stdin(Stdio::null());
                start_cmd.stdout(Stdio::null());
                start_cmd.stderr(Stdio::null());

                match tokio::time::timeout(attempt_timeout, start_cmd.status()).await {
                    Ok(Ok(st)) if st.success() => {
                        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                            eprintln!("[separate] la GPU {gpu} asigno (ronda {round})");
                        }
                        return Ok(session_name);
                    }
                    Ok(Ok(_)) => {
                        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                            eprintln!("[separate] la GPU {gpu} no asigno; siguiente");
                        }
                    }
                    Ok(Err(_)) => {
                        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                            eprintln!("[separate] no se pudo ejecutar colab new para {gpu}; siguiente");
                        }
                    }
                    Err(_) => {
                        // A late assignment is a live session nobody is going to
                        // use. `colab new -s NAME` names the session client-side
                        // and the endpoint keeps working after the client gives
                        // up (L4 once answered 120s late), so the abandoned
                        // session may well exist. Leaving it is not harmless:
                        // the next `new` is refused with 412 Precondition Failed
                        // because a session is already active, and it bills GPU
                        // time for work nobody will run.
                        let stopped = self.stop_session(&session_name).await;
                        if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                            eprintln!(
                                "[separate] la GPU {gpu} agoto {:?} sin responder; stop={stopped}; siguiente",
                                attempt_timeout
                            );
                        }
                    }
                }
            }
        }

        Err(VocalSeparationError::ColabUnavailable)
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
            "-s",
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

        // The exit code cannot decide this. `colab upload` exits 1 with
        // "kernel write failed" while still leaving the file byte-perfect on the
        // runtime: measured with a 289.689-byte MP3 whose local and remote
        // sha256 matched exactly. Aborting on the status failed every separation
        // even though the audio had already arrived.
        //
        // So ask the runtime what it holds. A genuinely empty runtime gets one
        // more attempt; a truncated file is not retried, because separating half
        // an audio would "succeed" wrongly.
        let expected_size = match tokio::fs::metadata(&request.source_path).await {
            Ok(m) => m.len(),
            Err(_) => return Err(VocalSeparationError::SeparationFailed),
        };

        let mut accepted = false;
        for attempt in 0..2u8 {
            if attempt > 0 && std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                eprintln!("[upload] reintento: el runtime seguia vacio");
            }

            let exit_success = match tokio::time::timeout(upload_timeout, upload_cmd.status()).await
            {
                Ok(Ok(st)) => st.success(),
                Ok(Err(_)) => false,
                Err(_) => return Err(VocalSeparationError::SeparationTimeout),
            };

            let remote_size = self
                .remote_file_size(session_id, "/content/input_audio", upload_timeout)
                .await;

            match crate::infra::vocal::colab_transfer::upload_verdict(
                exit_success,
                remote_size,
                expected_size,
            ) {
                crate::infra::vocal::colab_transfer::UploadVerdict::Accept => {
                    accepted = true;
                    break;
                }
                crate::infra::vocal::colab_transfer::UploadVerdict::Retry => continue,
                crate::infra::vocal::colab_transfer::UploadVerdict::Reject => break,
            }
        }

        if !accepted {
            return Err(VocalSeparationError::SeparationFailed);
        }

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
            self.config.batch_size,
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

        // The CLI aborts a call whose kernel stays silent for `--timeout`
        // seconds, defaulting to 30. The remote script installs packages and
        // loads the separation models before printing anything, so that default
        // killed real separations after 36s with an empty stdout. The budget left
        // for this step is handed over instead, in whole seconds rounded up, so
        // the only bound that can cut a healthy run is our own tokio timeout.
        let silence_secs = cli_silence_timeout_secs(exec_timeout);
        let silence_arg = silence_secs.to_string();

        let mut exec_cmd = Command::new(&self.config.colab_bin);
        exec_cmd.kill_on_drop(true);
        exec_cmd.args([
            "exec",
            "-s",
            session_id,
            "--timeout",
            silence_arg.as_str(),
            "-f",
            script_path
                .to_str()
                .ok_or(VocalSeparationError::SeparationFailed)?,
        ]);
        exec_cmd.stdin(Stdio::null());
        exec_cmd.stdout(Stdio::piped());
        exec_cmd.stderr(Stdio::piped());

        let exec_output = match tokio::time::timeout(exec_timeout, exec_cmd.output()).await {
            Ok(Ok(o)) if o.status.success() => o,
            Ok(Ok(o)) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                report_exec_failure(
                    self.config.diagnostic_sink.as_ref(),
                    session_id,
                    Some(&o.status),
                    &o.stderr,
                );
                return Err(VocalSeparationError::SeparationFailed);
            }
            Ok(Err(err)) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                // The CLI could not even be spawned (missing binary, no execute
                // permission). Nothing was run, but the cause must still be
                // visible from the log.
                report_exec_failure(
                    self.config.diagnostic_sink.as_ref(),
                    session_id,
                    None,
                    format!("no se pudo ejecutar el binario colab: {err}").as_bytes(),
                );
                return Err(VocalSeparationError::SeparationFailed);
            }
            Err(_) => {
                let _ = tokio::fs::remove_file(&script_path).await;
                // Killed by our own outer bound: the child never got to report
                // anything, so the line records the bound instead of a stderr
                // that does not exist.
                report_exec_failure(
                    self.config.diagnostic_sink.as_ref(),
                    session_id,
                    None,
                    format!("agotado el limite externo de {exec_timeout:?}").as_bytes(),
                );
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

        // 5. Download the separated MP3 straight from the runtime with the
        // official CLI. The old curl/catbox + AES decrypt arc is gone: the
        // remote script leaves the plain `output.mp3` on the runtime and
        // `colab download -s NAME REMOTE LOCAL` fetches it. SHA-256 and
        // duration are still verified below, exactly as before.
        let dl_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        if let Some(parent) = request.output_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut dl_cmd = Command::new(&self.config.colab_bin);
        dl_cmd.kill_on_drop(true);
        dl_cmd.args([
            "download",
            "-s",
            session_id,
            REMOTE_OUTPUT_PATH,
            request
                .output_path
                .to_str()
                .ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
        ]);
        dl_cmd.stdin(Stdio::null());
        dl_cmd.stdout(Stdio::null());
        dl_cmd.stderr(Stdio::null());

        if !matches!(
            tokio::time::timeout(dl_timeout, dl_cmd.status()).await,
            Ok(Ok(st)) if st.success()
        ) {
            return Err(VocalSeparationError::SeparationFailed);
        }

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

            // 2. Validate token without starting GPU (S4), then make it
            // available to the official CLI at its legacy path.
            this.config.validate_token()?;
            this.config.ensure_legacy_token()?;

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
            // for ten minutes, and anything asked inside that window runs on
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

                    // Colab can report an accelerator as available while its
                    // assignment endpoint refuses (503) or hangs until the
                    // client gives up. Rather than spend the whole budget on the
                    // first candidate, walk the chain with a short bound per
                    // attempt and cycle while the request budget lasts. Credits
                    // are not the constraint here: capacity on a given
                    // accelerator is, so rotating is what turns a temporary
                    // outage into a completed separation.
                    let deadline = Instant::now() + start_timeout;
                    this.assign_session(deadline).await?
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
            // for ten minutes to keep a GPU alive would be worse than the cold
            // start it saves, so the release is a separate task.
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
