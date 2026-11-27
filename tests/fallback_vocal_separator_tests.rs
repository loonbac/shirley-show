//! T2/T3: fallback composition and the honest "no GPU" message.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tokio::sync::Semaphore;

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::fallback_vocal_separator::FallbackVocalSeparator;
use shirley_show::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

#[derive(Clone)]
struct RecordingSeparator {
    name: &'static str,
    warm: bool,
    fail: bool,
    delay: Duration,
    calls: Arc<Mutex<Vec<Call>>>,
}

#[derive(Clone, Copy)]
struct Call {
    name: &'static str,
    timeout: Duration,
    budget: Option<Duration>,
    at: Instant,
}

impl RecordingSeparator {
    fn new(name: &'static str, calls: Arc<Mutex<Vec<Call>>>) -> Self {
        Self {
            name,
            warm: false,
            fail: false,
            delay: Duration::ZERO,
            calls,
        }
    }

    fn failing(mut self) -> Self {
        self.fail = true;
        self
    }

    fn warm(mut self) -> Self {
        self.warm = true;
        self
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

impl RecordingSeparator {
    fn execute(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let fail = self.fail;
        let delay = self.delay;
        Box::pin(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            if fail {
                return Err(VocalSeparationError::ColabUnavailable);
            }
            tokio::fs::write(&request.output_path, b"separated").await.unwrap();
            Ok(SeparatedAudio {
                duration_secs: 12,
                size_bytes: 9,
            })
        })
    }
}

impl VocalSeparator for RecordingSeparator {
    fn warm_instance_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let warm = self.warm;
        Box::pin(async move { warm })
    }

    fn separate_with_assignment_budget(
        &self,
        request: VocalSeparationRequest,
        budget: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        self.calls.lock().unwrap().push(Call {
            name: self.name,
            timeout: request.timeout,
            budget: Some(budget),
            at: Instant::now(),
        });
        self.execute(request)
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        self.calls.lock().unwrap().push(Call {
            name: self.name,
            timeout: request.timeout,
            budget: None,
            at: Instant::now(),
        });
        self.execute(request)
    }
}

fn request(timeout: Duration) -> VocalSeparationRequest {
    VocalSeparationRequest {
        source_path: std::path::PathBuf::from("/tmp/source.mp3"),
        output_path: std::path::PathBuf::from("/tmp/out.mp3"),
        mode: VocalSeparationMode::Coros,
        timeout,
    }
}

#[tokio::test]
async fn colab_gets_only_its_assignment_budget_then_vast_finishes_the_request() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let colab = RecordingSeparator::new("colab", calls.clone())
        .failing()
        .with_delay(Duration::from_millis(120));
    let vast = RecordingSeparator::new("vast", calls.clone());
    // Colab's own adapter reserves half of its request for the pipeline, so
    // 2x the configured budget leaves ~60s for GPU assignment by default.
    let composed = FallbackVocalSeparator::new(
        Arc::new(colab),
        Some(Arc::new(vast)),
        Duration::from_millis(50),
    );
    let started = Instant::now();
    let result = composed
        .separate(request(Duration::from_secs(10)))
        .await
        .expect("Vast completes the separation");
    assert_eq!(result.duration_secs, 12);

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "colab");
    assert_eq!(calls[0].budget, Some(Duration::from_millis(50)));
    assert_eq!(calls[0].timeout, Duration::from_secs(10));
    assert_eq!(calls[1].name, "vast");
    assert!(
        calls[1].at.duration_since(started) >= Duration::from_millis(95),
        "Vast must start only after the Colab budget elapsed"
    );
    assert!(calls[1].timeout <= Duration::from_secs(10));
    assert!(calls[1].timeout > Duration::from_secs(9));
}

#[tokio::test]
async fn assigned_colab_gpu_keeps_its_full_pipeline_deadline() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    // Colab assigns after the short budget but still has time to finish: S4
    // says a GPU Google actually gave us must not be thrown away.
    let colab = RecordingSeparator::new("colab", calls.clone())
        .with_delay(Duration::from_millis(120));
    let vast = RecordingSeparator::new("vast", calls.clone());
    let composed = FallbackVocalSeparator::new(
        Arc::new(colab),
        Some(Arc::new(vast)),
        Duration::from_millis(50),
    );
    let result = composed
        .separate(request(Duration::from_secs(10)))
        .await
        .expect("Colab pipeline finishes");
    assert_eq!(result.duration_secs, 12);
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "Vast must not be used after Colab assigns");
    assert_eq!(calls[0].name, "colab");
    assert_eq!(calls[0].budget, Some(Duration::from_millis(50)));
    assert_eq!(calls[0].timeout, Duration::from_secs(10));
}

#[tokio::test]
async fn warm_vast_instance_is_preferred_over_colab() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let colab = RecordingSeparator::new("colab", calls.clone()).failing();
    let vast = RecordingSeparator::new("vast", calls.clone()).warm();
    let composed =
        FallbackVocalSeparator::new(Arc::new(colab), Some(Arc::new(vast)), Duration::from_millis(50));
    composed
        .separate(request(Duration::from_secs(30)))
        .await
        .expect("warm Vast separates");
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "vast");
    assert_eq!(calls[0].timeout, Duration::from_secs(30));
}

#[tokio::test]
async fn no_vast_key_keeps_the_existing_colab_only_behaviour() {
    unsafe { std::env::remove_var("VAST_API_KEY") };
    let tmp = TempDir::new().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let colab = RecordingSeparator::new("colab", calls.clone());
    let composed = FallbackVocalSeparator::from_env(
        Arc::new(colab),
        tmp.path().join("data"),
        Arc::new(Semaphore::new(1)),
    );
    assert!(!composed.has_vast());
    composed
        .separate(request(Duration::from_secs(123)))
        .await
        .expect("Colab still succeeds");
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].timeout, Duration::from_secs(123));
    unsafe { std::env::remove_var("VAST_API_KEY") };
}

#[tokio::test]
async fn both_providers_failing_surfaces_the_colab_unavailable_code() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let colab = RecordingSeparator::new("colab", calls.clone()).failing();
    let vast = RecordingSeparator::new("vast", calls.clone()).failing();
    let composed =
        FallbackVocalSeparator::new(Arc::new(colab), Some(Arc::new(vast)), Duration::ZERO);
    let err = composed
        .separate(request(Duration::from_secs(10)))
        .await
        .unwrap_err();
    assert_eq!(err, VocalSeparationError::ColabUnavailable);
    assert_eq!(err.code(), "colab_unavailable");
}

#[test]
fn no_gpu_message_is_honest_and_keeps_the_error_code() {
    let err = VocalSeparationError::ColabUnavailable;
    const EXPECTED: &str =
        "No hay GPU disponible ahora mismo en los proveedores; inténtalo de nuevo en unos minutos.";
    assert_eq!(err.code(), "colab_unavailable");
    assert_eq!(err.message(), EXPECTED);
    assert_eq!(err.to_string(), EXPECTED);
    assert!(!err.message().to_lowercase().contains("cuota"));
    assert!(!err.message().to_lowercase().contains("agot"));
}
