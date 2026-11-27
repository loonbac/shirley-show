//! T2: Vast.ai backend behind the existing VocalSeparator port.
//!
//! The HTTP API and ssh/scp are replaced by local fakes: no Vast account, no
//! network egress and no SSH connection are needed to verify the contract.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path as AxumPath, RawQuery, State};
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
    DEFAULT_LAUNCH_TIMEOUT, DEFAULT_MIN_RETRY_REMAINING, DEFAULT_SSH_PROBE_CONNECT_TIMEOUT,
    DEFAULT_SSH_PROBE_INTERVAL, DEFAULT_TRANSIENT_RETRY_DELAY, DEFAULT_VAST_FALLBACK_AFTER_SECS,
    DEFAULT_VAST_IMAGE, DEFAULT_VAST_MAX_DPH, VastVocalSeparatorConfig,
};
use shirley_show::ports::vocal_separator::{
    VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

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
    /// Ordered rent/delete events: a retry must destroy the old instance
    /// before renting the next offer, never hold two at once.
    events: Vec<String>,
    ssh_posts: Vec<Value>,
    instance_gets: usize,
    list_queries: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default)]
enum V1ListShape {
    #[default]
    Wrapped,
    ObjectMap,
    ArrayRoot,
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
    /// Vast v1 may wrap the list differently; exercise all documented shapes.
    v1_list_shape: V1ListShape,
    /// Simulates a host that never leaves `loading` before the launch deadline.
    stall_first_instance: bool,
    /// Each successful rent gets its own contract id, as on the real API.
    next_instance_id: u64,
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
                v1_list_shape: V1ListShape::Wrapped,
                stall_first_instance: false,
                next_instance_id: 7001,
                calls: Calls::default(),
            })),
        }
    }
}

/// The live Vast API rejects scalar filters with 400. The fake must do the
/// same, otherwise an adapter that sends `"verified": true` looks healthy.
const FILTER_FIELDS: [&str; 11] = [
    "verified",
    "rentable",
    "rented",
    "reliability2",
    "num_gpus",
    "gpu_ram",
    "cuda_max_good",
    "inet_down",
    "disk_space",
    "gpu_name",
    "dph_total",
];

fn non_dictionary_filter(body: &Value) -> Option<&'static str> {
    FILTER_FIELDS
        .iter()
        .copied()
        .find(|field| body.get(*field).is_some_and(|value| !value.is_object()))
}

async fn bundles(
    State(state): State<FakeState>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    state.inner.lock().unwrap().calls.bundles.push(body.clone());
    if let Some(field) = non_dictionary_filter(&body) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "bad_request",
                "msg": format!("{field}: Input should be a valid dictionary")
            })),
        )
            .into_response();
    }
    let offers = {
        let inner = state.inner.lock().unwrap();
        let min_down = body
            .get("inet_down")
            .and_then(|filter| filter.get("gte"))
            .and_then(Value::as_f64);
        inner
            .offers
            .clone()
            .into_iter()
            .filter(|offer| {
                min_down.is_none_or(|min| {
                    offer["inet_down"].as_f64().is_some_and(|down| down >= min)
                })
            })
            .collect::<Vec<_>>()
    };
    Json(json!({ "offers": offers })).into_response()
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
    let instance_id = inner.next_instance_id;
    inner.next_instance_id += 1;
    inner.calls.events.push(format!("rent:{instance_id}"));
    Json(json!({ "new_contract": instance_id })).into_response()
}

async fn get_instance(AxumPath(id): AxumPath<String>, State(state): State<FakeState>) -> Json<Value> {
    let mut inner = state.inner.lock().unwrap();
    inner.calls.instance_gets += 1;
    let dead = inner.die_on_next_health && inner.calls.instance_gets >= 2;
    if dead {
        inner.die_on_next_health = false;
    }
    let stalled = inner.stall_first_instance && id == "7001";
    let state = if dead {
        "exited"
    } else if stalled {
        "loading"
    } else {
        "running"
    };
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
    let mut inner = state.inner.lock().unwrap();
    inner.calls.deletes.push(id.clone());
    inner.calls.events.push(format!("delete:{id}"));
    Json(json!({ "deleted": id }))
}

/// Vast retired the v0 listing; the exact body is from the live probe.
async fn deprecated_list_instances() -> axum::response::Response {
    (
        StatusCode::GONE,
        Json(json!({
            "success": false,
            "error": "deprecated_endpoint",
            "msg": "Use /api/v1/instances/ instead."
        })),
    )
        .into_response()
}

async fn list_instances_v1(
    State(state): State<FakeState>,
    RawQuery(query): RawQuery,
) -> axum::response::Response {
    let mut inner = state.inner.lock().unwrap();
    inner.calls.list_queries.push(query.unwrap_or_default());
    let instances = inner.instances.clone();
    let shape = inner.v1_list_shape;
    drop(inner);
    match shape {
        V1ListShape::Wrapped => Json(json!({
            "success": true,
            "instances_found": instances.len(),
            "total_instances": instances.len(),
            "label_counts": {},
            "next_token": null,
            "instances": instances
        }))
        .into_response(),
        V1ListShape::ObjectMap => {
            let map = instances
                .into_iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), value))
                .collect::<serde_json::Map<String, Value>>();
            Json(Value::Object(map)).into_response()
        }
        V1ListShape::ArrayRoot => Json(instances).into_response(),
    }
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
        .route("/api/v0/instances/", get(deprecated_list_instances))
        .route("/api/v1/instances/", get(list_instances_v1))
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
        "inet_down": 1200,
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
/// The fake `scp` is deliberately strict, like the real one: `-p` is
/// preserve-times (never a port), `-P` is mandatory, and exactly one operand
/// must be a `user@host:path` remote. An adapter that reuses ssh argv fails.
fn write_fake_binaries(
    dir: &Path,
    log: &Path,
    scp_argv_log: &Path,
    result_json: &Path,
    output_mp3: &Path,
    fail_python: bool,
    fail_scp: bool,
    ssh_refusals: usize,
    scp_transient_failures: usize,
) -> (PathBuf, PathBuf, PathBuf) {
    let ssh_refusal_file = dir.join("ssh_refusals_left");
    let scp_transient_file = dir.join("scp_transient_left");
    std::fs::write(&ssh_refusal_file, ssh_refusals.to_string()).unwrap();
    std::fs::write(&scp_transient_file, scp_transient_failures.to_string()).unwrap();
    let remote = dir.join("remote");
    std::fs::create_dir_all(&remote).unwrap();
    std::fs::copy(output_mp3, remote.join("output.mp3")).unwrap();
    let ssh = dir.join("ssh");
    let scp = dir.join("scp");
    let ffprobe = dir.join("ffprobe");
    let exit_line = if fail_python { "exit 1" } else { "exit 0" };
    let fail_scp_flag = if fail_scp { "1" } else { "0" };
    std::fs::write(
        &ssh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\nrefuse='{refusal_file}'\nif [ -f \"$refuse\" ]; then n=$(cat \"$refuse\"); else n=0; fi\nif [ \"$n\" -gt 0 ]; then\n  n=$((n - 1)); echo \"$n\" > \"$refuse\"\n  echo 'ssh: connect to host 91.245.78.79 port 16552: Connection refused' >&2\n  exit 255\nfi\ncase \"$*\" in\n  *python*) cat > '{script_file}'; cat '{result_json}'; {exit_line} ;;\n  *rm*) exit 0 ;;\n  *) exit 0 ;;\nesac\n",
            log = log.display(),
            refusal_file = ssh_refusal_file.display(),
            script_file = remote.join("script.py").display(),
            result_json = result_json.display(),
            exit_line = exit_line
        ),
    )
    .unwrap();
    std::fs::write(
        &scp,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
printf '\n-- call --\n' >> '{argv_log}'
for arg in "$@"; do printf '%s\n' "$arg" >> '{argv_log}'; done
port=0
n=0
src=''
dst=''
src_remote=0
dst_remote=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    -p) echo 'scp: -p means preserve-times; use -P for the port' >&2; exit 2 ;;
    -P)
      if [ "$#" -lt 2 ]; then echo 'scp: -P needs a value' >&2; exit 2; fi
      case "$2" in
        ''|*[!0-9]*) echo 'scp: -P needs a numeric port' >&2; exit 2 ;;
      esac
      port="$2"
      shift 2
      ;;
    -i|-o)
      if [ "$#" -lt 2 ]; then echo "scp: $1 needs a value" >&2; exit 2; fi
      shift 2
      ;;
    -r|-q|-v) shift ;;
    -*) echo "scp: unsupported option: $1" >&2; exit 2 ;;
    *)
      n=$((n + 1))
      if [ "$n" -eq 1 ]; then src="$1"; else dst="$1"; fi
      shift
      ;;
  esac
done
if [ "$port" -eq 0 ]; then echo 'scp: missing -P port' >&2; exit 2; fi
if [ "$n" -ne 2 ]; then echo 'scp: exactly two operands are required' >&2; exit 2; fi
case "$src" in
  *@*:*) src_remote=1 ;;
  *@*) echo 'scp: remote operand must be user@host:path' >&2; exit 2 ;;
esac
case "$dst" in
  *@*:*) dst_remote=1 ;;
  *@*) echo 'scp: remote operand must be user@host:path' >&2; exit 2 ;;
esac
if [ "$src_remote" = "$dst_remote" ]; then
  echo 'scp: exactly one user@host:path operand is required' >&2; exit 2
fi
transient='{transient}'
if [ -f "$transient" ]; then n=$(cat "$transient"); else n=0; fi
if [ "$n" -gt 0 ]; then
  n=$((n - 1)); echo "$n" > "$transient"
  echo 'scp: Connection closed by 91.245.78.79' >&2
  exit 255
fi
if [ '{fail_scp}' = "1" ]; then
  echo 'scp: Permission denied (SCP-FAIL-MARKER-42)' >&2
  exit 1
fi
if [ "$src_remote" -eq 1 ]; then
  cp '{remote_output}' "$dst"
else
  cp "$src" '{remote_input}'
fi
"#,
            log = log.display(),
            argv_log = scp_argv_log.display(),
            fail_scp = fail_scp_flag,
            transient = scp_transient_file.display(),
            remote_output = remote.join("output.mp3").display(),
            remote_input = remote.join("input_audio").display(),
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
    scp_argv_log: PathBuf,
    key_path: PathBuf,
    output_bytes: PathBuf,
    ssh_refusal_file: PathBuf,
    scp_transient_file: PathBuf,
}

/// Splits the fake scp's one-argument-per-line log into individual calls.
fn scp_calls(log: &Path) -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(log).unwrap();
    text.split("-- call --")
        .map(|group| {
            group
                .lines()
                .map(str::to_string)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|call| !call.is_empty())
        .collect()
}

impl Fixture {
    async fn new(offers: Vec<Value>) -> Self {
        Self::new_with_scp(offers, false).await
    }

    async fn new_with_scp(offers: Vec<Value>, fail_scp: bool) -> Self {
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
        let scp_argv_log = tmp.path().join("scp-argv.log");
        let binaries_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&binaries_dir).unwrap();
        let (ssh, scp, ffprobe) = write_fake_binaries(
            &binaries_dir,
            &ssh_log,
            &scp_argv_log,
            &result_json,
            &output_bytes,
            false,
            fail_scp,
            0,
            0,
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
        // Long enough that a second separation cannot outlive the lease on a
        // busy host; the retention tests still see it expire promptly.
        config.retention = Duration::from_millis(1500);
        config.poll_interval = Duration::from_millis(5);
        config.http_timeout = Duration::from_secs(5);
        config.launch_timeout = Duration::from_secs(5);
        // The fakes are local, but the suite runs many detached ssh/scp
        // processes in parallel. Keep their budgets generous while the
        // launch-deadline tests set `launch_timeout` explicitly.
        config.preflight_timeout = Duration::from_secs(15);
        config.ssh_probe_interval = Duration::from_millis(20);
        config.ssh_probe_connect_timeout = Duration::from_secs(1);
        config.transient_retry_delay = Duration::from_millis(20);
        let ssh_refusal_file = binaries_dir.join("ssh_refusals_left");
        let scp_transient_file = binaries_dir.join("scp_transient_left");
        Self {
            _tmp: tmp,
            config,
            state,
            _server: server,
            ssh_log,
            scp_argv_log,
            key_path,
            output_bytes,
            ssh_refusal_file,
            scp_transient_file,
        }
    }

    fn set_ssh_refusals(&self, refusals: usize) {
        std::fs::write(&self.ssh_refusal_file, refusals.to_string()).unwrap();
    }

    fn set_scp_transient_failures(&self, failures: usize) {
        std::fs::write(&self.scp_transient_file, failures.to_string()).unwrap();
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
            events: inner.calls.events.clone(),
            ssh_posts: inner.calls.ssh_posts.clone(),
            instance_gets: inner.calls.instance_gets,
            list_queries: inner.calls.list_queries.clone(),
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
    assert_eq!(query["verified"], json!({"eq": true}));
    assert_eq!(query["rentable"], json!({"eq": true}));
    assert_eq!(query["rented"], json!({"eq": false}));
    assert_eq!(query["reliability2"], json!({"gte": 0.99}));
    assert_eq!(query["num_gpus"], json!({"eq": 1}));
    assert_eq!(query["gpu_ram"], json!({"gte": 20000}));
    assert_eq!(query["cuda_max_good"], json!({"gte": 12.4}));
    assert_eq!(query["inet_down"], json!({"gte": 1000.0}));
    assert_eq!(query["disk_space"], json!({"gte": 40}));
    assert_eq!(query["type"], "on-demand");
    assert_eq!(calls.rents.len(), 1);
    assert_eq!(calls.rents[0].0, "5");
    assert_eq!(calls.rents[0].1["image"], fixture.config.image);
    assert_eq!(calls.rents[0].1["disk"], 40);
    assert_eq!(calls.rents[0].1["runtype"], "ssh_direc ssh_proxy");
    assert_eq!(calls.rents[0].1["client_id"], "me");
    assert_eq!(calls.rents[0].1["label"], "shirley-show");
    assert_eq!(calls.ssh_posts.len(), 1);
    assert_eq!(
        calls.ssh_posts[0]["ssh_key"],
        "ssh-ed25519 AAAATESTKEY shirley-test"
    );

    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    assert!(log.contains("python"), "script must run through ssh: {log}");
    assert!(log.contains("-p 2222"), "ssh must use the instance port: {log}");
    let key = fixture.key_path.to_str().unwrap();
    let expected_options = vec![
        "-i",
        key,
        "-P",
        "2222",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=5",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let scp_invocations = scp_calls(&fixture.scp_argv_log);
    assert_eq!(scp_invocations.len(), 2, "upload and download must both use scp");
    let mut upload = expected_options.clone();
    upload.push(
        fixture.request(Duration::from_secs(1)).source_path.to_str().unwrap().to_string(),
    );
    upload.push("root@127.0.0.1:/content/input_audio".to_string());
    assert_eq!(
        scp_invocations[0], upload,
        "scp must not reuse ssh's -p or a bare host operand"
    );
    let mut download = expected_options;
    download.push("root@127.0.0.1:/content/colab_sep_work/output.mp3".to_string());
    download.push(fixture._tmp.path().join("output.mp3").to_str().unwrap().to_string());
    assert_eq!(
        scp_invocations[1], download,
        "download must use a user@host:path operand"
    );
    assert!(
        log.lines().any(|line| line == format!(
            "-i {key} -p 2222 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o BatchMode=yes -o ConnectTimeout=5 root@127.0.0.1 bash -lc 'cd /content && python -'"
        )),
        "exact ssh argv must stay stable: {log}"
    );
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
    let mut fixture = Fixture::new(vec![offer(9, 0.21, "RTX 3090", "PL")]).await;
    // A slow host must not let the retention timer destroy the GPU in the
    // middle of the second request; the expiry is still asserted below.
    fixture.config.retention = Duration::from_secs(5);
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

    let deadline = Instant::now() + Duration::from_secs(8);
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
        let (ssh, scp, ffprobe) = write_fake_binaries(
            &dir,
            &fixture.ssh_log,
            &fixture.scp_argv_log,
            &result_json,
            &output,
            true,
            false,
            0,
            0,
        );
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
        "a proven dead instance must not keep billing (instance_gets={})",
        fixture.calls().instance_gets
    );
    assert!(!separator.warm_instance_available().await);

    // Restore healthy fakes: a fresh instance can still complete a separation.
    let dir = fixture._tmp.path().join("bin");
    let (ssh, scp, ffprobe) = write_fake_binaries(
        &dir,
        &fixture.ssh_log,
        &fixture.scp_argv_log,
        &fixture._tmp.path().join("result.json"),
        &fixture.output_bytes,
        false,
        false,
        0,
        0,
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
        "SHIRLEY_VAST_MIN_RETRY_REMAINING_SECS",
    ] {
        remove_env(key);
    }
    let gpu = Arc::new(Semaphore::new(1));
    let cfg = VastVocalSeparatorConfig::new("super-secret-key".into(), PathBuf::from("/tmp/data"), gpu.clone());
    assert_eq!(cfg.image, DEFAULT_VAST_IMAGE);
    assert_eq!(cfg.max_dph, DEFAULT_VAST_MAX_DPH);
    assert_eq!(cfg.label, "shirley-show");
    assert_eq!(cfg.fallback_after, Duration::from_secs(DEFAULT_VAST_FALLBACK_AFTER_SECS));
    assert_eq!(cfg.launch_timeout, DEFAULT_LAUNCH_TIMEOUT);
    assert_eq!(cfg.preflight_timeout, Duration::from_secs(5));
    assert_eq!(cfg.ssh_probe_interval, DEFAULT_SSH_PROBE_INTERVAL);
    assert_eq!(cfg.ssh_probe_connect_timeout, DEFAULT_SSH_PROBE_CONNECT_TIMEOUT);
    assert_eq!(cfg.transient_retry_delay, DEFAULT_TRANSIENT_RETRY_DELAY);
    assert_eq!(cfg.min_retry_remaining, DEFAULT_MIN_RETRY_REMAINING);
    assert_eq!(DEFAULT_MIN_RETRY_REMAINING, Duration::from_secs(150));
    assert_eq!(DEFAULT_LAUNCH_TIMEOUT, Duration::from_secs(150));
    assert_eq!(DEFAULT_SSH_PROBE_INTERVAL, Duration::from_secs(3));
    assert_eq!(DEFAULT_SSH_PROBE_CONNECT_TIMEOUT, Duration::from_secs(5));
    assert_eq!(DEFAULT_TRANSIENT_RETRY_DELAY, Duration::from_secs(3));
    assert!(!format!("{cfg:?}").contains("super-secret-key"));

    set_env("VAST_API_KEY", "unit-test-dummy");
    set_env("SHIRLEY_VAST_IMAGE", "ghcr.io/loonbac/shirley-vast-worker:sha-123");
    set_env("SHIRLEY_VAST_MAX_DPH", "0.35");
    set_env("SHIRLEY_VAST_FALLBACK_AFTER_SECS", "45");
    set_env("SHIRLEY_VAST_LAUNCH_TIMEOUT_SECS", "120");
    set_env("SHIRLEY_VAST_MIN_RETRY_REMAINING_SECS", "90");
    let from_env = VastVocalSeparatorConfig::from_env(PathBuf::from("/tmp/data"), gpu).unwrap();
    assert_eq!(from_env.image, "ghcr.io/loonbac/shirley-vast-worker:sha-123");
    assert_eq!(from_env.max_dph, 0.35);
    assert_eq!(from_env.fallback_after, Duration::from_secs(45));
    assert_eq!(from_env.launch_timeout, Duration::from_secs(120));
    assert_eq!(from_env.min_retry_remaining, Duration::from_secs(90));
    for key in [
        "VAST_API_KEY",
        "SHIRLEY_VAST_IMAGE",
        "SHIRLEY_VAST_MAX_DPH",
        "SHIRLEY_VAST_FALLBACK_AFTER_SECS",
        "SHIRLEY_VAST_LAUNCH_TIMEOUT_SECS",
        "SHIRLEY_VAST_MIN_RETRY_REMAINING_SECS",
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

/// Fails the first remote script invocation, then delegates to the healthy
/// fake ssh. The counter lets a test prove exactly one pipeline step failed.
fn write_fail_once_ssh(dir: &Path, real_ssh: &Path) -> (PathBuf, PathBuf) {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("ssh_fail_once");
    let counter = dir.join("python_failures_left");
    std::fs::write(&counter, "1").unwrap();
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\ncounter='{counter}'\ncase \"$*\" in\n  *python*)\n    if [ -f \"$counter\" ]; then n=$(cat \"$counter\"); else n=0; fi\n    if [ \"$n\" -gt 0 ]; then\n      n=$((n - 1)); echo \"$n\" > \"$counter\"\n      cat >/dev/null\n      echo 'ssh: simulated remote script failure' >&2\n      exit 1\n    fi\n    ;;\nesac\nexec '{real}' \"$@\"\n",
            counter = counter.display(),
            real = real_ssh.display(),
        ),
    )
    .unwrap();
    make_executable(&path);
    (path, counter)
}

/// Fails the first script, then refuses the very next readiness probe. The
/// flag is cleared by that refusal so the replacement instance can rent and
/// run normally: this isolates one failed pipeline + one failed ssh probe.
fn write_fail_once_then_refuse_probe_ssh(
    dir: &Path,
    real_ssh: &Path,
) -> (PathBuf, PathBuf) {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("ssh_fail_then_probe");
    let counter = dir.join("python_failures_left");
    let flag = dir.join("pipeline_failed_flag");
    std::fs::write(&counter, "1").unwrap();
    std::fs::write(&flag, "0").unwrap();
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nflag='{flag}'\ncounter='{counter}'\ncase \"$*\" in\n  *python*)\n    echo 1 > \"$flag\"\n    if [ -f \"$counter\" ]; then n=$(cat \"$counter\"); else n=0; fi\n    if [ \"$n\" -gt 0 ]; then\n      n=$((n - 1)); echo \"$n\" > \"$counter\"\n      cat >/dev/null\n      echo 'ssh: simulated remote script failure' >&2\n      exit 1\n    fi\n    ;;\n  *true*)\n    if [ -f \"$flag\" ] && [ \"$(cat \"$flag\")\" = \"1\" ]; then\n      echo 0 > \"$flag\"\n      echo 'ssh: connect to host 127.0.0.1 port 2222: Connection refused' >&2\n      exit 255\n    fi\n    ;;\nesac\nexec '{real}' \"$@\"\n",
            counter = counter.display(),
            flag = flag.display(),
            real = real_ssh.display(),
        ),
    )
    .unwrap();
    make_executable(&path);
    (path, counter)
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
    let scp_invocations = scp_calls(&fixture.scp_argv_log);
    assert!(
        scp_invocations.iter().flatten().any(|arg| arg == "-P"),
        "scp must use -P for the port: {scp_invocations:?}"
    );
    assert!(
        scp_invocations
            .iter()
            .flatten()
            .any(|arg| arg == "root@127.0.0.1:/content/input_audio"),
        "scp must use user@host:path: {scp_invocations:?}"
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
    let scp_invocations = scp_calls(&fixture.scp_argv_log);
    assert!(
        scp_invocations
            .iter()
            .flatten()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair[0] == "-P" && pair[1] == "3333"),
        "proxy port must be passed to scp as -P 3333: {scp_invocations:?}"
    );
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

/// A real `POST /bundles/` response: `verified` is absent/null (the server
/// already filtered on it) and `disk_space` is fractional, not an integer.
fn live_offer(id: u64, price: f64, gpu: &str, geo: &str) -> Value {
    json!({
        "id": id,
        "dph_total": price,
        "gpu_name": gpu,
        "geolocation": geo,
        "verified": null,
        "rentable": true,
        "rented": false,
        "reliability2": 0.9923963,
        "num_gpus": 1,
        "gpu_ram": 24576,
        "cuda_max_good": 13.0,
        "inet_down": 870.1,
        "disk_space": 155.75
    })
}

#[tokio::test]
async fn live_offer_shape_with_null_verified_and_fractional_disk_is_accepted() {
    let fixture = Fixture::new(vec![live_offer(77, 0.19, "RTX 3090", "PL")]).await;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("the real bundles response shape must yield a rentable offer");
    assert_eq!(fixture.calls().rents[0].0, "77");
}

#[tokio::test]
async fn strict_fake_rejects_scalar_filters_and_deprecated_v0_list() {
    let fixture = Fixture::new(vec![]).await;
    let client = reqwest::Client::new();
    let rejected = client
        .post(format!("{}/bundles/", fixture.config.api_base))
        .json(&json!({"verified": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    let body: Value = rejected.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(
        body["msg"],
        "verified: Input should be a valid dictionary"
    );

    let deprecated = client
        .get(format!("{}/instances/", fixture.config.api_base))
        .send()
        .await
        .unwrap();
    assert_eq!(deprecated.status(), StatusCode::GONE);
    let body: Value = deprecated.json().await.unwrap();
    assert_eq!(body["error"], "deprecated_endpoint");
}

#[tokio::test]
async fn query_uses_operator_filters_gpu_whitelist_and_price_cap() {
    let fixture = Fixture::new(vec![offer(41, 0.20, "RTX 3090", "PL")]).await;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("separation succeeds");
    let calls = fixture.calls();
    let query = &calls.bundles[0];
    assert_eq!(query["verified"], json!({"eq": true}));
    assert_eq!(query["rentable"], json!({"eq": true}));
    assert_eq!(query["rented"], json!({"eq": false}));
    assert_eq!(query["reliability2"], json!({"gte": 0.99}));
    assert_eq!(query["num_gpus"], json!({"eq": 1}));
    assert_eq!(query["gpu_ram"], json!({"gte": 20000}));
    assert_eq!(query["cuda_max_good"], json!({"gte": 12.4}));
    assert_eq!(query["inet_down"], json!({"gte": 1000.0}));
    assert_eq!(query["disk_space"], json!({"gte": 40}));
    assert_eq!(
        query["gpu_name"]["in"],
        json!([
            "RTX 4090",
            "RTX 3090",
            "RTX A5000",
            "RTX A6000",
            "L4",
            "A10",
            "RTX 4080S"
        ])
    );
    assert_eq!(query["dph_total"]["lte"], json!(fixture.config.max_dph));
    assert_eq!(query["type"], "on-demand");
    assert_eq!(query["order"], json!([["dph_total", "asc"]]));
    assert!(
        query["limit"].as_u64().is_some_and(|limit| limit >= 100),
        "a full page is needed to reach non-CN offers after client-side filtering"
    );
    assert_eq!(calls.rents[0].1["client_id"], "me");
    assert_eq!(calls.rents[0].1["runtype"], "ssh_direc ssh_proxy");
}

#[tokio::test]
async fn chinese_offers_are_filtered_client_side_before_choosing() {
    let mut offers = (0..45u64)
        .map(|id| offer(100 + id, 0.05 + id as f64 * 0.0001, "RTX 4090", "Guangdong, CN"))
        .collect::<Vec<_>>();
    offers.push(offer(999, 0.30, "RTX 3090", "PL"));
    let fixture = Fixture::new(offers).await;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("a non-CN offer must still be rented");
    assert_eq!(fixture.calls().rents[0].0, "999");
}

#[tokio::test]
async fn v1_listing_shapes_are_handled_defensively() {
    for shape in [
        V1ListShape::Wrapped,
        V1ListShape::ObjectMap,
        V1ListShape::ArrayRoot,
    ] {
        let fixture = Fixture::new(vec![]).await;
        {
            let mut inner = fixture.state.inner.lock().unwrap();
            inner.instances = vec![
                json!({"id": 101, "label": "shirley-show"}),
                json!({"id": 102, "label": "otro-proyecto"}),
            ];
            inner.v1_list_shape = shape;
        }
        let removed = fixture.separator().destroy_orphans().await;
        assert_eq!(removed, 1, "shape {shape:?} must still reap the label");
        assert_eq!(fixture.calls().deletes, vec!["101".to_string()]);
        assert!(
            fixture
                .calls()
                .list_queries
                .iter()
                .any(|query| query.contains("select_filters")),
            "the v1 listing should narrow by label server-side"
        );
    }
}

/// Runs a separation whose scp fails. `eprintln!` from the library cannot be
/// captured in-process, so this ignored test is invoked as a child by the
/// assertion below; the parent inspects the child's stderr.
#[tokio::test]
#[ignore = "invoked by vast_failing_scp_logs_stderr_warning"]
async fn vast_failing_scp_warn_helper() {
    let fixture = Fixture::new_with_scp(vec![offer(88, 0.20, "RTX 3090", "PL")], true).await;
    let result = fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await;
    assert!(result.is_err(), "a failed upload must fail the separation");
}

#[test]
fn vast_failing_scp_logs_stderr_warning() {
    let exe = std::env::current_exe().unwrap();
    let output = Command::new(exe)
        .args([
            "--exact",
            "vast_failing_scp_warn_helper",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "helper failed: {stderr}");
    assert!(
        stderr.contains("[vast] WARN upload failed: exit=1 stderr="),
        "the failing step and exit code must be logged: {stderr}"
    );
    assert!(
        stderr.contains("SCP-FAIL-MARKER-42"),
        "scp's stderr must reach the log: {stderr}"
    );
    assert!(
        !stderr.contains("TEST PRIVATE KEY"),
        "no key material may be logged: {stderr}"
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

/// The fake enforces `inet_down.gte` exactly like Vast, so an offer below the
/// fast threshold is invisible until the adapter retries with the floor.
#[tokio::test]
async fn fast_inet_down_query_falls_back_to_500_when_fewer_than_five_offers() {
    let fixture = Fixture::new(vec![offer(51, 0.20, "RTX 3090", "US")]).await;
    fixture.state.inner.lock().unwrap().offers[0]["inet_down"] = json!(900);
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("the slower but acceptable host must be rented");
    let calls = fixture.calls();
    assert!(calls.bundles.len() >= 2);
    assert_eq!(calls.bundles[0]["inet_down"], json!({"gte": 1000.0}));
    assert_eq!(calls.bundles[1]["inet_down"], json!({"gte": 500.0}));
    assert_eq!(calls.rents[0].0, "51");
}

#[tokio::test]
async fn five_fast_offers_avoid_the_slower_inet_down_query() {
    let offers = (0..5u64)
        .map(|i| offer(60 + i, 0.20 + i as f64 * 0.01, "RTX 3090", "PL"))
        .collect::<Vec<_>>();
    let fixture = Fixture::new(offers).await;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("fast host separates");
    let calls = fixture.calls();
    assert_eq!(
        calls.bundles.len(),
        1,
        "no fallback query is needed with five fast offers"
    );
    assert_eq!(calls.bundles[0]["inet_down"], json!({"gte": 1000.0}));
    assert_eq!(calls.rents[0].0, "60");
}

#[tokio::test]
async fn stalled_instance_hits_the_launch_deadline_and_the_next_offer_is_rented() {
    let mut fixture = Fixture::new(vec![
        offer(71, 0.20, "RTX 3090", "PL"),
        offer(72, 0.22, "L4", "US"),
    ])
    .await;
    fixture.state.inner.lock().unwrap().stall_first_instance = true;
    fixture.config.launch_timeout = Duration::from_millis(150);
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(10)))
        .await
        .expect("the second offer reaches running");
    let calls = fixture.calls();
    assert_eq!(
        calls.rents.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["71", "72"]
    );
    assert_eq!(
        calls.deletes,
        vec!["7001".to_string()],
        "a host still loading after the launch deadline must be destroyed"
    );
}

#[tokio::test]
async fn cancelled_rental_of_a_stalled_instance_is_destroyed() {
    let mut fixture = Fixture::new(vec![
        offer(83, 0.20, "RTX 3090", "PL"),
        offer(84, 0.22, "L4", "US"),
    ])
    .await;
    fixture.state.inner.lock().unwrap().stall_first_instance = true;
    fixture.config.launch_timeout = Duration::from_millis(150);
    let separator = fixture.separator();
    let request = fixture.request(Duration::from_secs(10));
    // Drive the future in a task: until it is polled no rental is attempted.
    let pending = tokio::spawn(async move { separator.separate(request).await });
    let deadline = Instant::now() + Duration::from_secs(2);
    while fixture.calls().rents.is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(fixture.calls().rents.len(), 1);
    pending.abort();
    tokio::task::yield_now().await;
    let deadline = Instant::now() + Duration::from_secs(8);
    while fixture.calls().deletes.is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(fixture.calls().deletes, vec!["7001".to_string()]);
    assert_eq!(
        fixture.calls().rents.len(),
        1,
        "cancellation must not try the next offer"
    );
}

/// Vast can report `running` before its sshd accepts connections. The first
/// ssh connections here are refused with the exact live error; the adapter
/// must probe through them before it claims readiness and uploads anything.
#[tokio::test]
async fn ssh_probe_waits_until_the_worker_accepts_before_upload() {
    let fixture = Fixture::new(vec![offer(91, 0.20, "RTX 3090", "PL")]).await;
    fixture.set_ssh_refusals(2);
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("separation succeeds once ssh accepts");
    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    let probes = log.lines().filter(|line| line.trim_end().ends_with(" true")).count();
    assert!(
        probes >= 3,
        "two refusals then one acceptance must all be probed: {log}"
    );
    assert_eq!(scp_calls(&fixture.scp_argv_log).len(), 2, "upload and download only");
    assert_eq!(fixture.calls().rents.len(), 1);
    assert!(fixture.calls().deletes.is_empty());
}

/// A host that stays `running` while ssh refuses connections must be billed
/// only until this offer's launch deadline, then destroyed in favour of the
/// next offer -- exactly like a host stuck in `loading`.
#[tokio::test]
async fn ssh_that_never_accepts_destroys_and_tries_the_next_offer() {
    let mut fixture = Fixture::new(vec![
        offer(92, 0.20, "RTX 3090", "PL"),
        offer(93, 0.22, "L4", "US"),
    ])
    .await;
    fixture.set_ssh_refusals(1000);
    fixture.config.launch_timeout = Duration::from_millis(200);
    assert!(
        fixture
            .separator()
            .separate(fixture.request(Duration::from_secs(10)))
            .await
            .is_err(),
        "neither host accepts ssh before its deadline"
    );
    let calls = fixture.calls();
    assert_eq!(
        calls.rents.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["92", "93"],
        "the next offer must be tried"
    );
    assert_eq!(
        calls.deletes,
        vec!["7001".to_string(), "7002".to_string()],
        "both unreachable instances must be destroyed"
    );
}

#[tokio::test]
async fn warm_instance_with_dead_ssh_is_destroyed_and_rented_fresh() {
    let mut fixture = Fixture::new(vec![offer(94, 0.21, "RTX 3090", "PL")]).await;
    fixture.config.retention = Duration::from_secs(10);
    let separator = fixture.separator();
    separator
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("first separation");
    assert!(separator.warm_instance_available().await);

    fixture.set_ssh_refusals(2);
    assert!(
        !separator.warm_instance_available().await,
        "a warm lease whose ssh is dead must not be reused"
    );
    assert_eq!(fixture.calls().deletes, vec!["7001".to_string()]);
    assert_eq!(fixture.calls().rents.len(), 1);

    fixture.set_ssh_refusals(0);
    separator
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("a fresh instance separates");
    assert_eq!(fixture.calls().rents.len(), 2);
}

#[tokio::test]
async fn transient_scp_255_is_retried_once() {
    let fixture = Fixture::new(vec![offer(95, 0.20, "RTX 3090", "PL")]).await;
    fixture.set_scp_transient_failures(1);
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("one transient Connection closed must not fail the transfer");
    let calls = scp_calls(&fixture.scp_argv_log);
    assert_eq!(calls.len(), 3, "one retry plus download");
    assert_eq!(calls[0], calls[1], "the upload is retried with the same argv");
    assert!(
        calls[2].iter().any(|arg| arg.contains("/content/colab_sep_work/output.mp3")),
        "the download follows the retried upload"
    );
}

/// `eprintln!` from the library cannot be captured in-process; this ignored
/// helper is invoked as a child by the assertion below.
#[tokio::test]
#[ignore = "invoked by transient_connection_logs_are_reported"]
async fn transient_connection_log_helper() {
    let fixture = Fixture::new(vec![offer(96, 0.20, "RTX 3090", "PL")]).await;
    fixture.set_ssh_refusals(2);
    fixture.set_scp_transient_failures(1);
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .expect("both transient failures recover");
}

#[test]
fn transient_connection_logs_are_reported() {
    let exe = std::env::current_exe().unwrap();
    let output = Command::new(exe)
        .args([
            "--exact",
            "transient_connection_log_helper",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "helper failed: {stderr}");
    assert!(
        stderr.contains("[vast] INFO ssh probe attempt 1 failed:"),
        "probe failures must be logged: {stderr}"
    );
    assert!(stderr.contains("Connection refused"), "stderr tail: {stderr}");
    assert!(
        stderr.contains("[vast] INFO ssh accepted after "),
        "readiness must be logged only after acceptance: {stderr}"
    );
    assert!(
        stderr.contains("[vast] INFO upload transient connection failure, retrying"),
        "the scp retry must be logged: {stderr}"
    );
    assert!(stderr.contains("Connection closed"), "scp stderr tail: {stderr}");
    assert!(
        !stderr.contains("TEST PRIVATE KEY"),
        "no key material may be logged: {stderr}"
    );
}

/// S1: one failed pipeline step is retried on the SAME instance when its ssh
/// still works. No second rent, and the healthy GPU stays warm afterwards.
#[tokio::test]
async fn pipeline_failure_retries_once_on_the_same_healthy_instance() {
    let mut fixture = Fixture::new(vec![offer(101, 0.20, "RTX 3090", "PL")]).await;
    fixture.config.retention = Duration::from_secs(5);
    fixture.config.min_retry_remaining = Duration::from_millis(100);
    let (ssh, counter) = write_fail_once_ssh(
        &fixture._tmp.path().join("bin_retry"),
        &fixture.config.ssh_bin,
    );
    fixture.config.ssh_bin = ssh;
    let separator = fixture.separator();

    let result = separator
        .separate(fixture.request(Duration::from_secs(200)))
        .await
        .expect("the retry on the same instance completes the separation");
    assert_eq!(result.duration_secs, 12);
    assert_eq!(std::fs::read_to_string(&counter).unwrap().trim(), "0");

    let calls = fixture.calls();
    assert_eq!(calls.rents.len(), 1, "no second GPU may be rented");
    assert_eq!(calls.events, vec!["rent:7001".to_string()]);
    assert!(calls.deletes.is_empty(), "the healthy instance stays warm");
    assert_eq!(
        scp_calls(&fixture.scp_argv_log).len(),
        3,
        "first attempt uploaded before the script failed; the retry uploads and downloads"
    );
    let log = std::fs::read_to_string(&fixture.ssh_log).unwrap();
    assert_eq!(
        log.lines().filter(|line| line.contains("python")).count(),
        1,
        "the successful retry runs on the already rented worker: {log}"
    );
    assert!(separator.warm_instance_available().await);
}

/// S1: when the quick probe proves the failed instance is dead, it is
/// destroyed BEFORE the next offer is rented. Only one instance is ever live.
#[tokio::test]
async fn failed_retry_probe_destroys_instance_and_rents_next_offer() {
    let mut fixture = Fixture::new(vec![
        offer(102, 0.20, "RTX 3090", "PL"),
        offer(103, 0.22, "L4", "US"),
    ])
    .await;
    fixture.config.retention = Duration::from_secs(5);
    fixture.config.min_retry_remaining = Duration::from_millis(100);
    let (ssh, counter) = write_fail_once_then_refuse_probe_ssh(
        &fixture._tmp.path().join("bin_retry_probe"),
        &fixture.config.ssh_bin,
    );
    fixture.config.ssh_bin = ssh;

    let result = fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(200)))
        .await
        .expect("the replacement instance completes the separation");
    assert_eq!(result.duration_secs, 12);
    assert_eq!(std::fs::read_to_string(&counter).unwrap().trim(), "0");

    let calls = fixture.calls();
    assert_eq!(
        calls.rents.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["102", "103"],
        "the next offer must be tried"
    );
    assert_eq!(
        calls.events,
        vec![
            "rent:7001".to_string(),
            "delete:7001".to_string(),
            "rent:7002".to_string()
        ],
        "destroy must precede the replacement rent; never two concurrently"
    );
    assert_eq!(scp_calls(&fixture.scp_argv_log).len(), 3);
}

/// S1: with a retry floor longer than this turn, the original error is
/// returned unchanged and no attempt is spent on a retry.
#[tokio::test]
async fn pipeline_failure_without_enough_time_returns_the_original_error() {
    let mut fixture = Fixture::new(vec![offer(104, 0.20, "RTX 3090", "PL")]).await;
    fixture.config.retention = Duration::from_secs(5);
    fixture.config.min_retry_remaining = Duration::from_secs(60);
    let (ssh, counter) = write_fail_once_ssh(
        &fixture._tmp.path().join("bin_no_retry"),
        &fixture.config.ssh_bin,
    );
    fixture.config.ssh_bin = ssh;

    let err = fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(30)))
        .await
        .unwrap_err();
    assert_eq!(err, VocalSeparationError::SeparationFailed);
    assert_eq!(err.code(), "separation_failed");
    assert_eq!(std::fs::read_to_string(&counter).unwrap().trim(), "0");
    let calls = fixture.calls();
    assert_eq!(calls.events, vec!["rent:7001".to_string()]);
    assert!(calls.deletes.is_empty(), "the healthy GPU stays warm");
    assert_eq!(
        scp_calls(&fixture.scp_argv_log).len(),
        1,
        "no retry pipeline may start"
    );
}

/// eprintln! from the library cannot be captured in-process; this ignored
/// helper is invoked as a child by the assertion below.
#[tokio::test]
#[ignore = "invoked by retry_logs_include_the_failure_reason"]
async fn retry_log_helper() {
    let mut fixture = Fixture::new(vec![offer(105, 0.20, "RTX 3090", "PL")]).await;
    fixture.config.retention = Duration::from_secs(5);
    fixture.config.min_retry_remaining = Duration::from_millis(100);
    let (ssh, _) = write_fail_once_ssh(
        &fixture._tmp.path().join("bin_retry_log"),
        &fixture.config.ssh_bin,
    );
    fixture.config.ssh_bin = ssh;
    fixture
        .separator()
        .separate(fixture.request(Duration::from_secs(200)))
        .await
        .expect("the retry succeeds");
}

#[test]
fn retry_logs_include_the_failure_reason() {
    let exe = std::env::current_exe().unwrap();
    let output = Command::new(exe)
        .args([
            "--exact",
            "retry_log_helper",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "helper failed: {stderr}");
    assert!(
        stderr.contains("[vast] INFO retry needed after pipeline failure on instance 7001: reason=separation_failed"),
        "the failure reason must be logged: {stderr}"
    );
    assert!(
        stderr.contains("[vast] INFO retry reason=separation_failed: instance 7001 ssh healthy"),
        "the same-instance decision must be logged: {stderr}"
    );
    assert!(
        !stderr.contains("TEST PRIVATE KEY"),
        "no key material may be logged: {stderr}"
    );
}
