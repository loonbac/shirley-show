//! T2: Vast.ai backend behind the existing VocalSeparator port.
//!
//! The HTTP API and ssh/scp are replaced by local fakes: no Vast account, no
//! network egress and no SSH connection are needed to verify the contract.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::fallback_vocal_separator::FallbackVocalSeparator;
use shirley_show::infra::vocal::vast_vocal_separator::VastVocalSeparator;
use shirley_show::infra::vocal::vast_vocal_separator_config::{
    DEFAULT_VAST_FALLBACK_AFTER_SECS, DEFAULT_VAST_IMAGE, DEFAULT_VAST_MAX_DPH,
    VastVocalSeparatorConfig,
};
use shirley_show::ports::vocal_separator::{VocalSeparationRequest, VocalSeparator};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

fn set_env(key: &str, value: &str) {
    unsafe { std::env::set_var(key, value) }
}

fn remove_env(key: &str) {
    unsafe { std::env::remove_var(key) }
}

#[derive(Default)]
struct Calls {
    bundles: Vec<Value>,
    rents: Vec<(String, Value)>,
    deletes: Vec<String>,
    ssh_posts: Vec<Value>,
    instance_gets: usize,
}

#[derive(Clone)]
struct FakeState {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    offers: Vec<Value>,
    instances: Vec<Value>,
    registered_keys: Vec<String>,
    rent_failures: usize,
    die_on_next_health: bool,
    /// Real Vast instances may expose only proxy SSH fields.
    proxy_only: bool,
    /// Accept the older unwrapped response shape as well.
    unwrapped: bool,
    calls: Calls,
}

impl FakeState {
    fn new(offers: Vec<Value>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                offers,
                instances: Vec::new(),
                registered_keys: Vec::new(),
                rent_failures: 0,
                die_on_next_health: false,
                proxy_only: false,
                unwrapped: false,
                calls: Calls::default(),
            })),
        }
    }
}

async fn bundles(State(state): State<FakeState>, Json(body): Json<Value>) -> Json<Value> {
    state.inner.lock().unwrap().calls.bundles.push(body);
    let offers = state.inner.lock().unwrap().offers.clone();
    Json(json!({ "offers": offers }))
}

async fn rent(
    AxumPath(id): AxumPath<String>,
    State(state): State<FakeState>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    let mut inner = state.inner.lock().unwrap();
    inner.calls.rents.push((id, body));
    if inner.rent_failures > 0 {
        inner.rent_failures -= 1;
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "busy"}))).into_response();
    }
    // The live probe reads `.new_contract`; keep legacy keys for compatibility.
    Json(json!({ "new_contract": 7001 })).into_response()
}

async fn get_instance(AxumPath(_id): AxumPath<String>, State(state): State<FakeState>) -> Json<Value> {
    let mut inner = state.inner.lock().unwrap();
    inner.calls.instance_gets += 1;
    let dead = inner.die_on_next_health && inner.calls.instance_gets >= 2;
    if dead {
        inner.die_on_next_health = false;
    }
    let state = if dead { "exited" } else { "running" };
    if inner.unwrapped {
        return Json(json!({
            "id": 7001,
            "status": state,
            "ssh_host": "ssh7.vast.ai",
            "ssh_port": 3333
        }));
    }
    let (ip, ports) = if inner.proxy_only {
        ("   ", json!({}))
    } else {
        // Whitespace and a string HostPort are exactly what the live probe saw.
        (" 127.0.0.1 ", json!({ "22/tcp": [{ "HostPort": "2222" }] }))
    };
    Json(json!({
        "instances": {
            "id": 7001,
            "actual_status": state,
            "public_ipaddr": ip,
            "ports": ports,
            "ssh_host": "ssh7.vast.ai",
            "ssh_port": 3333
        }
    }))
}

async fn delete_instance(
    AxumPath(id): AxumPath<String>,
    State(state): State<FakeState>,
) -> Json<Value> {
    state.inner.lock().unwrap().calls.deletes.push(id.clone());
    Json(json!({ "deleted": id }))
}

async fn list_instances(State(state): State<FakeState>) -> Json<Value> {
    let instances = state.inner.lock().unwrap().instances.clone();
    Json(json!({ "instances": instances }))
}

async fn get_ssh_keys(State(state): State<FakeState>) -> Json<Value> {
    let keys = state.inner.lock().unwrap().registered_keys.clone();
    Json(json!({ "ssh_keys": keys }))
}

async fn post_ssh_key(State(state): State<FakeState>, Json(body): Json<Value>) -> Json<Value> {
    let mut inner = state.inner.lock().unwrap();
    inner.calls.ssh_posts.push(body.clone());
    if let Some(key) = body.get("ssh_key").and_then(Value::as_str) {
        inner.registered_keys.push(key.to_string());
    }
    Json(json!({ "success": true }))
}

async fn spawn_fake_vast(state: FakeState) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/api/v0/bundles/", post(bundles))
        .route("/api/v0/asks/{id}/", put(rent))
        .route("/api/v0/instances/{id}/", get(get_instance).delete(delete_instance))
        .route("/api/v0/instances/", get(list_instances))
        .route("/api/v0/ssh/", get(get_ssh_keys).post(post_ssh_key))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/api/v0", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (base, task)
}

fn offer(id: u64, price: f64, gpu: &str, geo: &str) -> Value {
    json!({
        "id": id,
        "dph_total": price,
        "gpu_name": gpu,
        "geolocation": geo,
        "verified": true,
        "rentable": true,
        "rented": false,
        "reliability2": 0.99,
        "num_gpus": 1,
        "gpu_ram": 24564,
        "cuda_max_good": 12.4,
        "inet_down": 900,
        "disk_space": 80,
        "rent_type": "on-demand"
    })
}

fn sha256_file(path: &Path) -> String {
    let output = Command::new("openssl")
        .args(["dgst", "-sha256", "-r"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

/// Writes fake `ssh`/`scp` binaries. Every child invocation is logged; `scp`
/// copies through a local directory that stands in for the worker filesystem.
fn write_fake_binaries(
    dir: &Path,
    log: &Path,
    result_json: &Path,
    output_mp3: &Path,
    fail_python: bool,
) -> (PathBuf, PathBuf, PathBuf) {
    let remote = dir.join("remote");
    std::fs::create_dir_all(&remote).unwrap();
    std::fs::copy(output_mp3, remote.join("output.mp3")).unwrap();
    let ssh = dir.join("ssh");
    let scp = dir.join("scp");
    let ffprobe = dir.join("ffprobe");
    let exit_line = if fail_python { "exit 1" } else { "exit 0" };
    std::fs::write(
        &ssh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in\n  *python*) cat > '{}'; cat '{}'; {} ;;\n  *rm*) exit 0 ;;\n  *) exit 0 ;;\nesac\n",
            log.display(),
            remote.join("script.py").display(),
            result_json.display(),
            exit_line
        ),
    )
    .unwrap();
    std::fs::write(
        &scp,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\neval \"src=\\${{$(( $# - 1 ))}}\"\neval \"dst=\\${{$#}}\"\ncase \"$src\" in\n  root@*) cp '{}' \"$dst\" ;;\n  *) cp \"$src\" '{}' ;;\nesac\n",
            log.display(),
            remote.join("output.mp3").display(),
            remote.join("input_audio").display()
        ),
    )
    .unwrap();
    std::fs::write(
        &ffprobe,
        "#!/bin/sh\necho '{\"streams\":[{\"codec_type\":\"audio\",\"codec_name\":\"mp3\",\"duration\":\"12.0\"}],\"format\":{\"duration\":\"12.0\"}}'\n",
    )
    .unwrap();
    for path in [&ssh, &scp, &ffprobe] {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    (ssh, scp, ffprobe)
}

struct Fixture {
    _tmp: TempDir,
    config: VastVocalSeparatorConfig,
    state: FakeState,
    _server: tokio::task::JoinHandle<()>,
    ssh_log: PathBuf,
    key_path: PathBuf,
    output_bytes: PathBuf,
}

impl Fixture {
    async fn new(offers: Vec<Value>) -> Self {
        let tmp = TempDir::new().unwrap();
        let data = tmp.path().join("data");
        let key_dir = data.join("vast/ssh");
        std::fs::create_dir_all(&key_dir).unwrap();
        let key_path = key_dir.join("id_ed25519");
        std::fs::write(&key_path, "TEST PRIVATE KEY").unwrap();
        std::fs::write(
            key_dir.join("id_ed25519.pub"),
            "ssh-ed25519 AAAATESTKEY shirley-test\n",
        )
        .unwrap();

        let output_bytes = tmp.path().join("expected.mp3");
        std::fs::write(&output_bytes, b"ID3\x03fake-separated-audio").unwrap();
        let hash = sha256_file(&output_bytes);
        let result_json = tmp.path().join("result.json");
        std::fs::write(
            &result_json,
            json!({"url": "", "sha256": hash, "duration": 12.0, "size": std::fs::metadata(&output_bytes).unwrap().len()}).to_string(),
        )
        .unwrap();
        let ssh_log = tmp.path().join("ssh.log");
        let binaries_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&binaries_dir).unwrap();
        let (ssh, scp, ffprobe) = write_fake_binaries(
            &binaries_dir,
            &ssh_log,
            &result_json,
            &output_bytes,
            false,
        );

        let state = FakeState::new(offers);
        let (base, server) = spawn_fake_vast(state.clone()).await;
        let mut config =
            VastVocalSeparatorConfig::new("unit-test-key".into(), data.clone(), Arc::new(Semaphore::new(1)));
        config.api_base = base;
        config.image = "ghcr.io/loonbac/shirley-vast-worker:sha-test".into();
        config.ssh_bin = ssh;
        config.scp_bin = scp;
        config.ffprobe_bin = ffprobe;
        config.retention = Duration::from_millis(250);
        config.poll_interval = Duration::from_millis(5);
        config.http_timeout = Duration::from_secs(5);
        config.launch_timeout = Duration::from_secs(5);
        Self {
            _tmp: tmp,
            config,
            state,
            _server: server,
            ssh_log,
            key_path,
            output_bytes,
        }
    }

    fn separator(&self) -> VastVocalSeparator {
        VastVocalSeparator::new(self.config.clone())
    }

    fn request(&self, timeout: Duration) -> VocalSeparationRequest {
        let source = self._tmp.path().join("source.mp3");
        if !source.exists() {
            std::fs::write(&source, b"ID3\x03source-audio").unwrap();
        }
        VocalSeparationRequest {
            source_path: source,
            output_path: self._tmp.path().join("output.mp3"),
            mode: VocalSeparationMode::Coros,
            timeout,
        }
    }

    fn calls(&self) -> Calls {
        let inner = self.state.inner.lock().unwrap();
        Calls {
            bundles: inner.calls.bundles.clone(),
            rents: inner.calls.rents.clone(),
            deletes: inner.calls.deletes.clone(),
            ssh_posts: inner.calls.ssh_posts.clone(),
            instance_gets: inner.calls.instance_gets,
        }
    }
}

#[tokio::test]
async fn vast_rents_cheapest_allowed_offer_and_separates_with_generated_script() {
    let fixture = Fixture::new(vec![
        offer(1, 0.10, "RTX A100", "US"),
        offer(2, 0.05, "RTX 4090", "CN"),
        offer(3, 0.90, "RTX 3090", "US"),
        offer(4, 0.30, "RTX A5000", "PL"),
        offer(5, 0.20, "RTX 3090", "US"),
    ])
    .await;
    let separator = fixture.separator();
    let result = separator
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("separation succeeds");

    assert_eq!(result.duration_secs, 12);
    assert_eq!(
        result.size_bytes,
        std::fs::metadata(&fixture.output_bytes).unwrap().len()
    );
    assert_eq!(
        std::fs::read(fixture.request(Duration::from_secs(1)).output_path).unwrap(),
        std::fs::read(&fixture.output_bytes).unwrap()
    );

    let calls = fixture.calls();
    let query = &calls.bundles[0];
    assert_eq!(query["verified"], true);
    assert_eq!(query["rentable"], true);
    assert_eq!(query["rented"], false);
    assert_eq!(query["reliability2"], 0.99);
    assert_eq!(query["num_gpus"], 1);
    assert_eq!(query["gpu_ram"], 20000);
    assert_eq!(query["cuda_max_good"], 12.4);
    assert_eq!(query["inet_down"], 500);
    assert_eq!(query["disk_space"], 40);
    assert_eq!(query["type"], "on-demand");
    assert_eq!(calls.rents.len(), 1);
    assert_eq!(calls.rents[0].0, "5");
    assert_eq!(calls.rents[0].1["image"], fixture.config.image);
    assert_eq!(calls.rents[0].1["disk"], 40);
    assert_eq!(calls.rents[0].1["runtype"], "ssh");
    assert_eq!(calls.rents[0].1["label"], "shirley-show");
    assert_eq!(calls.ssh_posts.len(), 1);
    assert_eq!(
        calls.ssh_posts[0]["ssh_key"],
        "ssh-ed25519 AAAATESTKEY shirley-test"
    );

    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    assert!(log.contains("python"), "script must run through ssh: {log}");
    assert!(log.contains("-p 2222"), "ssh must use the instance port: {log}");
    let script = std::fs::read_to_string(fixture._tmp.path().join("bin/remote/script.py")).unwrap();
    assert!(script.contains("audio_separator"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&fixture.key_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[tokio::test]
async fn warm_instance_is_reused_and_destroyed_after_idle_window() {
    let fixture = Fixture::new(vec![offer(9, 0.21, "RTX 3090", "PL")]).await;
    let separator = fixture.separator();
    separator
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("first separation");
    assert!(separator.warm_instance_available().await);
    assert_eq!(fixture.calls().rents.len(), 1);

    separator
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("second separation reuses the warm instance");
    assert_eq!(
        fixture.calls().rents.len(),
        1,
        "a warm GPU must not be rented twice"
    );
    assert_eq!(
        fixture.calls().ssh_posts.len(),
        1,
        "a registered key must not be posted twice"
    );
    assert!(fixture.calls().deletes.is_empty());

    let deadline = Instant::now() + Duration::from_secs(3);
    while fixture.calls().deletes.is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fixture.calls().deletes,
        vec!["7001".to_string()],
        "idle GPU must be destroyed once"
    );
    assert!(!separator.warm_instance_available().await);
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(fixture.calls().deletes.len(), 1, "release must run once");
}

#[tokio::test]
async fn proven_dead_instance_is_destroyed_immediately_and_rented_again() {
    let mut fixture = Fixture::new(vec![offer(11, 0.23, "RTX 4090", "US")]).await;
    fixture.state.inner.lock().unwrap().die_on_next_health = true;
    {
        let dir = fixture._tmp.path().join("bin_fail");
        std::fs::create_dir_all(&dir).unwrap();
        let result_json = dir.join("result.json");
        std::fs::write(&result_json, json!({"url":"","sha256":"x","duration":1.0,"size":1}).to_string()).unwrap();
        let output = dir.join("output.mp3");
        std::fs::write(&output, b"x").unwrap();
        let (ssh, scp, ffprobe) = write_fake_binaries(&dir, &fixture.ssh_log, &result_json, &output, true);
        fixture.config.ssh_bin = ssh;
        fixture.config.scp_bin = scp;
        fixture.config.ffprobe_bin = ffprobe;
    }
    let separator = fixture.separator();
    assert!(
        separator
            .separate(fixture.request(Duration::from_secs(30)))
            .await
            .is_err(),
        "the failing remote script must fail the separation"
    );
    assert_eq!(
        fixture.calls().deletes,
        vec!["7001".to_string()],
        "a proven dead instance must not keep billing"
    );
    assert!(!separator.warm_instance_available().await);

    // Restore healthy fakes: a fresh instance can still complete a separation.
    let dir = fixture._tmp.path().join("bin");
    let (ssh, scp, ffprobe) = write_fake_binaries(
        &dir,
        &fixture.ssh_log,
        &fixture._tmp.path().join("result.json"),
        &fixture.output_bytes,
        false,
    );
    fixture.config.ssh_bin = ssh;
    fixture.config.scp_bin = scp;
    fixture.config.ffprobe_bin = ffprobe;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("new instance separates");
    assert_eq!(fixture.calls().rents.len(), 2);
}

#[tokio::test]
async fn failed_rent_tries_the_next_offer() {
    let fixture = Fixture::new(vec![
        offer(21, 0.18, "RTX 3090", "US"),
        offer(22, 0.22, "L4", "PL"),
    ])
    .await;
    fixture.state.inner.lock().unwrap().rent_failures = 1;
    let separator = fixture.separator();
    separator
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("second offer succeeds");
    let calls = fixture.calls();
    assert_eq!(
        calls.rents.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["21", "22"]
    );
    assert!(calls.deletes.is_empty());
}

#[tokio::test]
async fn boot_cleans_only_instances_labelled_for_this_app() {
    let fixture = Fixture::new(vec![]).await;
    {
        let mut inner = fixture.state.inner.lock().unwrap();
        inner.instances = vec![
            json!({"id": 101, "label": "shirley-show", "status": "running"}),
            json!({"id": 102, "label": "otro-proyecto", "status": "running"}),
        ];
    }
    let removed = fixture.separator().destroy_orphans().await;
    assert_eq!(removed, 1);
    assert_eq!(fixture.calls().deletes, vec!["101".to_string()]);
}

#[test]
fn vast_config_env_defaults_and_key_is_never_in_debug_output() {
    let _lock = ENV_MUTEX.lock().unwrap();
    for key in [
        "SHIRLEY_VAST_IMAGE",
        "SHIRLEY_VAST_MAX_DPH",
        "SHIRLEY_VAST_FALLBACK_AFTER_SECS",
    ] {
        remove_env(key);
    }
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = VastVocalSeparatorConfig::new("super-secret-key".into(), PathBuf::from("/tmp/data"), gpu.clone());
    assert_eq!(cfg.image, DEFAULT_VAST_IMAGE);
    assert_eq!(cfg.max_dph, DEFAULT_VAST_MAX_DPH);
    assert_eq!(cfg.label, "shirley-show");
    assert_eq!(cfg.fallback_after, Duration::from_secs(DEFAULT_VAST_FALLBACK_AFTER_SECS));
    assert!(!format!("{cfg:?}").contains("super-secret-key"));

    set_env("VAST_API_KEY", "unit-test-dummy");
    set_env("SHIRLEY_VAST_IMAGE", "ghcr.io/loonbac/shirley-vast-worker:sha-123");
    set_env("SHIRLEY_VAST_MAX_DPH", "0.35");
    set_env("SHIRLEY_VAST_FALLBACK_AFTER_SECS", "45");
    let from_env = VastVocalSeparatorConfig::from_env(PathBuf::from("/tmp/data"), gpu).unwrap();
    assert_eq!(from_env.image, "ghcr.io/loonbac/shirley-vast-worker:sha-123");
    assert_eq!(from_env.max_dph, 0.35);
    assert_eq!(from_env.fallback_after, Duration::from_secs(45));
    for key in [
        "VAST_API_KEY",
        "SHIRLEY_VAST_IMAGE",
        "SHIRLEY_VAST_MAX_DPH",
        "SHIRLEY_VAST_FALLBACK_AFTER_SECS",
    ] {
        remove_env(key);
    }
}

// Keeps the fallback wiring under test even in this binary: with no Vast key
// the composed separator must call the Colab backend exactly once.
#[tokio::test]
async fn no_vast_key_falls_back_to_colab_only() {
    let _lock = ENV_MUTEX.lock().unwrap();
    remove_env("VAST_API_KEY");
    let tmp = TempDir::new().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let colab = TestSeparator::new("colab", calls.clone(), true);
    let composed = FallbackVocalSeparator::from_env(
        Arc::new(colab.clone()),
        tmp.path().join("data"),
        Arc::new(Semaphore::new(1)),
    );
    assert!(!composed.has_vast());
    composed
        .separate(VocalSeparationRequest {
            source_path: tmp.path().join("source.mp3"),
            output_path: tmp.path().join("out.mp3"),
            mode: VocalSeparationMode::Coros,
            timeout: Duration::from_secs(77),
        })
        .await
        .expect("colab succeeds");
    let recorded = calls.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].0, "colab");
    assert_eq!(recorded[0].1.timeout, Duration::from_secs(77));
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Wraps the fake ssh with a delay long enough to cancel the request mid-pipeline.
fn slow_ssh_wrapper(dir: &Path, real_ssh: &Path) -> PathBuf {
    let slow = dir.join("ssh_slow");
    std::fs::write(
        &slow,
        format!("#!/bin/sh\nsleep 1\nexec '{}' \"$@\"\n", real_ssh.display()),
    )
    .unwrap();
    make_executable(&slow);
    slow
}

#[tokio::test]
async fn real_wrapped_instance_shape_uses_direct_ip_and_port() {
    let fixture = Fixture::new(vec![offer(31, 0.21, "RTX 3090", "PL")]).await;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("wrapped real API shape must rent and run");
    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    assert!(
        log.contains("root@127.0.0.1"),
        "public_ipaddr must be trimmed and used: {log}"
    );
    assert!(
        log.contains("-p 2222"),
        "the string HostPort must be used: {log}"
    );
}

#[tokio::test]
async fn proxy_ssh_fields_are_used_when_direct_port_is_missing() {
    let fixture = Fixture::new(vec![offer(32, 0.22, "L4", "PL")]).await;
    fixture.state.inner.lock().unwrap().proxy_only = true;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("proxy-only instance must still be usable");
    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    assert!(log.contains("root@ssh7.vast.ai"), "{log}");
    assert!(log.contains("-p 3333"), "{log}");
}

#[tokio::test]
async fn unwrapped_instance_shape_is_still_accepted() {
    let fixture = Fixture::new(vec![offer(33, 0.23, "RTX A5000", "US")]).await;
    fixture.state.inner.lock().unwrap().unwrapped = true;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("unwrapped legacy response must also work");
    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    assert!(log.contains("root@ssh7.vast.ai"), "{log}");
}

#[tokio::test]
async fn safety_reaper_removes_stale_labelled_instances() {
    let fixture = Fixture::new(vec![]).await;
    {
        let mut inner = fixture.state.inner.lock().unwrap();
        inner.instances = vec![
            json!({ "id": 101, "label": "shirley-show" }),
            json!({ "id": 102, "label": "otro-proyecto" }),
        ];
    }
    let _reaper = fixture.separator().spawn_safety_reaper(Duration::from_millis(30));
    let deadline = Instant::now() + Duration::from_secs(3);
    while fixture.calls().deletes.is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fixture.calls().deletes,
        vec!["101".to_string()],
        "only this app's stale instance may be destroyed"
    );
}

#[tokio::test]
async fn dropped_separation_future_still_releases_the_rented_instance() {
    let mut fixture = Fixture::new(vec![offer(34, 0.24, "RTX 3090", "PL")]).await;
    fixture.config.retention = Duration::from_millis(200);
    let bin_dir = fixture._tmp.path().join("bin");
    let slow = slow_ssh_wrapper(&bin_dir, &fixture.config.ssh_bin);
    fixture.config.ssh_bin = slow;
    let separator = fixture.separator();
    let request = fixture.request(Duration::from_secs(30));
    let task = tokio::spawn(async move { separator.separate(request).await });

    let deadline = Instant::now() + Duration::from_secs(2);
    while fixture.calls().rents.is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(fixture.calls().rents.len(), 1, "an instance was rented");
    task.abort();

    let deadline = Instant::now() + Duration::from_secs(4);
    while fixture.calls().deletes.is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fixture.calls().deletes,
        vec!["7001".to_string()],
        "cancellation must not leave a rented GPU billing forever"
    );
}

/// Minimal in-file separator used only by the no-key wiring check.
#[derive(Clone)]
struct TestSeparator {
    name: &'static str,
    calls: Arc<Mutex<Vec<(String, VocalSeparationRequest)>>>,
    succeed: bool,
}

impl TestSeparator {
    fn new(name: &'static str, calls: Arc<Mutex<Vec<(String, VocalSeparationRequest)>>>, succeed: bool) -> Self {
        Self { name, calls, succeed }
    }
}

impl VocalSeparator for TestSeparator {
    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<shirley_show::ports::vocal_separator::SeparatedAudio, shirley_show::ports::vocal_separator::VocalSeparationError>> + Send>,
    > {
        self.calls
            .lock()
            .unwrap()
            .push((self.name.to_string(), request.clone()));
        let succeed = self.succeed;
        Box::pin(async move {
            if !succeed {
                return Err(shirley_show::ports::vocal_separator::VocalSeparationError::ColabUnavailable);
            }
            std::fs::write(&request.output_path, b"fake").unwrap();
            Ok(shirley_show::ports::vocal_separator::SeparatedAudio {
                duration_secs: 1,
                size_bytes: 4,
            })
        })
    }
}
