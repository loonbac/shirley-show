```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:b9687e8dfd20c6c280446e291aa146f86d8e9d7bf4f3558033d3107eedf9c737
verdict: fail
blockers: 2
critical_findings: 2
requirements: 9/10
scenarios: 14/15
test_command: cargo test --test ssh_vocal_separator_tests --test agent_chat_remote_ux_tests --test agent_chat_ui_tests && node tests/agent-chat-state.test.mjs && cargo test && cargo fmt --all -- --check
test_exit_code: 0
test_output_hash: sha256:9204b83d83ea10f8f45bb8a299cb582a41f1ef99520611c5c2df5a19c5859f4c
build_command: cargo build --all-targets
build_exit_code: 0
build_output_hash: sha256:4cf86278c1e6d659943a46a3d7e3da429ac15be90f0f17742e24ef85228e7d3e
```

# Verify report: shirley-show-deploy

**Estado:** FAIL — la implementación y las suites están verdes, pero quedan dos blockers CRITICAL de contrato/proceso; no está lista para sync ni archive.

## Resumen ejecutivo

Se re-verificó `b8aabf0569496304363ba1459bcc01002b7b89d5` con foco en los cinco blockers anteriores. El dispatcher ya deja que `SshVocalSeparator` sostenga la espera y continúe en el mismo turno; el indicador cambia en el primer intento; los manifests tienen los defaults contractuales; y el usuario confirmó explícitamente `size:exception` para la serie `stacked-to-main`.

Persisten dos blockers. Primero, con transporte JSON no streaming, el adapter real nunca emite `wake_wait` durante su espera: sólo termina en éxito o `pc_wake_timeout`; por tanto, el copy contractual de “enciende la PC” y el notice tipado no llegan mientras la operación está esperando. El timer frontend sólo cambia el label y además lo hace para cualquier separación vocal de más de tres segundos, incluida la local. Segundo, la tabla TDD tiene 23 filas pero no cumple el formato/evidencia strict-TDD: 0/23 filas registran `✅ Written` y `✅ Passed`, sólo 16/23 apuntan a archivos de prueba y la afirmación de clippy limpio contradice la ejecución completa.

No se tocó el servidor del usuario en `:3100`, no se ejecutó E2E real y no se escribió en `.run/`.

## Resultado de los cinco ex-bloqueadores

| # | Ex-bloqueador | Resultado | Evidencia |
|---:|---|---|---|
| 1 | Wake-wait real en el mismo turno | RESUELTO | `src/agent/tool_dispatcher.rs` ya llama directamente a `separate_vocals.execute`; el adapter mantiene probe/backoff y las pruebas herméticas de boot en espera pasan. |
| 2 | Typing en primer intento | PARCIAL | El timer y su restauración pasan 17a–17g, pero no recibe una señal remota real y muestra “esperando la PC” también en separación local/lenta. El notice y el copy contractual de espera siguen ausentes durante la espera real. |
| 3 | Manifests | RESUELTO | `--start-period=10s`, `JOSHUA_PC_HOST` vacío por defecto, volumen, host network y clave `:ro`; 11/11 checks estáticos pasan. |
| 4 | Evidencia strict-TDD | NO RESUELTO | Existe tabla de 23 filas, pero 0/23 filas contienen los marcadores obligatorios RED/GREEN y 7/23 no identifican archivo de prueba. |
| 5 | Review workload / `size:exception` | RESUELTO | El prompt de re-verificación registra aceptación humana explícita; el límite de esta unidad `verify-deploy` es 500 y este reporte permanece dentro de él. |

## Cobertura de especificación

| # | Requirement | Estado | Evidencia |
|---:|---|---|---|
| 1 | Multi-Stage Dockerfile and Non-Root Runtime | COMPLETE | Builder/runtime, dependencias, UID/GID 1000, layout y exclusiones presentes. |
| 2 | Persistent Storage and Volume Composition | COMPLETE | Volumen nombrado, `/app/data`, envs y clave `:ro`; recreación no se ejecutó por la prohibición de E2E. |
| 3 | Healthcheck and Network Routing | COMPLETE | Timing/endpoint exactos y `network_mode: host`; routing tailnet no se probó en vivo. |
| 4 | Adapter Selection Local vs Remote | COMPLETE | Config y composición local/remota pasan en suite hermética. |
| 5 | Presence Probe and Wake-Wait | COMPLETE | Probe argv, backoff, timeout y continuación same-turn están implementados y cubiertos sin red real. |
| 6 | Idle Shutdown Lifecycle | COMPLETE | `shutdown -c`, orden y `+60` post-validación están cubiertos por stubs. |
| 7 | Isolation, Transfer, Cleanup, Validation | COMPLETE | UUID, RAII, transferencia y ffprobe están cubiertos; quedan warnings de semántica de status. |
| 8 | Composed Remote Deadline | COMPLETE | Wiring permite 1500s sin clamp remoto; la prueba temporal no cruza 600s. |
| 9 | PC-Offline Signal and Verbatim Copy | INCOMPLETE | `pc_wake_timeout` y give-up existen, pero el adapter real no produce `wake_wait` durante la espera; el copy de encendido no llega al cliente en ese intervalo. |
| 10 | Waiting Typing Indicator | COMPLETE | Label exacto, pulse, `aria-live="polite"` y restauración están presentes; existe el warning de falso positivo local. |

**Requirements completas:** 9/10.

### Escenarios

- **Completos (14/15):** imagen/configuración no-root; persistencia por volumen; host-network topology; selección remota y local; boot durante misma espera; timeout; cancelación de apagado; programación tras éxito; salida corrupta; clave ausente; deadline remoto; give-up; ciclo del indicador.
- **Incompleto (1/15):** `Offline PC asks to power on`, porque el camino real espera dentro de la petición sin entregar `notice: wake_wait` ni el copy de encendido.
- Los tres escenarios de despliegue sólo tuvieron comprobación estática por la restricción explícita de no E2E.

## Completitud de tareas

- Checkboxes: **23/23 marcados**.
- Líneas exactas `- [ ]` restantes: **ninguna**.
- La ausencia de checkboxes pendientes no elimina los blockers de contrato y strict-TDD.

## Estado estructurado y actionContext

- Native SDD status: `ready`; `applyState: all_done`; store `openspec`; cambio inequívoco `shirley-show-deploy`.
- `actionContext.mode: repo-local`; workspace y raíz autorizada: `/home/loonbac/Proyectos/shirley-show`.
- Implementación, pruebas y artefactos pertenecen al workspace autorizado.
- Attempt authority consumida sólo para `verify-deploy`, request `dv-acquire-003`, max 500; no se invocó settle.

## Verificaciones ejecutadas

| Comando exacto | Exit | SHA-256 del output | Resultado |
|---|---:|---|---|
| `cargo test --test ssh_vocal_separator_tests --test agent_chat_remote_ux_tests --test agent_chat_ui_tests && node tests/agent-chat-state.test.mjs && cargo test && cargo fmt --all -- --check` | 0 | `9204b83d83ea10f8f45bb8a299cb582a41f1ef99520611c5c2df5a19c5859f4c` | Focal Rust 29/29; JS directo 17/17; full Cargo 269 passed, 0 failed, 1 ignored; fmt limpio. |
| `cargo build --all-targets` | 0 | `4cf86278c1e6d659943a46a3d7e3da429ac15be90f0f17742e24ef85228e7d3e` | Build correcto. |
| `python3 /tmp/shirley-show-deploy-static-check.py` | 0 | `9487800d74e90b68a2ae2f0ce9d4c04f284490841d4c6186e44cc3e97c332d21` | 11/11 checks de manifests y runbook. |
| `cargo clippy --all-targets -- -D warnings` | 101 | `4dd2ee43320f0f253e1cd912bbce5e1901dd971493e0222535430b5087189feb` | 14 errores clippy del árbol completo; ninguno apunta a los paths de remediación de `b8aabf0`, pero contradice “resolved all clippy warnings”. |
| `test -z "$(git ls-files \| grep -E '(^\|/)(\\.env$\|.*id_(rsa\|ed25519).*$\|.*\\.pem$)' \|\| true)" && ! git grep -nE 'BEGIN (OPENSSH\|RSA\|EC\|DSA\|PRIVATE) PRIVATE KEY' -- . && ! git grep -nE 'gsk_[A-Za-z0-9_-]{24,}' -- ':!tests/**' ':!openspec/**'` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | Sin secretos rastreados detectados. |

Docker/Compose no están instalados; no se construyó imagen. Coverage se omitió porque no hay `cargo-llvm-cov`, tarpaulin ni grcov disponible.

## Correctitud, seguridad y coherencia de diseño

- La frontera `VocalSeparator` y la composición local/remota conservan cohesión y dirección de dependencias.
- `SshCommandBuilder`, `RemoteDirectoryGuard` y `SshVocalSeparator` mantienen responsabilidades explícitas y pruebas herméticas.
- El dispatcher ya no hace el probe prematuro que abortaba la operación.
- **CRITICAL:** D6 y el requisito de notice `wake_wait` no son realizables con la implementación JSON actual: no hay canal para emitir progreso mientras el mismo request permanece abierto, y el adapter real nunca retorna `WakeWait`.
- **WARNING:** `frontend/js/core/state/chat-state.js:168-177` infiere espera remota por tiempo e intención; una separación local o remota ya conectada que dure más de 3s muestra un estado falso.
- **WARNING:** `RemoteDirectoryGuard::run_status` devuelve `Ok(status)` incluso si mkdir/scp termina non-zero, y sus callers no revisan `status.success()`.
- **WARNING:** `schedule_remote_shutdown` imprime stderr remoto crudo en servidor.
- **WARNING:** el runbook recomienda `sudo -n true`, que no valida una regla NOPASSWD limitada al binario `shutdown`.

## Strict TDD Compliance

| Check | Result | Details |
|---|---|---|
| TDD Evidence reported | ✅ | Existe `TDD Cycle Evidence` con 23 filas. |
| All tasks have tests | ❌ | Sólo 16/23 filas apuntan a archivos `tests/...`; 7 usan manifest, documentación o `Workspace`. |
| RED confirmed | ❌ | 0/23 filas dicen `✅ Written`; el texto narra ausencia previa, no identifica una prueba RED ejecutable. |
| GREEN confirmed | ⚠️ | Las suites actuales pasan, pero 0/23 filas dicen `✅ Passed` y no enlazan cada GREEN a la ejecución actual. |
| Triangulation adequate | ⚠️ | 0/23 filas usa conteo `✅ N cases` o `➖ Single`; sólo hay prosa no reconciliable mecánicamente. |
| Safety Net for modified files | ⚠️ | No se registra `✅ N/N` por fila; S1.4 usa `✅ N/A` aunque el soporte exige justificar archivo nuevo/modificado. |

**TDD Compliance:** 1/6 checks plenamente satisfechos; **0/23 filas tienen evidencia formal completa**.

La tabla también afirma en S3.8 “resolved all clippy warnings”, pero `cargo clippy --all-targets -- -D warnings` falla con 14 findings. Bajo strict TDD, la evidencia incompleta/inconsistente es CRITICAL.

### Test Layer Distribution

| Layer | Tests/casos | Files | Tools |
|---|---:|---:|---|
| Unit | 17 | 1 | Node `assert` |
| Integration hermética | 26 | 2 | Cargo/Tokio + SQLite in-memory + stubs |
| Wrappers de suites JS | 3 | 1 | Cargo ejecutando Node; solapan casos JS |
| E2E | 0 | 0 | No ejecutado por instrucción del usuario |

### Changed File Coverage

Coverage analysis skipped — no coverage tool detected.

### Assertion Quality

| File | Línea | Assertion | Issue | Severity |
|---|---:|---|---|---|
| `tests/agent_chat_remote_ux_tests.rs` | 372 | deadline 1500s con delay de 50ms | No cruza el límite 600s que pretende proteger. | WARNING |
| `tests/agent_chat_remote_ux_tests.rs` | 651 | mock `PcBootsMidWaitSeparator` | Prueba el dispatcher same-turn, pero el mock encapsula la espera; el backoff real se demuestra por separado, no end-to-end. | WARNING |

**Assertion quality:** 0 CRITICAL, 2 WARNING. No se detectaron tautologías, ghost loops, assertions type-only aisladas, smoke-only ni assertions CSS en los tests auditados.

### Quality Metrics

- **Formatter:** ✅ limpio.
- **Build/type checking:** ✅ limpio.
- **Linter completo:** ⚠️ 14 findings elevados a error; informativo y no introducido por los paths de remediación, pero invalida la afirmación documental de clippy limpio.

## Review Workload / límite de PR

- Forecast: tres slices encadenados, estrategia `stacked-to-main`, presupuesto canónico 400.
- Los commits S1/S2/S3 existen y mantienen fronteras temáticas.
- La aceptación explícita `size:exception` está en el prompt humano actual y en `apply-progress.md`; no se infirió.
- El commit de remediación `b8aabf0` contiene 438 inserciones y 34 eliminaciones en los nueve paths de remediación inspeccionados; la excepción explícita cubre el riesgo.
- Esta unidad de verificación no modifica implementación y respeta max 500.

## Blockers exactos

1. **CRITICAL — Signal/copy offline aún no cumple el contrato:** `SshVocalSeparator::probe_or_wait_wake` espera internamente y sólo retorna éxito o `PcWakeTimeout`; `WakeWait` no tiene productor real. Con JSON no streaming, el cliente no recibe `notice: wake_wait` ni el copy de encendido durante la espera, y el timer frontend no sustituye ese contrato.
2. **CRITICAL — Evidencia strict-TDD incompleta:** aunque hay 23 filas, 0/23 usa los marcadores obligatorios `✅ Written`/`✅ Passed`, sólo 16/23 identifica test file y la fila S3.8 contradice el clippy completo. La evidencia no permite confirmar el ciclo por tarea.

## Próximo paso recomendado

Volver a apply para reconciliar el contrato de progreso con el transporte elegido y corregir la evidencia TDD por tarea. Después repetir verify; no ejecutar sync/archive mientras estos blockers sigan abiertos.

## Key Learnings

1. La espera same-turn necesita un canal explícito para comunicar progreso contractual.
2. Un timer por intención puede mostrar estado remoto falso en modo local.
3. La evidencia TDD requiere marcadores verificables y archivos de prueba por tarea.
4. Una suite verde no convierte una tabla TDD narrativa en evidencia completa.
