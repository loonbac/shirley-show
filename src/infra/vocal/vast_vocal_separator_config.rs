//! Configuration for the Vast.ai separation backend.
//!
//! Vast is opt-in: without `VAST_API_KEY` the application still composes only
//! the Colab separator. The API key is deliberately redacted in `Debug` so an
//! accidental diagnostic print cannot leak it.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

/// Image used when the owner does not pin one explicitly.
///
/// Production should pin a digest/tag from `SHIRLEY_VAST_IMAGE`; `latest` can
/// change under a running deployment.
pub const DEFAULT_VAST_IMAGE: &str = "ghcr.io/loonbac/shirley-vast-worker:latest";
pub const DEFAULT_VAST_API_BASE: &str = "https://console.vast.ai/api/v0";
pub const DEFAULT_VAST_MAX_DPH: f64 = 0.6;
pub const DEFAULT_VAST_FALLBACK_AFTER_SECS: u64 = 60;
pub const VAST_INSTANCE_LABEL: &str = "shirley-show";
pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(15);
/// A rental that is still `loading` after this deadline is destroyed and the
/// next offer is tried: on the live hosts the image took >4.5 min to appear
/// once, which is longer than the whole chat turn.
pub const DEFAULT_LAUNCH_TIMEOUT: Duration = Duration::from_secs(150);
/// Preferred download speed. Popular cached base layers reach a Vast host much
/// faster, so slow hosts are only used when there are fewer than five fast
/// acceptable offers.
pub const FAST_INET_DOWN_MBPS: f64 = 1000.0;
pub const MIN_INET_DOWN_MBPS: f64 = 500.0;
pub const FAST_OFFER_MIN_COUNT: usize = 5;
/// Vast can report `running` before the container's sshd accepts connections.
/// Probe with a short connect timeout at this interval until the launch
/// deadline expires.
pub const DEFAULT_SSH_PROBE_INTERVAL: Duration = Duration::from_secs(3);
pub const DEFAULT_SSH_PROBE_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// A refused/closed/reset connection during scp or script ssh is often just
/// the sshd coming up; retry the operation once after this pause.
pub const DEFAULT_TRANSIENT_RETRY_DELAY: Duration = Duration::from_secs(3);
/// A pipeline step can fail while the turn still has time for another full
/// separation. A retry is only started with at least this much of the caller's
/// deadline left; a cold replacement normally needs about this long.
pub const DEFAULT_MIN_RETRY_REMAINING: Duration = Duration::from_secs(150);

/// How long a Vast instance stays warm after the last separation.
pub const VAST_SESSION_IDLE_RETENTION: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct VastVocalSeparatorConfig {
    pub api_key: String,
    /// Overridable for tests and private deployments; no credential lives here.
    pub api_base: String,
    pub image: String,
    /// Reject offers above this price per hour.
    pub max_dph: f64,
    pub data_dir: PathBuf,
    /// Every instance we rent carries this label, and boot cleanup uses it.
    pub label: String,
    pub ssh_bin: PathBuf,
    pub scp_bin: PathBuf,
    pub ssh_keygen_bin: PathBuf,
    pub openssl_bin: PathBuf,
    pub ffprobe_bin: PathBuf,
    pub overlap: u32,
    pub batch_size: u32,
    pub http_timeout: Duration,
    pub launch_timeout: Duration,
    pub retention: Duration,
    pub poll_interval: Duration,
    pub preflight_timeout: Duration,
    pub ssh_probe_interval: Duration,
    pub ssh_probe_connect_timeout: Duration,
    pub transient_retry_delay: Duration,
    /// Minimum time left in the turn before retrying a failed pipeline.
    pub min_retry_remaining: Duration,
    /// GPU assignment window granted to Colab before Vast takes over.
    pub fallback_after: Duration,
    pub gpu: Arc<Semaphore>,
}

impl fmt::Debug for VastVocalSeparatorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VastVocalSeparatorConfig")
            .field("api_key", &"<redacted>")
            .field("api_base", &self.api_base)
            .field("image", &self.image)
            .field("max_dph", &self.max_dph)
            .field("data_dir", &self.data_dir)
            .field("label", &self.label)
            .field("fallback_after", &self.fallback_after)
            .field("min_retry_remaining", &self.min_retry_remaining)
            .finish_non_exhaustive()
    }
}

fn env_secs(name: &str, default_secs: u64) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}

impl VastVocalSeparatorConfig {
    /// Explicit constructor, also used by the test doubles.
    pub fn new(api_key: String, data_dir: PathBuf, gpu: Arc<Semaphore>) -> Self {
        Self {
            api_key,
            api_base: DEFAULT_VAST_API_BASE.to_string(),
            image: DEFAULT_VAST_IMAGE.to_string(),
            max_dph: DEFAULT_VAST_MAX_DPH,
            data_dir,
            label: VAST_INSTANCE_LABEL.to_string(),
            ssh_bin: PathBuf::from("ssh"),
            scp_bin: PathBuf::from("scp"),
            ssh_keygen_bin: PathBuf::from("ssh-keygen"),
            openssl_bin: PathBuf::from("openssl"),
            ffprobe_bin: PathBuf::from("ffprobe"),
            overlap: 12,
            batch_size: 1,
            http_timeout: DEFAULT_HTTP_TIMEOUT,
            launch_timeout: DEFAULT_LAUNCH_TIMEOUT,
            retention: VAST_SESSION_IDLE_RETENTION,
            poll_interval: Duration::from_secs(2),
            preflight_timeout: Duration::from_secs(5),
            ssh_probe_interval: DEFAULT_SSH_PROBE_INTERVAL,
            ssh_probe_connect_timeout: DEFAULT_SSH_PROBE_CONNECT_TIMEOUT,
            transient_retry_delay: DEFAULT_TRANSIENT_RETRY_DELAY,
            min_retry_remaining: DEFAULT_MIN_RETRY_REMAINING,
            fallback_after: Duration::from_secs(DEFAULT_VAST_FALLBACK_AFTER_SECS),
            gpu,
        }
    }

    /// `None` means Vast is disabled: keep the application's Colab-only path.
    pub fn from_env(data_dir: PathBuf, gpu: Arc<Semaphore>) -> Option<Self> {
        let api_key = std::env::var("VAST_API_KEY")
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty())?;
        let mut config = Self::new(api_key, data_dir, gpu);
        if let Ok(image) = std::env::var("SHIRLEY_VAST_IMAGE") {
            let image = image.trim();
            if !image.is_empty() {
                config.image = image.to_string();
            }
        }
        if let Ok(max) = std::env::var("SHIRLEY_VAST_MAX_DPH")
            && let Ok(parsed) = max.trim().parse::<f64>()
            && parsed.is_finite()
            && parsed > 0.0
        {
            config.max_dph = parsed;
        }
        config.fallback_after = env_secs(
            "SHIRLEY_VAST_FALLBACK_AFTER_SECS",
            DEFAULT_VAST_FALLBACK_AFTER_SECS,
        );
        config.launch_timeout = env_secs(
            "SHIRLEY_VAST_LAUNCH_TIMEOUT_SECS",
            DEFAULT_LAUNCH_TIMEOUT.as_secs(),
        );
        config.min_retry_remaining = env_secs(
            "SHIRLEY_VAST_MIN_RETRY_REMAINING_SECS",
            DEFAULT_MIN_RETRY_REMAINING.as_secs(),
        );
        Some(config)
    }
}
