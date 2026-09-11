use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::Command;
use uuid::Uuid;

use super::remote_directory_guard::RemoteDirectoryGuard;
use super::ssh_command_builder::SshCommandBuilder;
use super::ssh_vocal_separator_config::SshVocalSeparatorConfig;
use crate::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

#[derive(Clone, Debug)]
pub struct SshVocalSeparator {
    config: SshVocalSeparatorConfig,
    command_builder: SshCommandBuilder,
}

async fn spawn_probe_retry(cmd: &mut Command) -> Result<tokio::process::Child, std::io::Error> {
    let mut attempts = 0;
    loop {
        match cmd.spawn() {
            Ok(c) => return Ok(c),
            Err(e) if e.raw_os_error() == Some(26) && attempts < 5 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(15)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

impl SshVocalSeparator {
    pub fn new(config: SshVocalSeparatorConfig) -> Self {
        let command_builder = SshCommandBuilder::new(
            config.ssh_bin.clone(),
            config.scp_bin.clone(),
            config.joshua_pc.user.clone(),
            config.joshua_pc.host.clone(),
            config.joshua_pc.key_path.clone(),
            config.joshua_pc.port,
        );
        Self {
            config,
            command_builder,
        }
    }

    pub fn config(&self) -> &SshVocalSeparatorConfig {
        &self.config
    }

    async fn run_ffprobe_preflight(
        &self,
        source_path: &Path,
        timeout: Duration,
    ) -> Result<(f64, String), VocalSeparationError> {
        let mut attempts = 0;
        let output = loop {
            let mut cmd = Command::new(&self.config.ffprobe_bin);
            cmd.kill_on_drop(true);
            cmd.args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "format=duration:stream=codec_type,codec_name",
                "-of",
                "json",
            ]);
            cmd.arg(source_path);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::null());

            match tokio::time::timeout(timeout, cmd.output()).await {
                Ok(Ok(o)) if o.status.success() => break o,
                Ok(Err(e)) if e.raw_os_error() == Some(26) && attempts < 5 => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(15)).await;
                    continue;
                }
                _ => return Err(VocalSeparationError::InvalidAudioDuration),
            }
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

        let codec = parsed
            .streams
            .as_ref()
            .and_then(|s| s.first())
            .and_then(|s| s.codec_name.clone())
            .unwrap_or_else(|| "mp3".to_string());

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

        let ext = match codec.to_ascii_lowercase().as_str() {
            "mp3" => "mp3",
            "wav" => "wav",
            "flac" => "flac",
            "aac" | "alac" | "m4a" => "m4a",
            "ogg" | "vorbis" | "opus" => "ogg",
            _ => source_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp3"),
        }
        .to_string();

        Ok((duration_num, ext))
    }

    async fn probe_or_wait_wake(
        &self,
        remaining_timeout: Duration,
    ) -> Result<(), VocalSeparationError> {
        // 1. Initial 3s probe
        let probe_step = Duration::from_secs(3).min(remaining_timeout);
        let mut probe_cmd = self.command_builder.build_probe(3);
        probe_cmd.kill_on_drop(true);
        probe_cmd.stdin(Stdio::null());
        probe_cmd.stdout(Stdio::null());
        probe_cmd.stderr(Stdio::null());

        let initial_success = match spawn_probe_retry(&mut probe_cmd).await {
            Ok(mut child) => match tokio::time::timeout(probe_step, child.wait()).await {
                Ok(Ok(status)) => status.success(),
                _ => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    false
                }
            },
            Err(_) => false,
        };

        if initial_success {
            return Ok(());
        }

        // 2. Exponential backoff wake-wait loop
        let wake_timeout = self.config.joshua_pc.wake_timeout.min(remaining_timeout);
        let deadline = Instant::now() + wake_timeout;
        let backoffs = [
            Duration::from_secs(2),
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(15),
        ];
        let mut backoff_idx = 0;

        loop {
            if Instant::now() >= deadline {
                return Err(VocalSeparationError::PcWakeTimeout);
            }

            let step_delay = backoffs[backoff_idx];
            if backoff_idx + 1 < backoffs.len() {
                backoff_idx += 1;
            }

            let time_left = deadline.saturating_duration_since(Instant::now());
            if time_left.is_zero() {
                return Err(VocalSeparationError::PcWakeTimeout);
            }

            let sleep_duration = step_delay.min(time_left);
            tokio::time::sleep(sleep_duration).await;

            if Instant::now() >= deadline {
                return Err(VocalSeparationError::PcWakeTimeout);
            }

            let probe_time_left = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(3));
            if probe_time_left.is_zero() {
                return Err(VocalSeparationError::PcWakeTimeout);
            }

            let mut probe_cmd = self
                .command_builder
                .build_probe(probe_time_left.as_secs().max(1) as u32);
            probe_cmd.kill_on_drop(true);
            probe_cmd.stdin(Stdio::null());
            probe_cmd.stdout(Stdio::null());
            probe_cmd.stderr(Stdio::null());

            let probe_success = match spawn_probe_retry(&mut probe_cmd).await {
                Ok(mut child) => match tokio::time::timeout(probe_time_left, child.wait()).await {
                    Ok(Ok(status)) => status.success(),
                    _ => {
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        false
                    }
                },
                Err(_) => false,
            };

            if probe_success {
                return Ok(());
            }
        }
    }

    async fn run_ffprobe_post_verify(
        &self,
        output_path: &Path,
        source_duration: f64,
        timeout: Duration,
    ) -> Result<(f64, u64), VocalSeparationError> {
        let meta = tokio::fs::metadata(output_path)
            .await
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

        if !meta.is_file() {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        let file_size = meta.len();
        if !(1024..=100 * 1024 * 1024).contains(&file_size) {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        let mut attempts = 0;
        let output = loop {
            let mut cmd = Command::new(&self.config.ffprobe_bin);
            cmd.kill_on_drop(true);
            cmd.args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "format=duration,bit_rate:stream=codec_name,bit_rate",
                "-of",
                "json",
            ]);
            cmd.arg(output_path);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::null());

            match tokio::time::timeout(timeout, cmd.output()).await {
                Ok(Ok(o)) if o.status.success() => break o,
                Ok(Err(e)) if e.raw_os_error() == Some(26) && attempts < 5 => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(15)).await;
                    continue;
                }
                _ => return Err(VocalSeparationError::InvalidSeparatorOutput),
            }
        };

        let parsed: FfprobeJsonOutput = serde_json::from_slice(&output.stdout)
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

        let stream = parsed
            .streams
            .as_ref()
            .and_then(|s| s.first())
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;

        let codec = stream
            .codec_name
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        if codec != "mp3" {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        let duration_val = stream
            .duration
            .clone()
            .or_else(|| parsed.format.as_ref().and_then(|f| f.duration.clone()));
        let duration_num = match duration_val {
            Some(serde_json::Value::Number(n)) => n
                .as_f64()
                .ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
            Some(serde_json::Value::String(s)) => s
                .parse::<f64>()
                .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?,
            _ => return Err(VocalSeparationError::InvalidSeparatorOutput),
        };

        if (duration_num - source_duration).abs() > 2.0 {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        let bitrate_val = stream
            .bit_rate
            .clone()
            .or_else(|| parsed.format.as_ref().and_then(|f| f.bit_rate.clone()));
        let bitrate_num = match bitrate_val {
            Some(serde_json::Value::Number(n)) => n
                .as_u64()
                .ok_or(VocalSeparationError::InvalidSeparatorOutput)?,
            Some(serde_json::Value::String(s)) => s
                .parse::<u64>()
                .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?,
            _ => return Err(VocalSeparationError::InvalidSeparatorOutput),
        };

        if !(310_000..=330_000).contains(&bitrate_num) {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        Ok((duration_num, file_size))
    }

    pub async fn check_probe(&self, timeout: Duration) -> bool {
        let probe_step = Duration::from_secs(3).min(timeout);
        let mut probe_cmd = self
            .command_builder
            .build_probe(probe_step.as_secs().max(1) as u32);
        probe_cmd.kill_on_drop(true);
        probe_cmd.stdin(Stdio::null());
        probe_cmd.stdout(Stdio::null());
        probe_cmd.stderr(Stdio::null());

        match spawn_probe_retry(&mut probe_cmd).await {
            Ok(mut child) => match tokio::time::timeout(probe_step, child.wait()).await {
                Ok(Ok(status)) => status.success(),
                _ => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    false
                }
            },
            Err(_) => false,
        }
    }

    async fn cancel_remote_shutdown(&self, timeout: Duration) -> Result<(), VocalSeparationError> {
        let mut cmd = self.command_builder.build_cancel_shutdown();
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(_)) => return Err(VocalSeparationError::SeparationFailed),
            Err(_) => return Err(VocalSeparationError::SeparationTimeout),
        };

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();

        if stderr.contains("password")
            || stderr.contains("denied")
            || stderr.contains("not allowed")
            || stderr.contains("not permitted")
            || stdout.contains("password")
            || stdout.contains("denied")
        {
            return Err(VocalSeparationError::RemoteSudoDenied);
        }

        // Exit code 1 without sudo denial means no shutdown was scheduled, which is harmless
        if output.status.code() == Some(1) {
            return Ok(());
        }

        Err(VocalSeparationError::SeparationFailed)
    }

    async fn schedule_remote_shutdown(&self, timeout: Duration) {
        let mut cmd = self.command_builder.build_schedule_shutdown();
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(out)) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    eprintln!("Aviso: fallo al programar apagado remoto (+60): {err}");
                }
            }
            Ok(Err(e)) => {
                eprintln!("Aviso: error al ejecutar comando de apagado remoto: {e}");
            }
            Err(_) => {
                eprintln!("Aviso: tiempo de espera agotado al programar apagado remoto");
            }
        }
    }
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
    bit_rate: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, Debug)]
struct FfprobeFormatEntry {
    duration: Option<serde_json::Value>,
    bit_rate: Option<serde_json::Value>,
}

impl VocalSeparator for SshVocalSeparator {
    fn probe(&self, timeout: Duration) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let this = self.clone();
        Box::pin(async move { this.check_probe(timeout).await })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            let start = Instant::now();

            // 1. Preflight SSH key check
            if !this.config.joshua_pc.key_path.is_file() {
                return Err(VocalSeparationError::SshKeyMissing);
            }

            // 2. Preflight source check
            if !request.source_path.is_file() {
                return Err(VocalSeparationError::InvalidAudioDuration);
            }

            // 3. Preflight ffprobe on source
            let preflight_timeout = request.timeout.min(Duration::from_secs(5));
            let (source_duration, ext) = this
                .run_ffprobe_preflight(&request.source_path, preflight_timeout)
                .await?;

            if source_duration > 600.0 {
                return Err(VocalSeparationError::DurationExceeded);
            }

            // 4. Probe / wake-wait loop
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            this.probe_or_wait_wake(remaining).await?;

            // 5. Acquire GPU concurrency permit
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let permit = match tokio::time::timeout(
                remaining,
                this.config.gpu.clone().acquire_owned(),
            )
            .await
            {
                Ok(Ok(p)) => p,
                _ => return Err(VocalSeparationError::GpuBusy),
            };

            // 5b. Cancel any pending idle shutdown before creating remote directories or uploading
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let cancel_step = remaining.min(Duration::from_secs(10));
            this.cancel_remote_shutdown(cancel_step).await?;

            // 6. Setup guard and create remote op dir
            let mut guard = RemoteDirectoryGuard::new(
                Some(request.output_path.clone()),
                permit,
                this.command_builder.clone(),
            );

            let op_id = Uuid::new_v4().to_string();
            let remote_dir = format!("/tmp/shirley-vocal/{op_id}");

            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let mkdir_step = remaining.min(Duration::from_secs(10));
            guard
                .run_status(this.command_builder.build_mkdir(&remote_dir), mkdir_step)
                .await?;
            guard.arm_remote_dir(&remote_dir);

            // 7. Upload input audio
            let input_filename = format!("input.{ext}");
            let remote_input_path = format!("{remote_dir}/{input_filename}");
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let upload_step = remaining.min(Duration::from_secs(120));
            guard
                .run_status(
                    this.command_builder
                        .build_upload(&request.source_path, &remote_input_path),
                    upload_step,
                )
                .await?;

            // 8. Run remote pipeline
            let mode_str = request.mode.as_str();
            let (pipeline_cmd, script) =
                this.command_builder
                    .build_pipeline(mode_str, &remote_dir, &input_filename);
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let pipeline_status = guard
                .run_pipeline_script(pipeline_cmd, script, remaining)
                .await?;

            if !pipeline_status.success() {
                guard.kill_and_cleanup().await;
                return Err(VocalSeparationError::SeparationFailed);
            }

            // 9. Download output MP3
            let remote_output_path = format!("{remote_dir}/output.mp3");
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let download_step = remaining.min(Duration::from_secs(60));
            guard
                .run_status(
                    this.command_builder
                        .build_download(&remote_output_path, &request.output_path),
                    download_step,
                )
                .await?;

            // 10. Local ffprobe post-transfer verification
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;
            let post_verify_step = remaining.min(Duration::from_secs(10));
            let (duration_num, size_bytes) = match this
                .run_ffprobe_post_verify(&request.output_path, source_duration, post_verify_step)
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    guard.kill_and_cleanup().await;
                    return Err(e);
                }
            };

            // 11. Complete guard (explicit remote cleanup, preserves local output, releases permit)
            guard.complete().await;

            // 12. Schedule idle shutdown (+60) after successful validation
            let remaining = request
                .timeout
                .checked_sub(start.elapsed())
                .unwrap_or(Duration::from_secs(5));
            let schedule_step = remaining.min(Duration::from_secs(10));
            this.schedule_remote_shutdown(schedule_step).await;

            Ok(SeparatedAudio {
                duration_secs: duration_num.round() as u32,
                size_bytes,
            })
        })
    }
}
