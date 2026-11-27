//! Colab-first composition with a bounded assignment window, then Vast.
//!
//! Without `VAST_API_KEY` this is a transparent pass-through to Colab. With a
//! key, a warm Vast instance is preferred directly; otherwise Colab is allowed
//! only a short window to hand over a GPU and Vast finishes the same request
//! inside the caller's original deadline. Error codes and response shapes do
//! not change.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::vast_vocal_separator::VastVocalSeparator;
use super::vast_vocal_separator_config::{
    DEFAULT_VAST_FALLBACK_AFTER_SECS, VastVocalSeparatorConfig,
};
use crate::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};
use tokio::sync::Semaphore;

pub struct FallbackVocalSeparator {
    colab: Arc<dyn VocalSeparator>,
    vast: Option<Arc<dyn VocalSeparator>>,
    /// GPU assignment window granted to Colab. The Colab adapter reserves half
    /// of its request for upload/exec/download, so it is called with 2x this.
    colab_assignment_budget: Duration,
}

impl FallbackVocalSeparator {
    pub fn new(
        colab: Arc<dyn VocalSeparator>,
        vast: Option<Arc<dyn VocalSeparator>>,
        colab_assignment_budget: Duration,
    ) -> Self {
        Self {
            colab,
            vast,
            colab_assignment_budget,
        }
    }

    /// Builds the production composition from the environment.
    ///
    /// A hard restart leaves no orphan billing: the boot task destroys every
    /// instance with this app's label before the first separation.
    pub fn from_env(
        colab: Arc<dyn VocalSeparator>,
        data_dir: PathBuf,
        gpu: Arc<Semaphore>,
    ) -> Self {
        let budget = std::env::var("SHIRLEY_VAST_FALLBACK_AFTER_SECS")
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .filter(|secs| *secs > 0)
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_VAST_FALLBACK_AFTER_SECS));
        let Some(config) = VastVocalSeparatorConfig::from_env(data_dir, gpu) else {
            return Self::new(colab, None, budget);
        };
        let vast = Arc::new(VastVocalSeparator::new(config));
        let boot = vast.clone();
        tokio::spawn(async move {
            let _ = boot.reap_orphans_on_boot().await;
        });
        // Safety net for a hard-killed process or a missed release timer.
        vast.spawn_safety_reaper(Duration::from_secs(300));
        Self::new(colab, Some(vast), budget)
    }

    pub fn has_vast(&self) -> bool {
        self.vast.is_some()
    }
}

impl VocalSeparator for FallbackVocalSeparator {
    fn is_remote(&self) -> bool {
        true
    }

    fn warm_instance_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let vast = self.vast.clone();
        Box::pin(async move {
            match vast {
                Some(vast) => vast.warm_instance_available().await,
                None => false,
            }
        })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let colab = self.colab.clone();
        let vast = self.vast.clone();
        let budget = self.colab_assignment_budget;
        Box::pin(async move {
            let Some(vast) = vast else {
                // The existing behaviour, byte for byte: no key, no Vast.
                return colab.separate(request).await;
            };

            if vast.warm_instance_available().await {
                let mut vast_request = request.clone();
                vast_request.timeout = request.timeout;
                return vast.separate(vast_request).await;
            }

            // Only Colab's ASSIGNMENT phase is shortened. A GPU that does
            // assign keeps the full original deadline for its pipeline (S4);
            // a backend without that distinction falls back to `separate`.
            let started = Instant::now();
            let colab_error = match tokio::time::timeout(
                request.timeout,
                colab.separate_with_assignment_budget(request.clone(), budget),
            )
            .await
            {
                Ok(Ok(audio)) => return Ok(audio),
                Ok(Err(error)) => error,
                Err(_) => VocalSeparationError::ColabUnavailable,
            };

            let elapsed = started.elapsed();
            let remaining = request.timeout.saturating_sub(elapsed);
            if remaining.is_zero() {
                return Err(colab_error);
            }
            // S1: a failure after Google actually assigned a GPU is not the
            // answer while the same turn can still fit another attempt. The
            // Colab adapter releases its permit before returning the error,
            // so Vast cannot rent a second GPU concurrently. Keep the original
            // deadline minus what Colab already spent.
            eprintln!(
                "[fallback] INFO colab failed after {}ms (assignment budget {}ms, reason={}); trying Vast with {}ms remaining",
                elapsed.as_millis(),
                budget.as_millis(),
                colab_error.code(),
                remaining.as_millis()
            );
            let mut vast_request = request;
            vast_request.timeout = remaining;
            vast.separate(vast_request).await
        })
    }
}
