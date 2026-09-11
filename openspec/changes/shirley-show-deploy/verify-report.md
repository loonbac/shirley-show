```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:c1aadfc10893d87b3c4d5cd29a74239b560a57753d0cfd736fc9706bd75dc5f8
verdict: fail
blockers: 5
critical_findings: 5
requirements: 6/10
scenarios: 8/15
test_command: cargo test --test ssh_vocal_separator_tests --test agent_chat_remote_ux_tests --test agent_chat_ui_tests && cargo test && cargo fmt --all -- --check
test_exit_code: 0
test_output_hash: sha256:d9486b92cb0d08eb3910c5ebf57db9f11e4849856785aba66dc9199f277a7e54
build_command: cargo build --all-targets
build_exit_code: 0
build_output_hash: sha256:4cf86278c1e6d659943a46a3d7e3da429ac15be90f0f17742e24ef85228e7d3e
```

# Verify report: shirley-show-deploy

**Estado:** FAIL — no está listo para sync ni archive.

## Resumen ejecutivo

La compilación, el formato y las suites Rust/JS herméticas están verdes: 268 pruebas pasaron y una prueba de GPU real quedó ignorada. Sin embargo, la implementación no cumple el flujo canónico de espera remota: el dispatcher devuelve `wake_wait` después de un probe de 3 segundos y termina el turno, por lo que la misma operación no espera hasta diez minutos ni continúa automáticamente. También hay dos incumplimientos estáticos de despliegue, falta la tabla obligatoria de evidencia TDD y las tres unidades excedieron ampliamente el presupuesto aprobado sin `size:exception`.

No se tocó el servidor del usuario en `:3100`, no se ejecutó E2E real y no se escribió en `.run/`.

## Cobertura de especificación

| # | Requirement | Estado | Evidencia |
|---:|---|---|---|
| 1 | Multi-Stage Dockerfile and Non-Root Runtime | COMPLETE | Stages fijados, runtime slim, UID/GID 1000, layout y exclusiones de secretos presentes. |
| 2 | Persistent Storage and Volume Composition | COMPLETE | `shirley-data:/app/data`, variables y clave `:ro` presentes; recreación no ejecutada por restricción de no E2E. |
| 3 | Healthcheck and Network Routing | INCOMPLETE | Endpoint y host networking presentes, pero `Dockerfile:35` usa `--start-period=15s`, no el contractual `10s`. |
| 4 | Adapter Selection Local vs Remote | COMPLETE | `Config` normaliza host vacío y `AppState` selecciona `Arc<dyn VocalSeparator>`; pruebas pasan. |
| 5 | Presence Probe and Wake-Wait | INCOMPLETE | `tool_dispatcher.rs:730-739` corta el turno tras un probe fallido de 3s; no entra al backoff de `SshVocalSeparator`. |
| 6 | Idle Shutdown Lifecycle | COMPLETE | Cancelación antes de transferencia, programación post-validación y fallos no fatales cubiertos por stubs. |
| 7 | Isolation, Transfer, Cleanup, Validation | COMPLETE | UUID, guard RAII, scp, ffprobe y rechazo de salida inválida cubiertos herméticamente. |
| 8 | Composed Remote Deadline | COMPLETE | Config y wiring admiten 1500s sin clamp remoto; la calidad de la prueba límite es insuficiente, indicada abajo. |
| 9 | PC-Offline Signal and Verbatim Copy | INCOMPLETE | `wake_wait` y copy existen, pero el camino normal offline nunca alcanza diez minutos ni produce `pc_wake_timeout` en el mismo turno. |
| 10 | Waiting Typing Indicator | INCOMPLETE | El label aparece sólo en un turno posterior a un notice previo, no durante la espera de la operación inicial. |

**Requirements completas:** 6/10.

### Escenarios

- **Completos (8/15):** selección remota; preservación local; nuevo trabajo cancela apagado; éxito programa apagado; descarga corrupta rechazada; clave ausente falla antes de red; deadline remoto configurado sin clamp; notice inmediato offline conserva staging y copy.
- **Incompletos/no demostrados (7/15):** imagen healthy con timing exacto; persistencia tras recreación; routing real por tailnet; PC que inicia durante la misma espera; timeout real de diez minutos en el turno; give-up real después de diez minutos; ciclo real del indicador durante espera.

## Completitud de tareas

- Checkboxes: **23/23 marcados**, sin líneas exactas `- [ ]` restantes.
- La marca S3.4 está corregida en el espejo y `apply-progress.md` existe.
- S3.7 se materializa como prueba de integración hermética multiturno con mocks, no como E2E del sistema; por instrucción del usuario no se ejecutó E2E.
- Aunque no quedan checkboxes, los blockers funcionales y de proceso impiden archive.

## Estado estructurado y actionContext

- Native SDD status: `ready`, `applyState: all_done`, store `openspec`, cambio inequívoco `shirley-show-deploy`.
- `actionContext.mode: repo-local`; workspace y raíz permitida: `/home/loonbac/Proyectos/shirley-show`.
- La implementación y los archivos objetivo están dentro de la raíz autorizada.
- No se usó autoridad de settle; el token de attempt se trató como sólo adquisición para `verify-deploy`.

## Verificaciones ejecutadas

| Comando exacto | Exit | Hash SHA-256 | Resultado |
|---|---:|---|---|
| `cargo test --test ssh_vocal_separator_tests --test agent_chat_remote_ux_tests --test agent_chat_ui_tests && cargo test && cargo fmt --all -- --check` | 0 | `d9486b92cb0d08eb3910c5ebf57db9f11e4849856785aba66dc9199f277a7e54` | Focal: 20 SSH + 5 UX remoto + 3 wrappers JS; full: 268 passed, 0 failed, 1 ignored; fmt limpio. |
| `cargo build --all-targets` | 0 | `4cf86278c1e6d659943a46a3d7e3da429ac15be90f0f17742e24ef85228e7d3e` | Build correcto. |
| `python3 /tmp/shirley-show-deploy-static-check.py` | 1 | `30053ad25a5ac6d2070d271217f9a96098f5f6dfc55eedf0766e50ed7d972528` | 10/12: falla timing de healthcheck y fallback local de compose. |
| `cargo clippy --all-targets -- -D warnings` | 101 | `91bb873633c069997e69e278cb9ba5978e44509c23b05595a9445e53683ec650` | 16 warnings elevadas a error; dos `collapsible_if` están en el archivo cambiado `src/agent/tool_dispatcher.rs`. |
| `test -z "$(git ls-files \| grep -E '(^\|/)(\\.env$\|.*id_(rsa\|ed25519).*$\|.*\\.pem$)' \|\| true)" && ! git grep -nE 'BEGIN (OPENSSH\|RSA\|EC\|DSA\|PRIVATE) PRIVATE KEY' -- . && ! git grep -nE 'gsk_[A-Za-z0-9_-]{24,}' -- ':!tests/**' ':!openspec/**'` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | Sin secretos rastreados detectados. |

Docker/Compose no están instalados en el entorno; la validación de manifests fue estática y no se construyó una imagen. La cobertura se omitió porque no se detectó `cargo-tarpaulin`, `grcov` ni `cargo-llvm-cov`.

## Correctitud, seguridad y cohesión

- La frontera `VocalSeparator` conserva al caso de uso desacoplado de SSH y la composición local/remota es explícita.
- `SshCommandBuilder`, `RemoteDirectoryGuard` y `SshVocalSeparator` tienen responsabilidades reconocibles y no introducen dependencia SSH en dominio/caso de uso.
- Los comandos locales usan `Command` con argv; el script remoto es constante y recibe modo/path controlados.
- La suite usa binarios stub y la prueba GPU real quedó ignorada; no hubo contacto intencional con Tailscale ni con `nixos-pc`.
- **WARNING:** `src/infra/vocal/ssh_vocal_separator.rs:424-430` imprime stderr/errores crudos al fallar `shutdown +60`; esto contradice la política de no filtrar detalles de infraestructura en logs visibles.
- **WARNING:** `docs/deployment-coolify.md:45` propone verificar la regla limitada con `sudo -n true`, comando que no está autorizado por la regla NOPASSWD sólo para `shutdown` y por tanto no verifica lo documentado.
- **WARNING:** la implementación ignora el `ExitStatus` no exitoso de mkdir/upload/download porque `run_status` devuelve cualquier status y sus callers sólo usan `?`; el fallo termina más tarde, perdiendo semántica y haciendo trabajo remoto innecesario.

## TDD Compliance

| Check | Result | Details |
|---|---|---|
| TDD Evidence reported | ❌ | Hay bullets bajo `Strict TDD Cycle Evidence`, pero no la tabla obligatoria por tarea. |
| All tasks have tests | ❌ | No existe mapeo task→test para las 23 tareas; packaging sólo declara revisión estática. |
| RED confirmed (tests exist) | ⚠️ | Los archivos reportados de S2/S3 existen, pero no hay filas RED verificables. |
| GREEN confirmed (tests pass) | ✅ | 268/268 pruebas ejecutadas pasaron; 1 E2E GPU permaneció ignorada. |
| Triangulation adequate | ⚠️ | Hay 20 casos SSH y 5 UX remoto, pero los límites 600/1500s se simulan sin cruzar 600s. |
| Safety Net for modified files | ⚠️ | Se afirma suite verde por slice, sin conteos/archivos por fila. |

**TDD Compliance:** 1/6 checks plenamente satisfechos; **0/23 tareas tienen evidencia formal completa**.

### Test Layer Distribution

| Layer | Tests | Files | Tools |
|---|---:|---:|---|
| Unit | 16 | 1 | Node assertions vía wrapper Cargo |
| Integration | 46 | 3 | Rust/Cargo con SQLite in-memory y binarios stub |
| E2E | 0 ejecutadas | 0 | Prohibido por el usuario; la prueba GPU real quedó ignored |
| **Total relacionado en archivos cambiados** | **62** | **4** | |

### Changed File Coverage

Coverage analysis skipped — no coverage tool detected.

### Assertion Quality

| File | Línea | Assertion | Issue | Severity |
|---|---:|---|---|---|
| `tests/agent_chat_remote_ux_tests.rs` | 365 | `test_agent_chat_composed_deadline_1500s_not_clamped_to_600s` | Sólo duerme 50ms; pasaría aunque el deadline efectivo siguiera clamped a 600s. | WARNING |
| `tests/agent_chat_remote_ux_tests.rs` | 220 | mock devuelve `PcWakeTimeout` directamente | No demuestra backoff ni que el flujo real alcance diez minutos. | WARNING |
| `tests/agent_chat_remote_ux_tests.rs` | 465 | flujo offline→online en dos turnos | Valida la conducta implementada que contradice el escenario canónico de continuar la misma operación. | WARNING |

No se detectaron tautologías, ghost loops ni assertions type-only aisladas en los cuatro archivos de prueba cambiados.

### Quality Metrics

- **Formatter:** ✅ limpio.
- **Build/type checking:** ✅ `cargo build --all-targets` limpio.
- **Linter:** ❌ `cargo clippy --all-targets -- -D warnings` reportó 16 findings, incluidos 2 en `src/agent/tool_dispatcher.rs`.

## Review Workload / límite de PR

La separación temática S1/S2/S3 existe en los commits `59cc9c3`, `773d10f` y `87373ac`, y no se observó mezcla funcional entre S2 y S3. Sin embargo, el límite aprobado era 400 líneas por unidad y los tamaños reales fueron:

| Slice | Líneas añadidas | Líneas eliminadas | Total |
|---|---:|---:|---:|
| S1 | 667 | 319 | 986 |
| S2 | 2380 | 19 | 2399 |
| S3 | 1546 | 33 | 1579 |

No hay registro explícito de `size:exception`; `apply-progress.md` confirma que no se usó. Con `ask-on-risk`, exceder de 2.5× a 6× el presupuesto exigía volver a pedir una decisión humana. El chain `stacked-to-main` aprobado no convierte estas unidades sobredimensionadas en conformes.

## Blockers exactos

1. **CRITICAL — Wake-wait contractual roto:** `src/agent/tool_dispatcher.rs:730-739` devuelve `wake_wait` tras 3s y termina el turno, de modo que la PC que enciende durante la ventana no reanuda la misma operación y el timeout real de 10 minutos no ocurre en el camino normal.
2. **CRITICAL — Indicador no representa espera real:** `frontend/js/core/state/chat-state.js:118-128` sólo selecciona `esperando la PC de Joshua…` cuando empieza otro turno después de un notice previo; durante el primer intento offline ya no existe request pendiente ni composer bloqueado.
3. **CRITICAL — Manifests contradicen requisitos:** `Dockerfile:35` usa start period 15s en vez de 10s y `compose.yaml:19` usa `${JOSHUA_PC_HOST:-100.73.247.39}`, por lo que unset o vacío fuerza remoto y contradice el fallback local documentado.
4. **CRITICAL — Evidencia strict-TDD inadmisible:** `apply-progress.md` no contiene la tabla `TDD Cycle Evidence` por tarea exigida; no se pueden validar RED/GREEN/TRIANGULATE/SAFETY NET para 23 tareas.
5. **CRITICAL — Gate de carga de revisión incumplido:** cada slice excede 400 líneas y no existe `size:exception` ni evidencia de una nueva decisión `ask-on-risk` para los tamaños reales.

## Próximo paso recomendado

Volver a apply para corregir los cinco blockers, completar evidencia strict-TDD por tarea y redividir/autorizar explícitamente la carga de revisión. Después, repetir verify antes de sync/archive.

## Key Learnings

1. El probe previo del dispatcher anula la espera interna del adaptador SSH.
2. Los defaults de Compose pueden contradecir la selección local definida en Rust.
3. Una prueba de deadline debe cruzar realmente el límite que pretende proteger.
4. La evidencia TDD estricta requiere una tabla por tarea, no un resumen narrativo.
