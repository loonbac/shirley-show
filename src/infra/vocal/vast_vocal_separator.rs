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

/// Same convention as the Colab adapter: no new logging dependency, and no
/// credential ever appears in a line (the API key is only used in a header
/// and scrubbed from any response body we print).
macro_rules! vast_info {
    ($($arg:tt)*) => { eprintln!("[vast] INFO {}", format_args!($($arg)*)) };
}
macro_rules! vast_warn {
    ($($arg:tt)*) => { eprintln!("[vast] WARN {}", format_args!($($arg)*)) };
}

/// Flatten and redact before printing: Vast error bodies can echo input.
fn sanitized_body(text: &str, secret: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let redacted = if secret.is_empty() {
        flat
    } else {
        flat.replace(secret, "<redacted>")
    };
    let mut out: String = redacted.chars().take(300).collect();
    if redacted.chars().count() > 300 {
        out.push_str("...");
    }
    out
}

/// 24 GB-class cards that run these RoFormers economically. No Blackwell:
/// the product decision is cheapest adequate GPU, not the fastest one. The
/// canonical spellings are also sent to Vast's `gpu_name.in` filter.
const ECONOMIC_GPUS: [&str; 7] = [
    "RTX 4090",
    "RTX 3090",
    "RTX A5000",
    "RTX A6000",
    "L4",
    "A10",
    "RTX 4080S",
];

fn normalized_gpu(name: &str) -> String {
    name.trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

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
    /// Real offers report numeric RAM/disk as floats (e.g. `155.75`).
    gpu_ram: Option<f64>,
    cuda_max_good: Option<f64>,
    inet_down: Option<f64>,
    disk_space: Option<f64>,
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
        let gpu = normalized_gpu(self.gpu_name.as_deref().unwrap_or(""));
        if !ECONOMIC_GPUS
            .iter()
            .any(|allowed| normalized_gpu(allowed) == gpu)
        {
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
        // `verified` is a server-side filter: real bundles rows carry it as
        // null/absent, so only an explicit false is disqualifying here.
        self.verified != Some(false)
            && self.rentable == Some(true)
            && self.rented == Some(false)
            && self.reliability2.is_some_and(|v| v >= 0.99)
            && self.num_gpus == Some(1)
            && self.gpu_ram.is_some_and(|v| v >= 20_000.0)
            && self.cuda_max_good.is_some_and(|v| v >= 12.4)
            && self.inet_down.is_some_and(|v| v >= 500.0)
            && self.disk_space.is_some_and(|v| v >= 40.0)
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

/// Vast v1 normally returns `instances: [...]`, but older shapes and bare
/// arrays/objects still occur. Accept all of them rather than guessing one.
fn unwrap_instances(value: &Value) -> Vec<Value> {
    match value {
        Value::Object(map) => {
            if let Some(items) = map.get("instances") {
                return match items {
                    Value::Array(items) => items.clone(),
                    Value::Object(inner) => vec![Value::Object(inner.clone())],
                    _ => Vec::new(),
                };
            }
            if let Some(data) = map.get("data") {
                return unwrap_instances(data);
            }
            if map.contains_key("id") {
                return vec![value.clone()];
            }
            if !map.is_empty() && map.values().all(Value::is_object) {
                return map.values().cloned().collect();
            }
            Vec::new()
        }
        Value::Array(items) => items.clone(),
        _ => Vec::new(),
    }
}

fn instance_id(instance: &Value) -> Option<String> {
    instance.get("id").and_then(|value| {
        value
            .as_u64()
            .map(|n| n.to_string())
            .or_else(|| value.as_str().map(str::to_string))
    })
}

fn instance_state(instance: &Value) -> String {
    instance
        .get("actual_status")
        .or_else(|| instance.get("status"))
        .or_else(|| instance.get("cur_state"))
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

    /// The list endpoint moved to `/api/v1/instances/`. Keep the v0 base for
    /// every endpoint that works there (bundles, asks, single instance, ssh).
    fn v1_base(&self) -> String {
        let base = self.config.api_base.trim_end_matches('/');
        if let Some(prefix) = base.strip_suffix("/api/v0") {
            format!("{prefix}/api/v1")
        } else if let Some(prefix) = base.strip_suffix("/v0") {
            format!("{prefix}/v1")
        } else {
            format!("{base}/v1")
        }
    }

    /// Errors never carry the key: only the path, status and a scrubbed,
    /// truncated response body are logged.
    async fn api_at(
        &self,
        base: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
        query: &[(&str, &str)],
    ) -> Result<Value, VocalSeparationError> {
        let method_name = method.as_str().to_string();
        let url = format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut request = self
            .client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", self.config.api_key));
        if !query.is_empty() {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = match tokio::time::timeout(self.config.http_timeout, request.send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                vast_warn!("{method_name} {path} request failed: {error}");
                return Err(VocalSeparationError::ColabUnavailable);
            }
            Err(_) => {
                vast_warn!("{method_name} {path} timed out");
                return Err(VocalSeparationError::ColabUnavailable);
            }
        };
        let status = response.status();
        let text = match tokio::time::timeout(self.config.http_timeout, response.text()).await {
            Ok(Ok(text)) => text,
            _ => {
                vast_warn!("{method_name} {path} body timed out (status {})", status.as_u16());
                return Err(VocalSeparationError::ColabUnavailable);
            }
        };
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            vast_warn!(
                "{method_name} {path} rejected credentials: status {} body {}",
                status.as_u16(),
                sanitized_body(&text, &self.config.api_key)
            );
            return Err(VocalSeparationError::ColabUnavailable);
        }
        if !status.is_success() {
            vast_warn!(
                "{method_name} {path} failed: status {} body {}",
                status.as_u16(),
                sanitized_body(&text, &self.config.api_key)
            );
            return Err(VocalSeparationError::ColabUnavailable);
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(_) => {
                vast_warn!(
                    "{method_name} {path} returned invalid JSON: status {} body {}",
                    status.as_u16(),
                    sanitized_body(&text, &self.config.api_key)
                );
                Err(VocalSeparationError::ColabUnavailable)
            }
        }
    }

    async fn api(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, VocalSeparationError> {
        self.api_at(&self.config.api_base, method, path, body, &[])
            .await
    }

    /// Paginated v1 listing, narrowed with the documented `select_filters`
    /// label filter. The response shape is parsed defensively by `unwrap_instances`.
    async fn list_instances(&self) -> Result<Vec<Value>, VocalSeparationError> {
        let filter = json!({ "label": { "eq": self.config.label } }).to_string();
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..8 {
            let mut query = vec![
                ("limit", "25"),
                ("select_filters", filter.as_str()),
            ];
            if let Some(token) = cursor.as_deref() {
                query.push(("after_token", token));
            }
            let listing = self
                .api_at(&self.v1_base(), Method::GET, "instances/", None, &query)
                .await?;
            let next = listing
                .get("next_token")
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
                .map(str::to_string);
            all.extend(unwrap_instances(&listing));
            cursor = next;
            if cursor.is_none() {
                break;
            }
        }
        vast_info!("v1 instance listing: found={}", all.len());
        Ok(all)
    }

    async fn destroy_instance(&self, instance_id: &str) -> bool {
        let path = format!("instances/{instance_id}/");
        let ok = matches!(self.api(Method::DELETE, &path, None).await, Ok(_));
        if ok {
            vast_info!("destroy instance {instance_id}: ok");
        } else {
            vast_warn!("destroy instance {instance_id}: failed");
        }
        ok
    }

    /// Releases every instance this app left behind. Called on application boot
    /// so a hard restart cannot leave an orphan billing indefinitely.
    pub async fn destroy_orphans(&self) -> usize {
        let Ok(listing) = self.list_instances().await else {
            return 0;
        };
        let mut removed = 0;
        vast_info!("orphan sweep at boot: candidates={}", listing.len());
        for instance in listing {
            let label = instance.get("label").and_then(Value::as_str).unwrap_or("");
            let id = instance_id(&instance);
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
                Ok(Ok(status)) if status.success() => vast_info!("ssh key generated"),
                _ => {
                    vast_warn!("ssh key generation failed");
                    return Err(VocalSeparationError::SeparationFailed);
                }
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
        let already_registered = registered.to_string().contains(&key);
        if !already_registered {
            self.api(Method::POST, "ssh/", Some(json!({ "ssh_key": key })))
                .await?;
        }
        vast_info!("ssh key ready: already_registered={already_registered}");
        Ok((private, key))
    }

    /// Vast validates every filter as an operator object; scalars return 400.
    /// The GPU whitelist is sent server-side, while CN/HK stays a client-side
    /// check on `geolocation`, so a full page can reach non-CN offers.
    fn bundle_query(&self) -> Value {
        json!({
            "verified": {"eq": true},
            "rentable": {"eq": true},
            "rented": {"eq": false},
            "reliability2": {"gte": 0.99},
            "num_gpus": {"eq": 1},
            "gpu_ram": {"gte": 20_000},
            "cuda_max_good": {"gte": 12.4},
            "inet_down": {"gte": 500},
            "disk_space": {"gte": 40},
            "gpu_name": {"in": ECONOMIC_GPUS},
            "dph_total": {"lte": self.config.max_dph},
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
        let received = values.len();
        let mut offers: Vec<Offer> = values
            .into_iter()
            .filter_map(|value| serde_json::from_value(value).ok())
            .filter(|offer: &Offer| offer.acceptable(self.config.max_dph))
            .collect();
        vast_info!(
            "offers found: received={received} acceptable={} (CN/HK excluded client-side)",
            offers.len()
        );
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

    async fn rent_offer(
        &self,
        offer: &Offer,
        deadline: Instant,
    ) -> Result<Lease, VocalSeparationError> {
        let id = offer
            .id()
            .ok_or(VocalSeparationError::ColabUnavailable)?;
        vast_info!(
            "chosen offer: id={id} gpu={} dph={} geolocation={}",
            offer.gpu_name.as_deref().unwrap_or("?"),
            offer.dph_total.unwrap_or(f64::NAN),
            offer.geolocation.as_deref().unwrap_or("?")
        );
        let response = self
            .api(
                Method::PUT,
                &format!("asks/{id}/"),
                Some(json!({
                    "client_id": "me",
                    "image": self.config.image,
                    "disk": 40,
                    "runtype": "ssh_direc ssh_proxy",
                    "label": self.config.label,
                })),
            )
            .await?;
        let instance_id = instance_id(&response)
            .or_else(|| {
                response
                    .get("new_contract")
                    .or_else(|| response.get("instance_id"))
                    .and_then(|value| {
                        value
                            .as_u64()
                            .map(|n| n.to_string())
                            .or_else(|| value.as_str().map(str::to_string))
                    })
            })
            .ok_or(VocalSeparationError::ColabUnavailable)?;
        vast_info!("rent result: instance={instance_id}");

        let mut previous_state = String::new();
        loop {
            if Instant::now() >= deadline {
                vast_warn!("instance {instance_id} launch deadline expired");
                break;
            }
            if let Some(status) = self.instance_status(&instance_id).await {
                let state = instance_state(&status);
                if state != previous_state {
                    vast_info!("instance {instance_id} state: {state}");
                    previous_state = state.clone();
                }
                if state == "running" {
                    if let Some((host, port)) = connection_details(&status) {
                        vast_info!("instance {instance_id} ssh ready at {host}:{port}");
                        return Ok(Lease {
                            instance_id,
                            host,
                            port,
                            last_used: Instant::now(),
                        });
                    }
                    vast_warn!("instance {instance_id} running without ssh details");
                }
                if is_dead_state(&state) {
                    vast_warn!("instance {instance_id} entered dead state {state}");
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
                Err(_) => {
                    vast_warn!("offer {} could not be rented; trying next", offer.id().unwrap_or_default());
                    continue; // try the next offer
                }
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
                vast_info!("retention expired for instance {instance_id}: destroying");
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
                let Ok(listing) = this.list_instances().await else {
                    continue;
                };
                let current = this.lease.lock().await.clone();
                vast_info!("safety reaper: candidates={}", listing.len());
                for instance in listing {
                    let label = instance.get("label").and_then(Value::as_str).unwrap_or("");
                    if label != this.config.label {
                        continue;
                    }
                    let Some(id) = instance_id(&instance) else {
                        continue;
                    };
                    let still_warm = current.as_ref().is_some_and(|lease| {
                        lease.instance_id == id && lease.last_used.elapsed() < this.config.retention
                    });
                    if still_warm {
                        vast_info!("safety reaper: keeping warm instance {id}");
                        continue;
                    }
                    vast_info!("safety reaper: stale instance {id}, destroying");
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
        let upload_started = Instant::now();
        self.scp(
            lease,
            private_key,
            source,
            &format!("root@{}:{REMOTE_INPUT_PATH}", lease.host),
            left(),
        )
        .await?;
        vast_info!("upload completed in {}ms", upload_started.elapsed().as_millis());

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
        let script_started = Instant::now();
        let output = self
            .ssh_command(
                lease,
                private_key,
                "bash -lc 'cd /content && python -'",
                Some(script_file),
                left(),
            )
            .await?;
        vast_info!(
            "remote script completed in {}ms",
            script_started.elapsed().as_millis()
        );
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
        let download_started = Instant::now();
        self.scp(
            lease,
            private_key,
            &format!("root@{}:{REMOTE_OUTPUT_PATH}", lease.host),
            destination,
            left(),
        )
        .await?;
        vast_info!(
            "download completed in {}ms",
            download_started.elapsed().as_millis()
        );

        // 3. Same verification as Colab: SHA-256 and ffprobe duration against
        //    the single JSON line printed by the shared script.
        let local_hash = self.sha256(&request.output_path, left()).await?;
        let hash_matches = local_hash == result.sha256.to_lowercase();
        vast_info!("verification sha256_match={hash_matches}");
        if !hash_matches {
            vast_warn!("verification failed: sha256 mismatch");
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }
        let local_duration = self
            .run_ffprobe_duration(&request.output_path, left())
            .await?;
        let duration_delta = (local_duration - result.duration).abs();
        vast_info!(
            "verification duration: reported={} local={} delta={duration_delta}",
            result.duration,
            local_duration
        );
        if duration_delta > 2.0 {
            vast_warn!("verification failed: duration delta > 2s");
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
            vast_info!("reusing warm instance {}", lease.instance_id);
            return true;
        }
        vast_warn!("warm instance {} is not running; destroying", lease.instance_id);
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
        vast_info!("separation started: timeout={}s", request.timeout.as_secs());
        let (private_key, _) = self.ensure_ssh_key().await?;
        // S3/S4: a GPU already rented for this app is reused instead of
        // paying a cold start for the next separation.
        let lease = if self.warm_instance_available().await {
            match self.lease.lock().await.clone() {
                Some(lease) => {
                    vast_info!("using already rented instance {}", lease.instance_id);
                    lease
                }
                None => self.acquire_instance(deadline).await?,
            }
        } else {
            self.acquire_instance(deadline).await?
        };

        // A rental that lands after the deadline or after the caller went away
        // has no consumer; destroy it here instead of caching a billing leak.
        if cancelled.load(Ordering::SeqCst) || Instant::now() >= deadline {
            vast_warn!(
                "instance {} has no consumer (cancelled={} deadline_passed={}); destroying",
                lease.instance_id,
                cancelled.load(Ordering::SeqCst),
                Instant::now() >= deadline
            );
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
            vast_info!(
                "keeping instance {} warm for {}s",
                lease.instance_id,
                self.config.retention.as_secs()
            );
            self.remember_instance(lease.clone()).await;
            self.schedule_release(lease.instance_id.clone());
        } else {
            vast_warn!(
                "releasing instance {} after failed/cancelled separation",
                lease.instance_id
            );
            let _ = self.destroy_instance(&lease.instance_id).await;
            self.forget_instance(&lease.instance_id).await;
        }
        vast_info!("separation finished: ok={}", result.is_ok());
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
