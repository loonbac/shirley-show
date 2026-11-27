//! Apaga runtimes de Colab que quedaron sin dueño.
//!
//! El release normal vive dentro del proceso: una tarea que duerme la ventana de
//! retención y ejecuta `colab stop`. Eso cubre el camino feliz y los errores, pero
//! **no cubre que el proceso muera**: un SIGKILL, un OOM del contenedor o un
//! reinicio duro no ejecutan ningún destructor, y la GPU sigue facturando hasta
//! que Colab expira la sesión por inactividad. Se midió en producción: una sesión
//! quedó con `Connected: true` durante horas.
//!
//! Lo único que puede cubrir eso es algo que corra **fuera** del turno que murió.
//! Dos momentos:
//!
//! 1. **Al arrancar.** El proceso nuevo es el primero capaz de limpiar lo que dejó
//!    el anterior, y es exactamente el escenario del SIGKILL. Aquí no hay turno en
//!    curso, así que cualquier runtime vivo es huérfano por definición.
//! 2. **Periódicamente.** Por si un release falló en silencio y nadie se enteró.
//!
//! El barrido apaga **todos** los runtimes de la cuenta, así que la condición para
//! hacerlo tiene que ser conservadora: apagar una sesión con trabajo en curso
//! perdería el trabajo y la GPU ya pagada, que es peor que dejarla encendida unos
//! minutos.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

/// Qué hacer con el runtime que esté vivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReapDecision {
    Keep,
    Stop,
}

/// Decide si el barrido periódico debe apagar.
///
/// `has_active_work` es la existencia de una separación en curso;
/// `inside_retention` indica que la sesión cacheada sigue dentro de su ventana y
/// se conserva por la regla de los tres minutos.
///
/// Se apaga solo cuando no hay trabajo **y** nada que conserve la sesión. Ser
/// conservador aquí es deliberado: apagar una sesión con trabajo en curso pierde
/// un trabajo ya pagado, mientras que dejar viva una sesión huérfana cuesta unos
/// minutos de GPU.
pub fn reap_decision(has_active_work: bool, inside_retention: bool) -> ReapDecision {
    if has_active_work || inside_retention {
        ReapDecision::Keep
    } else {
        ReapDecision::Stop
    }
}

/// Decide si el barrido del arranque debe apagar.
///
/// Al arrancar no existe ningún turno en curso del proceso nuevo, así que la
/// ventana de retención no aplica: cualquier runtime vivo lo dejó un proceso
/// anterior que ya no puede apagarlo.
pub fn boot_reap_decision(has_active_work: bool) -> ReapDecision {
    if has_active_work {
        ReapDecision::Keep
    } else {
        ReapDecision::Stop
    }
}

/// Ejecuta `colab stop` sobre todos los runtimes de la cuenta.
///
/// Devuelve `true` cuando el comando terminó con éxito. Un fallo no es fatal: el
/// siguiente barrido lo reintenta y, mientras tanto, el límite es el mismo que ya
/// existía.
/// Whether this process is the one that owns Colab runtimes for this host.
///
/// The boot sweep and any recovery sweep answer the same question: is a runtime
/// alive because *we* left it, or because another live instance is using it?
/// `colab stop` with no session kills **every** runtime in the account, so a
/// second instance sweeping blindly destroys its sibling's work.
///
/// The answer is an owner file holding the process id that claimed it. A pid
/// that is no longer running, a file naming ourselves, or no file at all means
/// the runtimes cannot belong to a living sibling.
///
/// This is a lock, not a guarantee: two instances starting in the same instant
/// can still both see a stale file. That is acceptable because the consequence
/// is a lost GPU, not corrupt data, and the alternative — never sweeping —
/// leaves the orphan billing forever, which is the defect being fixed.
fn lease_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("colab-runtime-owner.pid")
}

/// Claims ownership for this process, returning whether it belonged to someone else.
pub fn claim_runtime_lease(data_dir: &std::path::Path) -> bool {
    let path = lease_path(data_dir);
    let previous = std::fs::read_to_string(&path).ok();
    let foreign = match previous
        .as_deref()
        .and_then(|s| s.trim().parse::<u32>().ok())
    {
        Some(pid) if pid == std::process::id() => false,
        Some(pid) => process_is_running(pid),
        None => false,
    };
    let _ = std::fs::create_dir_all(data_dir);
    let _ = std::fs::write(&path, std::process::id().to_string());
    foreign
}

/// Releases the lease when this process owned it.
pub fn release_runtime_lease(data_dir: &std::path::Path) {
    let path = lease_path(data_dir);
    let ours = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(|pid| pid == std::process::id())
        .unwrap_or(false);
    if ours {
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(unix)]
fn process_is_running(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(unix))]
fn process_is_running(_pid: u32) -> bool {
    false
}
/// Whether a `colab stop -s NAME` outcome counts as a successful cleanup.
///
/// The official CLI exits 0 when it stopped the named session, and 1 when that
/// session no longer exists. Both are a successful cleanup: the session is gone
/// either way. Any other non-zero exit is a real failure.
pub fn stop_ok(status: &std::process::ExitStatus, stderr: &[u8]) -> bool {
    if status.success() {
        return true;
    }
    let err = String::from_utf8_lossy(stderr);
    err.to_ascii_lowercase().contains("not found")
}

/// The path where the official CLI records its sessions (`~/.config/colab-cli/sessions.json`).
pub fn sessions_json_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".config/colab-cli/sessions.json")
    } else {
        std::path::PathBuf::from(".config/colab-cli/sessions.json")
    }
}

/// Enumerates the session names recorded by the official CLI.
///
/// The CLI keeps sessions as a JSON object keyed by session name; a flat array
/// of names (or of objects carrying a name field) is also accepted defensively.
/// A missing or unreadable file yields an empty list, which is safe: there is
/// nothing to stop.
pub fn enumerate_session_names(sessions_json: &std::path::Path) -> Vec<String> {
    let content = match std::fs::read_to_string(sessions_json) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    match value {
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Object(o) => ["name", "session", "id"]
                    .iter()
                    .find_map(|k| match o.get(*k) {
                        Some(serde_json::Value::String(s)) => Some(s.clone()),
                        _ => None,
                    }),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Stops every orphan runtime the official CLI currently knows about.
///
/// The old session-less `colab stop` frees *every* runtime in the account, so
/// the sweep now enumerates the session names from the CLI's own sessions.json
/// and stops each one individually. The boot sweep and the periodic sweep share
/// this path. The token is synced to the legacy path first so the CLI can
/// authenticate.
pub async fn stop_all_runtimes(
    colab_bin: &std::path::Path,
    token_source: &std::path::Path,
    timeout: Duration,
) -> bool {
    let _ = crate::infra::vocal::colab_vocal_separator_config::sync_token_to_legacy(
        token_source,
        &crate::infra::vocal::colab_vocal_separator_config::legacy_token_path(),
    );
    let names = enumerate_session_names(&sessions_json_path());
    let mut all_ok = true;
    for name in names {
        let mut cmd = Command::new(colab_bin);
        cmd.kill_on_drop(true);
        cmd.args(["stop", "-s", &name]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());
        let output = match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(o)) => o,
            _ => {
                all_ok = false;
                continue;
            }
        };
        if !stop_ok(&output.status, &output.stderr) {
            all_ok = false;
        }
    }
    all_ok
}

/// Arranca el vigilante periódico y devuelve su handle.
///
/// Cubre el caso en que el release falló en silencio: la tarea de retención
/// durmió, intentó apagar y no pudo. Sin este vigilante nadie lo reintenta y la
/// GPU sigue facturando hasta que Colab la expire.
///
/// El intervalo es deliberadamente largo. Un barrido apaga **todos** los runtimes
/// de la cuenta, así que hacerlo con frecuencia arriesga matar trabajo en curso;
/// cada minuto es un buen equilibrio entre reaccionar y no estorbar.
pub fn spawn_periodic_reaper(
    colab_bin: std::path::PathBuf,
    token_source: std::path::PathBuf,
    interval: Duration,
    has_active_work: std::sync::Arc<dyn Fn() -> bool + Send + Sync>,
    inside_retention: std::sync::Arc<dyn Fn() -> bool + Send + Sync>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let decision = reap_decision(has_active_work(), inside_retention());
            if decision == ReapDecision::Stop {
                let stopped = stop_all_runtimes(&colab_bin, &token_source, Duration::from_secs(60)).await;
                if std::env::var("SHIRLEY_COLAB_DEBUG_OUTPUT").is_ok() {
                    eprintln!("[reaper] barrido periódico: liberados={stopped}");
                }
            }
        }
    })
}
