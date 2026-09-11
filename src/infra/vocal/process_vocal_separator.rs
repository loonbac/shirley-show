use std::fs::File;
use std::future::Future;
use std::io::Read;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::Command;
use uuid::Uuid;

use super::process_vocal_separator_config::ProcessVocalSeparatorConfig;
use super::resolve_stem::resolve_stem;
use super::vocal_command_builder::VocalCommandBuilder;
use super::vocal_process_guard::VocalProcessGuard;
use crate::audio::avg_wave::{AvgWaveError, avg_wave};
use crate::audio::encode_mp3::build_encode_mp3_cmd;
use crate::domain::vocal_separation_mode::VocalSeparationMode;
use crate::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

const COROS_RECIPE: [&str; 3] = ["gabox-invertido", "frazer-becruily", "anvuew"];

#[derive(Clone, Debug)]
pub struct ProcessVocalSeparator {
    config: ProcessVocalSeparatorConfig,
}

impl ProcessVocalSeparator {
    pub fn new(config: ProcessVocalSeparatorConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ProcessVocalSeparatorConfig {
        &self.config
    }

    async fn run_ffprobe_preflight(
        &self,
        source_path: &Path,
        timeout: Duration,
    ) -> Result<f64, VocalSeparationError> {
        struct PreflightChildGuard(Option<tokio::process::Child>);
        impl Drop for PreflightChildGuard {
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

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) if e.raw_os_error() == Some(26) && attempts < 5 => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(15)).await;
                    continue;
                }
                Err(_) => return Err(VocalSeparationError::InvalidAudioDuration),
            };
            let mut stdout_pipe = child.stdout.take();
            let mut pguard = PreflightChildGuard(Some(child));

            let read_stdout = async {
                let mut out = Vec::new();
                if let Some(ref mut pipe) = stdout_pipe {
                    let _ = tokio::io::AsyncReadExt::read_to_end(pipe, &mut out).await;
                }
                out
            };
            let wait_child = pguard.0.as_mut().unwrap().wait();
            let combined = async {
                let (status_res, stdout) = tokio::join!(wait_child, read_stdout);
                status_res.map(|status| std::process::Output {
                    status,
                    stdout,
                    stderr: Vec::new(),
                })
            };

            match tokio::time::timeout(timeout, combined).await {
                Ok(Ok(o)) if o.status.success() => {
                    pguard.0 = None;
                    break o;
                }
                Ok(Err(e)) if e.raw_os_error() == Some(26) && attempts < 5 => {
                    pguard.0 = None;
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

    async fn run_ffprobe_post_verify(
        &self,
        guard: &mut VocalProcessGuard,
        output_path: &Path,
        remaining_timeout: Duration,
    ) -> Result<(f64, String, u64), VocalSeparationError> {
        let mut attempts = 0;
        let output = loop {
            let mut cmd = Command::new(&self.config.ffprobe_bin);
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

            let probe_step = remaining_timeout.min(Duration::from_secs(5));
            match guard.run_output(cmd, probe_step).await {
                Ok(o) if o.status.success() => break o,
                Ok(o) => {
                    let is_etxtbsy = o.status.code() == Some(26);
                    if is_etxtbsy && attempts < 5 {
                        attempts += 1;
                        tokio::time::sleep(Duration::from_millis(15)).await;
                        continue;
                    }
                    return Err(VocalSeparationError::TranscodeFailed);
                }
                Err(_) => return Err(VocalSeparationError::TranscodeFailed),
            }
        };

        let parsed: FfprobeJsonOutput = serde_json::from_slice(&output.stdout)
            .map_err(|_| VocalSeparationError::TranscodeFailed)?;

        let stream = parsed
            .streams
            .as_ref()
            .and_then(|s| s.first())
            .ok_or(VocalSeparationError::TranscodeFailed)?;

        let codec = stream
            .codec_name
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        if codec != "mp3" {
            return Err(VocalSeparationError::TranscodeFailed);
        }

        let duration_val = parsed
            .format
            .as_ref()
            .and_then(|f| f.duration.clone())
            .or_else(|| stream.duration.clone());
        let duration_num = match duration_val {
            Some(serde_json::Value::Number(n)) => {
                n.as_f64().ok_or(VocalSeparationError::TranscodeFailed)?
            }
            Some(serde_json::Value::String(s)) => s
                .parse::<f64>()
                .map_err(|_| VocalSeparationError::TranscodeFailed)?,
            _ => return Err(VocalSeparationError::TranscodeFailed),
        };

        let bitrate_val = stream
            .bit_rate
            .clone()
            .or_else(|| parsed.format.as_ref().and_then(|f| f.bit_rate.clone()));
        let bitrate_num = match bitrate_val {
            Some(serde_json::Value::Number(n)) => {
                n.as_u64().ok_or(VocalSeparationError::TranscodeFailed)?
            }
            Some(serde_json::Value::String(s)) => s
                .parse::<u64>()
                .map_err(|_| VocalSeparationError::TranscodeFailed)?,
            _ => return Err(VocalSeparationError::TranscodeFailed),
        };

        Ok((duration_num, codec, bitrate_num))
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

impl VocalSeparator for ProcessVocalSeparator {
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

            // 2. Instrumental preflight before GPU
            // Guarantee PolarFormer checkpoint and config exist locally before acquiring the GPU permit.
            // The karaoke-separator wrapper uses fixed weights paths for PolarFormer without accepting CLI flags,
            // so availability is verified strictly at this preflight stage.
            if request.mode == VocalSeparationMode::Instrumental {
                this.config.validate_polarformer_artifacts()?;
            }

            // 3. Preflight duration verification via ffprobe (<= 600s) before GPU
            let preflight_timeout = request.timeout.min(Duration::from_secs(5));
            let preflight_duration = this
                .run_ffprobe_preflight(&request.source_path, preflight_timeout)
                .await?;
            if preflight_duration > 600.0 {
                return Err(VocalSeparationError::DurationExceeded);
            }

            // 4. Resolve separator binary before acquiring GPU
            let vocal_bin = this.config.resolve_vocal_binary()?;

            // 5. Acquire GPU semaphore permit within remaining timeout
            let start = Instant::now();
            let remaining_for_gpu = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout)?;

            let permit = match tokio::time::timeout(
                remaining_for_gpu,
                this.config.gpu.clone().acquire_owned(),
            )
            .await
            {
                Ok(Ok(p)) => p,
                Ok(Err(_)) => return Err(VocalSeparationError::GpuBusy),
                Err(_) => return Err(VocalSeparationError::GpuBusy),
            };

            // 6. Create isolated operation working directory
            let op_id = Uuid::new_v4().to_string();
            let work_dir = this.config.work_root.join(&op_id);
            tokio::fs::create_dir_all(&work_dir)
                .await
                .map_err(|_| VocalSeparationError::SeparationFailed)?;

            let mut guard =
                VocalProcessGuard::new(work_dir.clone(), Some(request.output_path.clone()), permit);

            // 7. Execute separation pipeline
            match request.mode {
                VocalSeparationMode::Coros => {
                    let mut stems = Vec::with_capacity(3);

                    for model_name in COROS_RECIPE {
                        let remaining_step = request
                            .timeout
                            .checked_sub(start.elapsed())
                            .ok_or(VocalSeparationError::SeparationTimeout);
                        let remaining_step = match remaining_step {
                            Ok(r) => r,
                            Err(e) => {
                                guard.kill_and_wait().await;
                                return Err(e);
                            }
                        };

                        let model_dir = work_dir.join(model_name);
                        if tokio::fs::create_dir_all(&model_dir).await.is_err() {
                            guard.kill_and_wait().await;
                            return Err(VocalSeparationError::SeparationFailed);
                        }

                        let cmd = VocalCommandBuilder::new_chorus(
                            &vocal_bin,
                            model_name,
                            &request.source_path,
                            &model_dir,
                        )
                        .current_dir(&work_dir)
                        .build();

                        let status = match guard.run_status(cmd, remaining_step).await {
                            Ok(s) if s.success() => s,
                            Ok(_) => {
                                guard.kill_and_wait().await;
                                return Err(VocalSeparationError::SeparationFailed);
                            }
                            Err(e) => return Err(e),
                        };
                        let _ = status;

                        let stem = match resolve_stem(&model_dir, model_name) {
                            Ok(p) => p,
                            Err(e) => {
                                guard.kill_and_wait().await;
                                return Err(e);
                            }
                        };
                        stems.push(stem);
                    }

                    // Convert any non-wav stem to wav using ffmpeg under guard so avg_wave (hound) can process it
                    let mut wav_stems = Vec::new();
                    for stem in &stems {
                        if stem.extension().and_then(|e| e.to_str()) == Some("wav") {
                            wav_stems.push(stem.clone());
                        } else {
                            let remaining_conv = request
                                .timeout
                                .checked_sub(start.elapsed())
                                .ok_or(VocalSeparationError::SeparationTimeout);
                            let remaining_conv = match remaining_conv {
                                Ok(r) => r,
                                Err(e) => {
                                    guard.kill_and_wait().await;
                                    return Err(e);
                                }
                            };

                            let wav_path = stem.with_extension("wav");
                            let mut conv = Command::new(&this.config.ffmpeg_bin);
                            conv.args([
                                "-v",
                                "error",
                                "-y",
                                "-i",
                                stem.to_str().unwrap(),
                                "-c:a",
                                "pcm_s24le",
                                wav_path.to_str().unwrap(),
                            ]);
                            conv.stdin(Stdio::null());
                            conv.stdout(Stdio::null());
                            conv.stderr(Stdio::null());

                            match guard.run_status(conv, remaining_conv).await {
                                Ok(s) if s.success() => wav_stems.push(wav_path),
                                _ => {
                                    guard.kill_and_wait().await;
                                    return Err(VocalSeparationError::TranscodeFailed);
                                }
                            }
                        }
                    }

                    // Blend with avg_wave
                    let avg_output = work_dir.join("avg_output.wav");
                    match avg_wave([&wav_stems[0], &wav_stems[1], &wav_stems[2]], &avg_output) {
                        Ok(()) => {}
                        Err(AvgWaveError::MismatchedChannels)
                        | Err(AvgWaveError::MismatchedSampleRate)
                        | Err(AvgWaveError::MismatchedLength) => {
                            guard.kill_and_wait().await;
                            return Err(VocalSeparationError::IncompatibleStems);
                        }
                        Err(_) => {
                            guard.kill_and_wait().await;
                            return Err(VocalSeparationError::InvalidSeparatorOutput);
                        }
                    }

                    // Transcode to MP3 320k under guard
                    let remaining_enc = request
                        .timeout
                        .checked_sub(start.elapsed())
                        .ok_or(VocalSeparationError::SeparationTimeout);
                    let remaining_enc = match remaining_enc {
                        Ok(r) => r,
                        Err(e) => {
                            guard.kill_and_wait().await;
                            return Err(e);
                        }
                    };

                    let enc_cmd = build_encode_mp3_cmd(
                        Some(&this.config.ffmpeg_bin),
                        &avg_output,
                        &request.output_path,
                    );
                    match guard.run_status(enc_cmd, remaining_enc).await {
                        Ok(s) if s.success() && request.output_path.is_file() => {}
                        _ => {
                            guard.kill_and_wait().await;
                            return Err(VocalSeparationError::TranscodeFailed);
                        }
                    }
                }
                VocalSeparationMode::Instrumental => {
                    let remaining_step = request
                        .timeout
                        .checked_sub(start.elapsed())
                        .ok_or(VocalSeparationError::SeparationTimeout);
                    let remaining_step = match remaining_step {
                        Ok(r) => r,
                        Err(e) => {
                            guard.kill_and_wait().await;
                            return Err(e);
                        }
                    };

                    let model_dir = work_dir.join("polarformer");
                    if tokio::fs::create_dir_all(&model_dir).await.is_err() {
                        guard.kill_and_wait().await;
                        return Err(VocalSeparationError::SeparationFailed);
                    }

                    // The karaoke-separator wrapper uses exact argv [modelo, input, output_dir].
                    // PolarFormer weights are loaded directly by the wrapper without CLI flags.
                    let cmd = VocalCommandBuilder::new_instrumental(
                        &vocal_bin,
                        &request.source_path,
                        &model_dir,
                    )
                    .current_dir(&work_dir)
                    .build();

                    let status = match guard.run_status(cmd, remaining_step).await {
                        Ok(s) if s.success() => s,
                        Ok(_) => {
                            guard.kill_and_wait().await;
                            return Err(VocalSeparationError::SeparationFailed);
                        }
                        Err(e) => return Err(e),
                    };
                    let _ = status;

                    let stem = match resolve_stem(&model_dir, "polarformer") {
                        Ok(p) => p,
                        Err(e) => {
                            guard.kill_and_wait().await;
                            return Err(e);
                        }
                    };

                    // Transcode to MP3 320k under guard
                    let remaining_enc = request
                        .timeout
                        .checked_sub(start.elapsed())
                        .ok_or(VocalSeparationError::SeparationTimeout);
                    let remaining_enc = match remaining_enc {
                        Ok(r) => r,
                        Err(e) => {
                            guard.kill_and_wait().await;
                            return Err(e);
                        }
                    };

                    let enc_cmd = build_encode_mp3_cmd(
                        Some(&this.config.ffmpeg_bin),
                        &stem,
                        &request.output_path,
                    );
                    match guard.run_status(enc_cmd, remaining_enc).await {
                        Ok(s) if s.success() && request.output_path.is_file() => {}
                        _ => {
                            guard.kill_and_wait().await;
                            return Err(VocalSeparationError::TranscodeFailed);
                        }
                    }
                }
            }

            // 8. Post-transcode verification
            let meta = match tokio::fs::metadata(&request.output_path).await {
                Ok(m) if m.is_file() => m,
                _ => {
                    guard.kill_and_wait().await;
                    return Err(VocalSeparationError::TranscodeFailed);
                }
            };

            let size_bytes = meta.len();
            if size_bytes == 0 || size_bytes > 104_857_600 {
                guard.kill_and_wait().await;
                return Err(VocalSeparationError::TranscodeFailed);
            }

            // Magic bytes verification
            let mut header = [0u8; 128];
            let is_valid_mp3 = match File::open(&request.output_path) {
                Ok(mut f) => {
                    let bytes_read = f.read(&mut header).unwrap_or(0);
                    let is_id3 = bytes_read >= 3 && &header[0..3] == b"ID3";
                    let is_sync =
                        bytes_read >= 2 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0;
                    is_id3 || is_sync
                }
                Err(_) => false,
            };

            if !is_valid_mp3 {
                guard.kill_and_wait().await;
                return Err(VocalSeparationError::TranscodeFailed);
            }

            // Second ffprobe verification under guard
            let remaining_probe = request
                .timeout
                .checked_sub(start.elapsed())
                .ok_or(VocalSeparationError::SeparationTimeout);
            let remaining_probe = match remaining_probe {
                Ok(r) => r,
                Err(e) => {
                    guard.kill_and_wait().await;
                    return Err(e);
                }
            };

            let (final_duration, codec, bitrate) = match this
                .run_ffprobe_post_verify(&mut guard, &request.output_path, remaining_probe)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    guard.kill_and_wait().await;
                    return Err(e);
                }
            };

            if codec != "mp3" || bitrate != 320000 {
                guard.kill_and_wait().await;
                return Err(VocalSeparationError::TranscodeFailed);
            }

            // Disarm guard: purges workdir, retains output artifact, and releases GPU permit
            guard.disarm();

            Ok(SeparatedAudio {
                duration_secs: final_duration.ceil() as u32,
                size_bytes,
            })
        })
    }
}
