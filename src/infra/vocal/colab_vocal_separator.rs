use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
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

pub fn parse_execution_result(stdout: &str) -> Option<ColabExecutionResult> {
    for line in stdout.lines().rev() {
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
}

impl ColabVocalSeparator {
    pub fn new(config: ColabVocalSeparatorConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ColabVocalSeparatorConfig {
        &self.config
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

    async fn stop_session(&self, session_id: &str) {
        let mut cmd = Command::new(&self.config.colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["stop", "--session", session_id]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        let _ = tokio::time::timeout(Duration::from_secs(30), cmd.status()).await;
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
                .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

        if !resp.status().is_success() {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

        tokio::fs::write(dest, &bytes)
            .await
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

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
            request.source_path.to_str().ok_or(VocalSeparationError::SeparationFailed)?,
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
            script_path.to_str().ok_or(VocalSeparationError::SeparationFailed)?,
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
        let exec_res = parse_execution_result(&exec_stdout)
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;

        // 5. Download encrypted output
        let dl_timeout = remaining_timeout
            .checked_sub(start.elapsed())
            .ok_or(VocalSeparationError::SeparationTimeout)?;

        let enc_path = self
            .config
            .work_dir
            .join(format!("colab_download_{}.enc", script_id));

        self.download_file(&exec_res.url, &enc_path, dl_timeout).await?;

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
            enc_path.to_str().ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
            "-out",
            request.output_path.to_str().ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
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
    use rand::distributions::Alphanumeric;
    use rand::Rng;
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

            // 5. Start Colab session only when there is work (S2)
            let start_timeout = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;

            let mut start_cmd = Command::new(&this.config.colab_bin);
            start_cmd.kill_on_drop(true);
            start_cmd.args(["start", "--gpu", &this.config.gpu_type, "--json"]);
            start_cmd.stdin(Stdio::null());
            start_cmd.stdout(Stdio::piped());
            start_cmd.stderr(Stdio::null());

            let start_output = match tokio::time::timeout(start_timeout, start_cmd.output()).await {
                Ok(Ok(o)) if o.status.success() => o,
                _ => return Err(VocalSeparationError::ColabUnavailable),
            };

            let parsed: ColabStartOutput = serde_json::from_slice(&start_output.stdout)
                .map_err(|_| VocalSeparationError::ColabUnavailable)?;

            let session_id = parsed
                .session
                .filter(|s| !s.trim().is_empty())
                .ok_or(VocalSeparationError::ColabUnavailable)?;

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

            // 8. Normal session teardown: ALWAYS stop session regardless of success/error/verification
            session_guard.deactivate();
            this.stop_session(&session_id).await;
            drop(permit);

            pipeline_result
        })
    }
}
