//! Vast.ai implementation of the `VocalSeparator` port.
//!
//! The control plane is Vast's REST API; the data plane is ordinary ssh/scp
//! against the rented instance. The worker image already contains the models,
//! ffmpeg and the Python environment, so the remote script generated for Colab
//! runs unchanged (see `docs/vast-worker.md` for the login-shell caveat).
//!
//! Nothing here logs `VAST_API_KEY`: the only place it appears is the
//! `Authorization` header of an outbound request.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::colab_remote_script::generate_remote_script;
use super::colab_vocal_separator::{REMOTE_OUTPUT_PATH, parse_execution_result};
use super::colab_vocal_separator_config::{COROS_MODELS, INSTRUMENTAL_MODELS};
use super::vast_vocal_separator_config::VastVocalSeparatorConfig;
use crate::domain::vocal_separation_mode::VocalSeparationMode;
use crate::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

const REMOTE_INPUT_PATH: &str = "/content/input_audio";

/// 24 GB-class cards that run these RoFormers economically. No Blackwell:
/// the product decision is cheapest adequate GPU, not the fastest one.
const ECONOMIC_GPUS: [&str; 6] = [
    "rtx 3090",
    "rtx 4090",
    "rtx a5000",
    "rtx a6000",
    "l4",
    "a10",
];

#[derive(Clone, Debug)]
struct Lease {
    instance_id: String,
    host: String,
    port: u16,
    last_used: Instant,
}

#[derive(Clone, Debug, Deserialize)]
struct Offer {
    id: Value,
    dph_total: Option<f64>,
    gpu_name: Option<String>,
    geolocation: Option<String>,
    verified: Option<bool>,
    rentable: Option<bool>,
    rented: Option<bool>,
    reliability2: Option<f64>,
    num_gpus: Option<u64>,
    gpu_ram: Option<u64>,
    cuda_max_good: Option<f64>,
    inet_down: Option<f64>,
    disk_space: Option<u64>,
}

impl Offer {
    fn id(&self) -> Option<String> {
        match &self.id {
            Value::Number(n) => n.as_u64().map(|n| n.to_string()),
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            _ => None,
        }
    }

    fn acceptable(&self, max_dph: f64) -> bool {
        let gpu = self
            .gpu_name
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !ECONOMIC_GPUS.iter().any(|allowed| gpu == *allowed) {
            return false;
        }
        // Vast hosts in these regions broke package downloads during the live
        // probe; the worker image removes installs, but the network is still
        // unreliable enough to exclude them.
        let geo = self.geolocation.as_deref().unwrap_or("").to_ascii_uppercase();
        if geo
            .split(|c: char| !c.is_alphanumeric())
            .any(|part| part == "CN" || part == "HK")
            || geo.contains("CHINA")
            || geo.contains("HONG KONG")
        {
            return false;
        }
        self.verified == Some(true)
            && self.rentable == Some(true)
            && self.rented == Some(false)
            && self.reliability2.is_some_and(|v| v >= 0.99)
            && self.num_gpus == Some(1)
            && self.gpu_ram.is_some_and(|v| v >= 20_000)
            && self.cuda_max_good.is_some_and(|v| v >= 12.4)
            && self.inet_down.is_some_and(|v| v >= 500.0)
            && self.disk_space.is_some_and(|v| v >= 40)
            && self.dph_total.is_some_and(|v| v.is_finite() && v >= 0.0 && v <= max_dph)
            && self.id().is_some()
    }
}

/// Vast wraps single-instance responses as `{"instances": {...}}` and list
/// responses as `{"instances": [...]}`. Older/unwrapped shapes still occur,
/// so both are accepted rather than trusting one guess.
fn unwrap_instance(value: &Value) -> Option<Value> {
    match value {
        Value::Object(map) => match map.get("instances") {
            Some(Value::Object(inner)) => Some(Value::Object(inner.clone())),
            Some(Value::Array(items)) => items.first().cloned(),
            _ => Some(value.clone()),
        },
        _ => None,
    }
}

fn unwrap_instances(value: &Value) -> Vec<Value> {
    match value {
        Value::Object(map) => match map.get("instances") {
            Some(Value::Array(items)) => items.clone(),
            Some(Value::Object(inner)) => vec![Value::Object(inner.clone())],
            _ => Vec::new(),
        },
        Value::Array(items) => items.clone(),
        _ => Vec::new(),
    }
}

fn instance_state(instance: &Value) -> String {
    instance
        .get("actual_status")
        .or_else(|| instance.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn is_dead_state(state: &str) -> bool {
    matches!(
        state,
        "exited" | "offline" | "error" | "terminated" | "failed" | "dead"
    )
}

fn parse_port(value: &Value) -> Option<u16> {
    match value {
        Value::String(text) => text.trim().parse::<u16>().ok(),
        Value::Number(number) => number.as_u64().and_then(|n| u16::try_from(n).ok()),
        _ => None,
    }
}

/// Direct SSH wins: `public_ipaddr` may carry whitespace and the port arrives
/// as the string `ports["22/tcp"][0].HostPort`. When Vast exposes only proxy
/// SSH, fall back to `ssh_host` + `ssh_port`.
fn connection_details(instance: &Value) -> Option<(String, u16)> {
    let direct_ip = instance
        .get("public_ipaddr")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|host| !host.is_empty());
    let direct_port = instance
        .get("ports")
        .and_then(|ports| ports.get("22/tcp"))
        .and_then(Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("HostPort"))
        .and_then(parse_port);
    if let (Some(host), Some(port)) = (direct_ip, direct_port) {
        return Some((host.to_string(), port));
    }
    let proxy_host = instance
        .get("ssh_host")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|host| !host.is_empty());
    let proxy_port = instance.get("ssh_port").and_then(parse_port);
    match (proxy_host, proxy_port) {
        (Some(host), Some(port)) => Some((host.to_string(), port)),
        _ => None,
    }
}

/// Set when the caller drops the `separate()` future; the detached task then
/// destroys the instance instead of leaving it to bill until next boot.
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct VastVocalSeparator {
    config: VastVocalSeparatorConfig,
    client: reqwest::Client,
    lease: Arc<Mutex<Option<Lease>>>,
}

impl VastVocalSeparator {
    pub fn new(config: VastVocalSeparatorConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.http_timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            client,
            lease: Arc::new(Mutex::new(None)),
        }
    }

    /// Errors never carry the key: only the status code and the path are used.
    async fn api(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, VocalSeparationError> {
        let url = format!(
            "{}/{}",
            self.config.api_base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut request = self
            .client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", self.config.api_key));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = tokio::time::timeout(self.config.http_timeout, request.send())
            .await
            .map_err(|_| VocalSeparationError::ColabUnavailable)?
            .map_err(|_| VocalSeparationError::ColabUnavailable)?;
        let status = response.status();
        let text = tokio::time::timeout(self.config.http_timeout, response.text())
            .await
            .map_err(|_| VocalSeparationError::ColabUnavailable)?
            .map_err(|_| VocalSeparationError::ColabUnavailable)?;
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(VocalSeparationError::ColabUnavailable);
        }
        if !status.is_success() {
            return Err(VocalSeparationError::ColabUnavailable);
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|_| VocalSeparationError::ColabUnavailable)
    }

    async fn destroy_instance(&self, instance_id: &str) -> bool {
        let path = format!("instances/{instance_id}/");
        matches!(self.api(Method::DELETE, &path, None).await, Ok(_))
    }

    /// Releases every instance this app left behind. Called on application boot
    /// so a hard restart cannot leave an orphan billing indefinitely.
    pub async fn destroy_orphans(&self) -> usize {
        let Ok(listing) = self.api(Method::GET, "instances/", None).await else {
            return 0;
        };
        let mut removed = 0;
        for instance in unwrap_instances(&listing) {
            let label = instance.get("label").and_then(Value::as_str).unwrap_or("");
            let id = instance
                .get("id")
                .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_string)));
            if label != self.config.label {
                continue;
            }
            if let Some(id) = id
                && self.destroy_instance(&id).await
            {
                removed += 1;
                let mut guard = self.lease.lock().await;
                if guard.as_ref().is_some_and(|current| current.instance_id == id) {
                    *guard = None;
                }
            }
        }
        removed
    }

    async fn ensure_ssh_key(&self) -> Result<(PathBuf, String), VocalSeparationError> {
        let dir = self.config.data_dir.join("vast").join("ssh");
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|_| VocalSeparationError::SeparationFailed)?;
        let private = dir.join("id_ed25519");
        let public = dir.join("id_ed25519.pub");
        if !private.is_file() || !public.is_file() {
            let mut cmd = Command::new(&self.config.ssh_keygen_bin);
            cmd.kill_on_drop(true);
            cmd.args([
                "-t",
                "ed25519",
                "-N",
                "",
                "-f",
                private
                    .to_str()
                    .ok_or(VocalSeparationError::SeparationFailed)?,
            ]);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            match tokio::time::timeout(Duration::from_secs(30), cmd.status()).await {
                Ok(Ok(status)) if status.success() => {}
                _ => return Err(VocalSeparationError::SeparationFailed),
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600))
                .map_err(|_| VocalSeparationError::SeparationFailed)?;
        }
        let key = tokio::fs::read_to_string(&public)
            .await
            .map_err(|_| VocalSeparationError::SeparationFailed)?
            .trim()
            .to_string();
        if key.is_empty() {
            return Err(VocalSeparationError::SeparationFailed);
        }
        let registered = self.api(Method::GET, "ssh/", None).await?;
        if !registered.to_string().contains(&key) {
            self.api(Method::POST, "ssh/", Some(json!({ "ssh_key": key })))
                .await?;
        }
        Ok((private, key))
    }

    fn bundle_query(&self) -> Value {
        json!({
            "verified": true,
            "rentable": true,
            "rented": false,
            "reliability2": 0.99,
            "num_gpus": 1,
            "gpu_ram": 20_000,
            "cuda_max_good": 12.4,
            "inet_down": 500,
            "disk_space": 40,
            "type": "on-demand",
            "order": [["dph_total", "asc"]],
            "limit": 100
        })
    }

    async fn offers(&self) -> Result<Vec<Offer>, VocalSeparationError> {
        let response = self
            .api(Method::POST, "bundles/", Some(self.bundle_query()))
            .await?;
        let values = response
            .get("offers")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| response.as_array().cloned())
            .unwrap_or_default();
        let mut offers: Vec<Offer> = values
            .into_iter()
            .filter_map(|value| serde_json::from_value(value).ok())
            .filter(|offer: &Offer| offer.acceptable(self.config.max_dph))
            .collect();
        offers.sort_by(|a, b| {
            a.dph_total
                .unwrap_or(f64::MAX)
                .total_cmp(&b.dph_total.unwrap_or(f64::MAX))
        });
        Ok(offers)
    }

    async fn instance_status(&self, instance_id: &str) -> Option<Value> {
        self.api(
            Method::GET,
            &format!("instances/{instance_id}/"),
            None,
        )
        .await
        .ok()
        .and_then(|value| unwrap_instance(&value))
    }

    async fn instance_is_running(&self, instance_id: &str) -> bool {
        self.instance_status(instance_id)
            .await
            .is_some_and(|status| instance_state(&status) == "running")
    }

    async fn rent_offer(&self, offer: &Offer, deadline: Instant) -> Result<Lease, VocalSeparationError> {
        let id = offer
            .id()
            .ok_or(VocalSeparationError::ColabUnavailable)?;
        let response = self
            .api(
                Method::PUT,
                &format!("asks/{id}/"),
                Some(json!({
                    "image": self.config.image,
                    "disk": 40,
                    "runtype": "ssh",
                    "label": self.config.label,
                })),
            )
            .await?;
        let instance_id = response
            .get("new_contract")
            .or_else(|| response.get("instance_id"))
            .or_else(|| response.get("id"))
            .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(str::to_string)))
            .ok_or(VocalSeparationError::ColabUnavailable)?;

        loop {
            if Instant::now() >= deadline {
                break;
            }
            if let Some(status) = self.instance_status(&instance_id).await {
                let state = instance_state(&status);
                if state == "running" {
                    if let Some((host, port)) = connection_details(&status) {
                        return Ok(Lease {
                            instance_id,
                            host,
                            port,
                            last_used: Instant::now(),
                        });
                    }
                }
                if is_dead_state(&state) {
                    break;
                }
            }
            tokio::time::sleep(self.config.poll_interval.min(Duration::from_secs(5))).await;
        }
        let _ = self.destroy_instance(&instance_id).await;
        Err(VocalSeparationError::ColabUnavailable)
    }

    async fn acquire_instance(
        &self,
        deadline: Instant,
    ) -> Result<Lease, VocalSeparationError> {
        for offer in self.offers().await? {
            if Instant::now() >= deadline {
                break;
            }
            let rent_deadline = Instant::now()
                + self
                    .config
                    .launch_timeout
                    .min(deadline.saturating_duration_since(Instant::now()));
            match self.rent_offer(&offer, rent_deadline).await {
                Ok(lease) => return Ok(lease),
                Err(_) => continue, // try the next offer
            }
        }
        Err(VocalSeparationError::ColabUnavailable)
    }

    async fn forget_instance(&self, id: &str) {
        let mut guard = self.lease.lock().await;
        if guard.as_ref().is_some_and(|lease| lease.instance_id == id) {
            *guard = None;
        }
    }

    async fn remember_instance(&self, mut lease: Lease) {
        lease.last_used = Instant::now();
        let mut guard = self.lease.lock().await;
        *guard = Some(lease);
    }

    /// Detached ten-minute timer. A reuse refreshes `last_used`, so an older
    /// timer exits without destroying a GPU that is now working again.
    fn schedule_release(&self, instance_id: String) {
        let this = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(this.config.retention).await;
            let expired = this
                .lease
                .lock()
                .await
                .as_ref()
                .is_some_and(|lease| {
                    lease.instance_id == instance_id
                        && lease.last_used.elapsed() >= this.config.retention
                });
            if expired {
                let _ = this.destroy_instance(&instance_id).await;
                this.forget_instance(&instance_id).await;
            }
        });
    }

    /// Independent safety net for instances a cancelled process or a failed
    /// release timer left behind. While work is in flight the GPU semaphore has
    /// no free permit, so a rental that is still being set up is never killed.
    pub fn spawn_safety_reaper(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if this.config.gpu.available_permits() == 0 {
                    continue;
                }
                let Ok(listing) = this.api(Method::GET, "instances/", None).await else {
                    continue;
                };
                let current = this.lease.lock().await.clone();
                for instance in unwrap_instances(&listing) {
                    let label = instance.get("label").and_then(Value::as_str).unwrap_or("");
                    if label != this.config.label {
                        continue;
                    }
                    let id = instance.get("id").and_then(|v| {
                        v.as_u64()
                            .map(|n| n.to_string())
                            .or_else(|| v.as_str().map(str::to_string))
                    });
                    let Some(id) = id else {
                        continue;
                    };
                    let still_warm = current.as_ref().is_some_and(|lease| {
                        lease.instance_id == id && lease.last_used.elapsed() < this.config.retention
                    });
                    if still_warm {
                        continue;
                    }
                    let _ = this.destroy_instance(&id).await;
                    this.forget_instance(&id).await;
                }
            }
        })
    }

    fn ssh_args(&self, lease: &Lease, private_key: &Path) -> Vec<String> {
        let connect_secs = self.config.http_timeout.as_secs().max(1).to_string();
        vec![
            "-i".to_string(),
            private_key.to_string_lossy().into_owned(),
            "-p".to_string(),
            lease.port.to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            format!("ConnectTimeout={connect_secs}"),
            format!("root@{}", lease.host),
        ]
    }

    async fn scp(
        &self,
        lease: &Lease,
        private_key: &Path,
        source: &str,
        destination: &str,
        timeout: Duration,
    ) -> Result<(), VocalSeparationError> {
        let mut cmd = Command::new(&self.config.scp_bin);
        cmd.kill_on_drop(true);
        let mut args = self.ssh_args(lease, private_key);
        args.push("-r".to_string());
        args.push(source.to_string());
        args.push(destination.to_string());
        cmd.args(&args);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        match tokio::time::timeout(timeout, cmd.status()).await {
            Ok(Ok(status)) if status.success() => Ok(()),
            _ => Err(VocalSeparationError::SeparationFailed),
        }
    }

    async fn ssh_command(
        &self,
        lease: &Lease,
        private_key: &Path,
        remote_command: &str,
        stdin_file: Option<std::fs::File>,
        timeout: Duration,
    ) -> Result<Output, VocalSeparationError> {
        let mut cmd = Command::new(&self.config.ssh_bin);
        cmd.kill_on_drop(true);
        let mut args = self.ssh_args(lease, private_key);
        args.push(remote_command.to_string());
        cmd.args(&args);
        cmd.stdin(match stdin_file {
            Some(file) => Stdio::from(file),
            None => Stdio::null(),
        });
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());
        match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(output)) if output.status.success() => Ok(output),
            _ => Err(VocalSeparationError::SeparationFailed),
        }
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
            Ok(Ok(output)) if output.status.success() => output,
            _ => return Err(VocalSeparationError::InvalidAudioDuration),
        };
        let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)
            .map_err(|_| VocalSeparationError::InvalidAudioDuration)?;
        let has_audio = parsed
            .streams
            .as_ref()
            .is_some_and(|streams| {
                streams
                    .iter()
                    .any(|s| s.codec_type.as_deref() == Some("audio") || s.codec_name.is_some())
            });
        if !has_audio {
            return Err(VocalSeparationError::InvalidAudioDuration);
        }
        let duration = parsed.format.and_then(|f| f.duration).or_else(|| {
            parsed
                .streams
                .as_ref()
                .and_then(|s| s.first())
                .and_then(|entry| entry.duration.clone())
        });
        let number = match duration {
            Some(Value::Number(n)) => n,
            Some(Value::String(s)) => s
                .parse::<f64>()
                .ok()
                .and_then(|n| serde_json::Number::from_f64(n))
                .ok_or(VocalSeparationError::InvalidAudioDuration)?,
            _ => return Err(VocalSeparationError::InvalidAudioDuration),
        };
        let seconds = number
            .as_f64()
            .filter(|value| *value > 0.0)
            .ok_or(VocalSeparationError::InvalidAudioDuration)?;
        Ok(seconds)
    }

    async fn sha256(&self, path: &Path, timeout: Duration) -> Result<String, VocalSeparationError> {
        let mut cmd = Command::new(&self.config.openssl_bin);
        cmd.kill_on_drop(true);
        cmd.args(["dgst", "-sha256", "-r"]);
        cmd.arg(path);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());
        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(output)) if output.status.success() => output,
            _ => return Err(VocalSeparationError::InvalidSeparatorOutput),
        };
        let hash = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?
            .to_lowercase();
        Ok(hash)
    }

    async fn run_pipeline(
        &self,
        lease: &Lease,
        private_key: &Path,
        request: &VocalSeparationRequest,
        deadline: Instant,
    ) -> Result<SeparatedAudio, VocalSeparationError> {
        let left = || {
            deadline
                .saturating_duration_since(Instant::now())
                .min(self.config.launch_timeout)
        };

        // 1. Upload audio. The worker image has no install step; the script is
        //    the exact one used for Colab, so both paths share its contract.
        let source = request
            .source_path
            .to_str()
            .ok_or(VocalSeparationError::SeparationFailed)?;
        self.scp(
            lease,
            private_key,
            source,
            &format!("root@{}:{REMOTE_INPUT_PATH}", lease.host),
            left(),
        )
        .await?;

        let work_dir = self.config.data_dir.join("vast").join("work");
        tokio::fs::create_dir_all(&work_dir)
            .await
            .map_err(|_| VocalSeparationError::SeparationFailed)?;
        let script_path = work_dir.join(format!("vast_script_{}.py", Uuid::new_v4()));
        let models = match request.mode {
            VocalSeparationMode::Coros => COROS_MODELS,
            VocalSeparationMode::Instrumental => INSTRUMENTAL_MODELS,
        };
        let script = generate_remote_script(
            request.mode,
            &models,
            self.config.overlap,
            self.config.batch_size,
            "",
            REMOTE_INPUT_PATH,
        );
        tokio::fs::write(&script_path, script)
            .await
            .map_err(|_| VocalSeparationError::SeparationFailed)?;

        // A login shell is required: Vast replaces the image entrypoint, and the
        // worker image exports its venv PATH from /etc/profile.d/venv.sh.
        let script_file = std::fs::File::open(&script_path)
            .map_err(|_| VocalSeparationError::SeparationFailed)?;
        let output = self
            .ssh_command(
                lease,
                private_key,
                "bash -lc 'cd /content && python -'",
                Some(script_file),
                left(),
            )
            .await?;
        let _ = tokio::fs::remove_file(&script_path).await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = parse_execution_result(&stdout)
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;

        // 2. Download the plain MP3 the script leaves on the worker.
        if let Some(parent) = request.output_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let destination = request
            .output_path
            .to_str()
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;
        self.scp(
            lease,
            private_key,
            &format!("root@{}:{REMOTE_OUTPUT_PATH}", lease.host),
            destination,
            left(),
        )
        .await?;

        // 3. Same verification as Colab: SHA-256 and ffprobe duration against
        //    the single JSON line printed by the shared script.
        let local_hash = self.sha256(&request.output_path, left()).await?;
        if local_hash != result.sha256.to_lowercase() {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }
        let local_duration = self
            .run_ffprobe_duration(&request.output_path, left())
            .await?;
        if (local_duration - result.duration).abs() > 2.0 {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }
        let size_bytes = tokio::fs::metadata(&request.output_path)
            .await
            .map(|m| m.len())
            .unwrap_or(result.size);

        // Best-effort scratch cleanup on the rented machine.
        let _ = self
            .ssh_command(
                lease,
                private_key,
                &format!("rm -f {REMOTE_INPUT_PATH} {REMOTE_OUTPUT_PATH}"),
                None,
                Duration::from_secs(5).min(left()),
            )
            .await;
        Ok(SeparatedAudio {
            duration_secs: local_duration.round() as u32,
            size_bytes,
        })
    }

    /// True when an instance from an earlier separation is still rented and
    /// inside the ten-minute retention window. A dead instance is destroyed at
    /// once instead of being offered to the next request.
    pub async fn warm_instance_available(&self) -> bool {
        let lease = {
            let guard = self.lease.lock().await;
            match guard.as_ref() {
                Some(lease) if lease.last_used.elapsed() < self.config.retention => lease.clone(),
                _ => return false,
            }
        };
        if self.instance_is_running(&lease.instance_id).await {
            return true;
        }
        let _ = self.destroy_instance(&lease.instance_id).await;
        self.forget_instance(&lease.instance_id).await;
        false
    }

    /// Destroys all instances carrying this app's label. Used on boot.
    pub async fn reap_orphans_on_boot(&self) -> usize {
        self.destroy_orphans().await
    }
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Option<Vec<FfprobeStream>>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    duration: Option<Value>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<Value>,
}

impl VastVocalSeparator {
    async fn separate_inner(
        &self,
        request: VocalSeparationRequest,
        cancelled: Arc<AtomicBool>,
    ) -> Result<SeparatedAudio, VocalSeparationError> {
        if !request.source_path.is_file() {
            return Err(VocalSeparationError::InvalidAudioDuration);
        }
        let permit = match self.config.gpu.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => return Err(VocalSeparationError::GpuBusy),
        };
        let preflight = self
            .run_ffprobe_duration(
                &request.source_path,
                request.timeout.min(Duration::from_secs(5)),
            )
            .await?;
        if preflight > 600.0 {
            return Err(VocalSeparationError::DurationExceeded);
        }

        let deadline = Instant::now() + request.timeout;
        let (private_key, _) = self.ensure_ssh_key().await?;
        // S3/S4: a GPU already rented for this app is reused instead of
        // paying a cold start for the next separation.
        let lease = if self.warm_instance_available().await {
            match self.lease.lock().await.clone() {
                Some(lease) => lease,
                None => self.acquire_instance(deadline).await?,
            }
        } else {
            self.acquire_instance(deadline).await?
        };

        // A rental that lands after the deadline or after the caller went away
        // has no consumer; destroy it here instead of caching a billing leak.
        if cancelled.load(Ordering::SeqCst) || Instant::now() >= deadline {
            let _ = self.destroy_instance(&lease.instance_id).await;
            self.forget_instance(&lease.instance_id).await;
            drop(permit);
            return Err(VocalSeparationError::SeparationTimeout);
        }

        let result = self
            .run_pipeline(&lease, &private_key, &request, deadline)
            .await;

        // A healthy instance is kept for ten minutes after success OR after
        // a failed separation; only a proven dead one is destroyed now.
        // Cancellation is different: nobody is queued to reuse it, so release
        // immediately even if the pipeline happened to complete.
        let healthy = if cancelled.load(Ordering::SeqCst) {
            false
        } else {
            match &result {
                Ok(_) => true,
                Err(_) => self.instance_is_running(&lease.instance_id).await,
            }
        };
        if healthy {
            self.remember_instance(lease.clone()).await;
            self.schedule_release(lease.instance_id.clone());
        } else {
            let _ = self.destroy_instance(&lease.instance_id).await;
            self.forget_instance(&lease.instance_id).await;
        }
        drop(permit);
        result
    }
}

impl VocalSeparator for VastVocalSeparator {
    fn is_remote(&self) -> bool {
        true
    }

    fn warm_instance_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let this = self.clone();
        Box::pin(async move { this.warm_instance_available().await })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            // The work owns an independent task: dropping this future (agent
            // deadline, client disconnect) must not interrupt renting or the
            // release that follows the pipeline. The oneshot only carries the
            // result back to whoever is still waiting.
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancel_guard = CancelOnDrop(cancelled.clone());
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(async move {
                let result = this.separate_inner(request, cancelled).await;
                let _ = sender.send(result);
            });
            let result = match receiver.await {
                Ok(result) => result,
                Err(_) => Err(VocalSeparationError::SeparationTimeout),
            };
            drop(cancel_guard);
            drop(task);
            result
        })
    }
}
