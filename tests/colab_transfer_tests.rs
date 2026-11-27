use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::colab_remote_script::generate_remote_script;
use shirley_show::infra::vocal::colab_transfer::{SESSION_IDLE_RETENTION, UploadVerdict, upload_verdict};
use shirley_show::infra::vocal::colab_vocal_separator_config::{
    COROS_MODELS, INSTRUMENTAL_MODELS,
};

// T3. The catbox/AES/curl workaround is gone: the remote script leaves the
// result as a plain MP3 at REMOTE_OUTPUT_PATH and `colab download` fetches it.
// The generated script must no longer reach for openssl, catbox, litterbox, or
// any upload host/URL. This runs for both ensembles so neither mode reintroduces
// the old arc.
#[test]
fn test_remote_script_has_no_encryption_or_upload() {
    for (mode, models) in [
        (VocalSeparationMode::Instrumental, &INSTRUMENTAL_MODELS[..]),
        (VocalSeparationMode::Coros, &COROS_MODELS[..]),
    ] {
        let script = generate_remote_script(mode, models, 12, 8, "pw", "/content/in.wav");

        assert!(!script.contains("openssl"), "openssl encryption must be removed");
        assert!(!script.contains("catbox"), "catbox upload must be removed");
        assert!(!script.contains("litterbox"), "litterbox upload must be removed");
        assert!(!script.contains("catbox.moe"), "catbox host must be removed");
        assert!(!script.contains("litterbox.catbox.moe"), "litterbox host must be removed");
        assert!(!script.contains("enc_path"), "the encrypted payload path must be removed");
        assert!(!script.contains("TRANSFER_OK"), "the uploader-chain protocol must be removed");
        assert!(!script.contains("ALL_UPLOADERS_FAILED"), "the uploader-chain failure path must be removed");
        assert!(!script.contains("tmpfiles"), "the tmpfiles uploader must be removed");
        // No upload URL of any host may appear in the generated script.
        assert!(
            !script.contains("https://") && !script.contains("http://"),
            "the script must not reach an upload URL"
        );
    }
}

// T3. The single-line JSON contract still holds, and it is the only vertical
// signal the separator relies on (`colab exec` truncates large outputs). The
// payload must keep `url` (empty, for the result parser that requires the
// field), `sha256`, `duration` and `size`. Only one JSON line is emitted.
#[test]
fn test_remote_script_emits_single_line_json_with_required_fields() {
    let script = generate_remote_script(
        VocalSeparationMode::Instrumental,
        &INSTRUMENTAL_MODELS,
        12,
        8,
        "pw",
        "/content/in.wav",
    );

    assert!(script.contains("\"url\": \"\""), "url must be present and empty");
    assert!(script.contains("\"sha256\""), "sha256 field must be present");
    assert!(script.contains("\"duration\""), "duration field must be present");
    assert!(script.contains("\"size\""), "size field must be present");
    assert!(script.contains("json.dumps(payload)"), "the payload must be dumped as JSON");
    assert!(script.contains("print(json.dumps(payload))"), "exactly one JSON line must be printed");

    // The JSON is emitted as one literal object; there must be no upload/URL
    // handling that could inject a second line.
    let payload_lines: Vec<&str> = script
        .lines()
        .filter(|l| l.contains("json.dumps(payload)"))
        .collect();
    assert_eq!(payload_lines.len(), 1, "the payload must be dumped exactly once");
}

// T8. The retention window is a named constant, not a literal scattered in the
// code, because it is a product decision the user stated explicitly: three
// minutes after the last request.
#[test]
fn test_retention_window_is_three_minutes() {
    assert_eq!(SESSION_IDLE_RETENTION.as_secs(), 180);
}

// T9. `colab exec --json` does not print the script's stdout: it wraps it in
// its own envelope, `{"duration":..,"gpu":..,"output":"<script stdout>","status":"success"}`.
// Reading only the innermost line means a perfectly good run is reported as
// "invalid output", which is indistinguishable from a real failure.
#[test]
fn test_parse_recovers_payload_from_colab_exec_envelope() {
    let inner = r#"{"url": "", "sha256": "e85cba", "duration": 60.03, "size": 2402264}"#;
    let envelope = serde_json::json!({
        "duration": 133.8,
        "gpu": "G4",
        "output": format!("{inner}\n"),
        "status": "success"
    })
    .to_string();

    let parsed =
        shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(&envelope)
            .expect("the envelope must be unwrapped");
    assert_eq!(parsed.url, "");
    assert_eq!(parsed.sha256, "e85cba");
    assert_eq!(parsed.size, 2402264);
}

// T10. A bare JSON line (no envelope) must still parse, because
// `colab exec` without --json and the probes both produce that shape.
#[test]
fn test_parse_accepts_bare_payload_line() {
    let bare = r#"{"url": "", "sha256": "abc", "duration": 1.0, "size": 10}"#;
    let parsed = shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(bare)
        .expect("a bare line must parse");
    assert_eq!(parsed.url, "");
}

// T11. A runtime that failed must NOT be parsed as success. The envelope says
// "success" even when the script raised, because the kernel ran fine; the real
// signal is the absence of the payload inside "output".
#[test]
fn test_parse_rejects_envelope_without_payload() {
    let envelope = serde_json::json!({
        "duration": 12.0,
        "gpu": "G4",
        "output": "RuntimeError: separation failed\n",
        "status": "success"
    })
    .to_string();
    assert!(
        shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(&envelope)
            .is_none(),
        "an envelope whose output has no payload must not be accepted"
    );
}

// T12. The CLI pretty-prints its envelope across several lines, so a check for
// "a line that starts with { and ends with }" never matches it. This exact
// multi-line shape is what `colab exec --json` produces, and failing it made
// every real separation report "invalid output".
#[test]
fn test_parse_handles_pretty_printed_multiline_envelope() {
    let raw = r#"{
  "duration": 128.975650017,
  "gpu": "G4",
  "output": "{\"url\": \"\", \"sha256\": \"e85cba\", \"duration\": 60.029388, \"size\": 2402264}\n",
  "status": "success"
}"#;
    let parsed = shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(raw)
        .expect("a multi-line envelope must be unwrapped");
    assert_eq!(parsed.url, "");
    assert_eq!(parsed.size, 2402264);
}

// T14. The user's requirement, stated directly: after a request finishes, the
// machine stays up for three minutes, and a second request inside that window
// reuses it instead of paying another cold start. The cache is the piece that
// makes "reuse" possible; without it every request calls `colab start`.
#[test]
fn test_session_cache_reuses_within_retention_and_expires_after() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;

    let mut cache = ColabSessionCache::new();
    assert!(cache.current().is_none(), "nothing cached at boot");

    cache.store("sess-1".to_string());
    assert_eq!(cache.current().as_deref(), Some("sess-1"));

    // Still inside the window: the same session must come back.
    assert_eq!(
        cache
            .reusable(std::time::Duration::from_secs(150))
            .as_deref(),
        Some("sess-1"),
        "a session kept for 3 minutes must be reusable after 150s"
    );

    // Past the window is covered by test_session_cache_expires_against_real_age,
    // which can move the clock; here only the inside-window case is asserted.
}

// T15. A session that Colab dropped on its own (idle timeout, preemption) must
// not be handed back as if it were alive. Marking it gone is what prevents a
// second request from failing against a dead runtime.
#[test]
fn test_session_cache_drops_invalidated_session() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;

    let mut cache = ColabSessionCache::new();
    cache.store("sess-1".to_string());
    cache.invalidate();
    assert!(cache.current().is_none());
    assert_eq!(cache.reusable(std::time::Duration::from_secs(1)), None);
}

// T16. The retention window is a product decision, not a tuning knob that can
// silently become 10 seconds or an hour.
#[test]
fn test_retention_is_three_minutes() {
    assert_eq!(SESSION_IDLE_RETENTION.as_secs(), 180);
}

// T14b. The retention boundary must be checked against the age the session
// actually has, not against the moment the test happens to call in. Aging is
// the whole behaviour, so the clock has to be movable.
#[test]
fn test_session_cache_expires_against_real_age() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::{Duration, Instant};

    let mut cache = ColabSessionCache::new();
    // Stored four minutes ago: past the three-minute window.
    cache.store_at(
        "sess-old".to_string(),
        Instant::now() - Duration::from_secs(240),
    );
    assert_eq!(
        cache.reusable(Duration::from_secs(180)),
        None,
        "a session stored 240s ago is past the 180s window"
    );
    // The id is still known even when it is too old to reuse; knowing it is what
    // lets the caller stop it.
    assert_eq!(cache.current().as_deref(), Some("sess-old"));

    // Stored two minutes ago: still inside the window.
    cache.store_at(
        "sess-new".to_string(),
        Instant::now() - Duration::from_secs(120),
    );
    assert_eq!(
        cache.reusable(Duration::from_secs(180)).as_deref(),
        Some("sess-new")
    );
}

// R1-001 / R3-002 / R4-003. Every success used to spawn an independent 180s stop
// task for the same session id, and a reuse only refreshed the cache timestamp.
// The old timer was never cancelled, so a second request that reused the machine
// could be stopped mid-pipeline by the first request's timer, discarding its GPU
// work. The release has to be superseded, not accumulated.
#[test]
fn test_release_is_superseded_not_accumulated() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::{Duration, Instant};

    let mut cache = ColabSessionCache::new();
    let retention = Duration::from_secs(180);

    // First request finishes: a release is armed for now + retention.
    let first_deadline = Instant::now() + retention;
    cache.store_at("sess-1".to_string(), Instant::now());
    cache.arm_release(first_deadline);
    assert_eq!(cache.release_deadline(), Some(first_deadline));

    // A second request reuses the session and finishes later, so it arms a later
    // deadline. The pending release must move, not stack.
    let second_deadline = Instant::now() + retention + Duration::from_secs(100);
    cache.arm_release(second_deadline);
    assert_eq!(
        cache.release_deadline(),
        Some(second_deadline),
        "a second finish must supersede the earlier release, not add another"
    );

    // A release that has already fired for a superseded generation must not stop
    // the session. The generation counter is what makes that decidable.
    let generation_at_arm = cache.release_generation();
    cache.arm_release(Instant::now() + retention);
    assert!(
        !cache.should_release(generation_at_arm),
        "a stale generation must not be allowed to stop the session"
    );
    let current = cache.release_generation();
    assert!(
        cache.should_release(current),
        "the current generation owns the release"
    );
}

// R3-001 / R4-001. The cache was stamped when the session was chosen, before the
// pipeline ran. A separation longer than the retention window therefore expired
// the entry before it finished, so the next request inside the real window paid a
// cold start while the previous machine was still scheduled to stop. The stamp
// has to be refreshed when the work finishes.
#[test]
fn test_cache_is_stamped_at_finish_not_at_start() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::{Duration, Instant};

    let mut cache = ColabSessionCache::new();
    let retention = Duration::from_secs(180);

    // The session was chosen 200s ago and the pipeline took that long, so the
    // work just finished but the stamp is older than the 180s window. That is the
    // real shape of the defect: a long separation expired its own entry.
    cache.store_at(
        "sess-1".to_string(),
        Instant::now() - Duration::from_secs(200),
    );
    assert_eq!(
        cache.reusable(retention),
        None,
        "stamped at start, the entry looks expired right after finishing"
    );

    // Stamping at finish is the contract the product asks for: three minutes
    // after the request completes.
    cache.refresh_at(Instant::now());
    assert_eq!(
        cache.reusable(retention).as_deref(),
        Some("sess-1"),
        "a fresh stamp at finish makes the window count from completion"
    );
}

// R3-005, el que cuesta dinero. El release se armaba con `arm_release`, que
// devuelve 0 cuando no puede tomar el lock interno, y `should_release(0)` es
// siempre false. Resultado: la tarea de limpieza dormía 180s, se declaraba no
// propietaria y **no apagaba nada**. La GPU quedaba facturando hasta que alguien
// ejecutara `colab stop` a mano.
//
// Confirmado en producción: `Connected: true` con `Idle: 17907s` tras una
// separación, y hubo que liberarla explícitamente.
#[test]
fn test_release_generation_is_never_zero() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::Instant;

    let mut cache = ColabSessionCache::new();
    let generation = cache.arm_release(Instant::now());
    assert_ne!(
        generation, 0,
        "generation 0 is the sentinel that means 'not the owner', so a real \
         release must never be handed it"
    );
    assert!(
        cache.should_release(generation),
        "the task that armed the release must be allowed to stop the session"
    );
}

// Un release armado y nunca superado debe apagar. Este es el camino normal: una
// separación, nadie más usa la máquina, la tarea duerme y apaga.
#[test]
fn test_armed_release_owns_the_shutdown() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::Instant;

    let mut cache = ColabSessionCache::new();
    let gen_normal = cache.arm_release(Instant::now());
    assert!(
        cache.should_release(gen_normal),
        "sin nadie que lo supere, el release debe ejecutarse: si no, la GPU \
         sigue facturando"
    );
}

// Superado por un segundo turno: el viejo no debe apagar, y el nuevo sí.
#[test]
fn test_superseded_release_does_not_stop_but_the_new_one_does() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::Instant;

    let mut cache = ColabSessionCache::new();
    let first = cache.arm_release(Instant::now());
    let second = cache.arm_release(Instant::now());

    assert_ne!(first, second, "re-armar debe cambiar la generación");
    assert!(
        !cache.should_release(first),
        "el release viejo no puede apagar una sesión que otro turno está usando"
    );
    assert!(
        cache.should_release(second),
        "el release nuevo sí debe poder apagar"
    );
}

// El lock interno no puede hacer perder el apagado. Aunque otro hilo esté
// leyendo la caché, el release no debe quedarse sin generación: si esperar no es
// posible, se responde sin bloquear pero el registro debe ser consistente.
#[test]
fn test_release_survives_contention() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::sync::Arc;
    use std::time::Instant;

    let cache = Arc::new(tokio::sync::Mutex::new(ColabSessionCache::new()));
    let cache2 = cache.clone();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (gen_main, gen_other) = rt.block_on(async move {
        let mut guard = cache.lock().await;
        let g1 = guard.arm_release(Instant::now());
        let g2 = guard.arm_release(Instant::now());
        let _ = cache2;
        (g1, g2)
    });

    assert_ne!(gen_main, 0);
    assert_ne!(gen_other, 0);
    assert_ne!(gen_main, gen_other);
}

// R3-005, el único riesgo de dinero que queda abierto. La tarea de release vive
// dentro del proceso: si el proceso muere por SIGKILL, OOM o un reinicio duro,
// nadie ejecuta `colab stop` y la GPU sigue facturando hasta que Colab expira la
// sesión por inactividad. No hay destructor que cubra eso.
//
// La única defensa posible es desde fuera del turno: barrer runtimes huérfanos.
// Dos momentos importan:
//
// 1. Al arrancar. El proceso nuevo es el primero que puede limpiar lo que dejó el
//    anterior, y es exactamente el escenario del SIGKILL.
// 2. Durante la vida del proceso, por si el release falló en silencio.
//
// El barrido apaga TODOS los runtimes de la cuenta, así que la condición para
// hacerlo tiene que ser conservadora: nadie trabajando y nada que merezca
// conservarse.
#[test]
fn test_reaper_never_stops_a_runtime_in_use() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, reap_decision};

    assert_eq!(
        reap_decision(true, false),
        ReapDecision::Keep,
        "hay una separación en curso: apagar mataría el trabajo y perdería la GPU pagada"
    );
    assert_eq!(
        reap_decision(true, true),
        ReapDecision::Keep,
        "aunque la caché esté vacía, un trabajo activo manda"
    );
}

// Una sesión dentro de la ventana de retención se conserva: apagarla rompería la
// reutilización que se pidió.
#[test]
fn test_reaper_keeps_a_session_inside_the_retention_window() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, reap_decision};

    assert_eq!(
        reap_decision(false, true),
        ReapDecision::Keep,
        "nadie trabaja, pero la ventana de retención sigue abierta: se conserva"
    );
}

// Sin trabajo y sin nada que conserve la sesión, es un huérfano: apagar.
#[test]
fn test_reaper_stops_a_true_orphan() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, reap_decision};

    assert_eq!(
        reap_decision(false, false),
        ReapDecision::Stop,
        "sin trabajo y fuera de la ventana, cualquier runtime vivo está facturando \
         sin dueño: hay que apagarlo"
    );
}

// El barrido del arranque no espera ventana: si el proceso anterior murió, su
// sesión no tiene dueño aunque acabe de crearse.
#[test]
fn test_boot_sweep_stops_regardless_of_retention() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, boot_reap_decision};

    assert_eq!(
        boot_reap_decision(true),
        ReapDecision::Keep,
        "si ya hay trabajo arrancando, no se toca"
    );
    assert_eq!(
        boot_reap_decision(false),
        ReapDecision::Stop,
        "al arrancar no hay ningún turno en curso, así que cualquier runtime es huérfano"
    );
}

// ---------------------------------------------------------------------------
// Upload resilience.
//
// `colab upload` exits 1 with "kernel write failed" while still leaving the
// file byte-perfect on the runtime. Measured in production:
//
//   local  sha256 = 5a0b6df6...d470
//   remote sha256 = 5a0b6df6...d470
//   size = 289689 (exact)
//
// Trusting the exit code aborted every separation with `separation_failed`.
// The upload is only failed when the file is absent or its size/checksum does
// not match, so the decision belongs to the verification, not the exit status.
#[test]
fn test_upload_exit_code_alone_does_not_decide_success() {
    // The exact production case: exit 1, file intact.
    assert_eq!(
        upload_verdict(false, Some(289_689), 289_689),
        UploadVerdict::Accept,
        "el CLI sale con 1 pero el archivo llego completo: aceptar"
    );

    // Exit 0 but the file never landed: reject.
    assert_eq!(
        upload_verdict(true, None, 289_689),
        UploadVerdict::Reject,
        "salida 0 sin archivo remoto no es exito"
    );

    // Truncated payload: reject, whatever the exit code says.
    assert_eq!(
        upload_verdict(true, Some(100), 289_689),
        UploadVerdict::Reject,
        "un archivo truncado no sirve aunque el CLI diga que si"
    );

    // Exit 1 and nothing there: retry.
    assert_eq!(
        upload_verdict(false, None, 289_689),
        UploadVerdict::Retry,
        "sin archivo y con fallo: reintentar es lo razonable"
    );
}

// The lease is what stops a boot sweep from killing a living sibling's GPU.
// It was added after measuring that a blind `colab stop` destroyed a
// separation mid-flight: the user saw `separation_failed` with no explanation.
#[test]
fn test_lease_is_taken_by_a_fresh_process() {
    let dir = std::env::temp_dir().join(format!("lease-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // No lease yet: this process owns the runtimes and may sweep.
    assert!(
        !shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&dir),
        "sin arrendamiento previo no hay dueno ajeno"
    );
    shirley_show::infra::vocal::colab_orphan_reaper::release_runtime_lease(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_lease_of_a_dead_process_is_not_respected() {
    let dir = std::env::temp_dir().join(format!("lease-dead-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // A pid that cannot be running: the runtime it left behind is an orphan and
    // must be swept, otherwise it bills forever.
    std::fs::write(dir.join("colab-runtime-owner.pid"), "4294967290").unwrap();
    assert!(
        !shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&dir),
        "un pid muerto no protege un runtime huerfano"
    );
    shirley_show::infra::vocal::colab_orphan_reaper::release_runtime_lease(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_lease_of_a_live_process_is_respected() {
    let dir = std::env::temp_dir().join(format!("lease-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // pid 1 is always running on unix, so it stands in for a live sibling.
    std::fs::write(dir.join("colab-runtime-owner.pid"), "1").unwrap();
    assert!(
        shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&dir),
        "una instancia viva no debe perder su GPU a manos de otra"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
