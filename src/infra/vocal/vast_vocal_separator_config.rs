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
pub const DEFAULT_LAUNCH_TIMEOUT: Duration = Duration::from_secs(300);

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
        Some(config)
    }
}
