```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:5e05e37005b1ed43e46d8e8a6288cbbacca056ab24faa92bc32b9cbb25580d5d
verdict: fail
blockers: 5
critical_findings: 5
requirements: 26/31
scenarios: 65/66
test_command: env SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-f82a2ba-full cargo test
test_exit_code: 0
test_output_hash: sha256:ac6534ac6656c4d26cfcf978952550de2b8c37be72ea7077a96ba32461c1d56d
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:59639588a6f1826f129cbcd121edc07279dab94786f5fa5bde6b3439b2e845cc
```

# Verify Report: `shirley-show-ticketing`

## Veredicto

**FAIL — no está listo para sync ni archive.** El commit `f82a2ba` corrige por completo C1 y C6, pero C2, C3, C4, C5 y C7 conservan brechas críticas verificables. La suite automatizada y el build están verdes; el fallo es de contratos, concurrencia, lifecycle y cumplimiento strict-TDD.

## Estado estructurado y alcance

- Estado SDD recibido: `ready`; cambio activo único `shirley-show-ticketing`.
- `actionContext.mode`: `repo-local`.
- Workspace autoritativo y raíz permitida: `/home/loonbac/Proyectos/shirley-show`.
- Implementación de remediación atribuible a `f82a2bab9b623ae1a05c916ec44dcf89b8e0bb61`.
- Authority de este intento: `sha256:84d6154f3fd1b81e9b2657b3bd277e9573f822ade54db5257fecd5de9a7df33e`, work-unit `verify-ticketing`, request `tv-acquire-002`, máximo 500; no se ejecutó settle.
- El árbol tenía `?? production-data/` antes y después. No se leyó ni modificó `production-data/` ni `.run/`, y no se inició ni tocó el servidor `:3100`.

## Completitud de tareas

- `tasks.md`: **50/50** tareas de implementación marcadas `[x]`.
- Líneas que coinciden con `^\s*- \[ \]`: **ninguna**.
- No hay bloqueador por checkbox, pero las casillas C2, C3, C4, C5 y C7 no concuerdan completamente con la implementación/evidencia observada.

## Cobertura de specs

Conteo canónico recuperado de ocho specs: **31 requirements y 66 scenarios**.

| Dominio | Requirements | Scenarios | Resultado |
|---|---:|---:|---|
| `checkin` | 3/3 | 8/8 | Completo. |
| `entrada-publica` | 4/4 | 7/7 | C1 restauró envelopes, resumen, venta rápida y puerta. |
| `entradas` | 5/6 | 13/14 | Cantidad negativa deserializa como `u32` antes de `TicketingError` y conserva 422. |
| `eventos` | 2/5 | 13/13 | Persisten brechas de MIME/lifecycle y carrera actualización-venta. |
| `gastos` | 3/3 | 7/7 | Completo. |
| `interesados` | 3/3 | 7/7 | Completo. |
| `ressy-ticketing` | 3/4 | 6/6 | El envío inicial funciona, pero no se conserva `active_event_id` retornado. |
| `resumen` | 3/3 | 4/4 | Los adapters UI normalizan el DTO real. |
| **Total** | **26/31** | **65/66** | **Incompleto**. |

Los scenarios enumerados de afiches y eventos cubren rutas exitosas normales, pero no expresan las brechas no escenarizadas de fallo de cleanup ni concurrencia de actualización; por eso puede haber requirements incompletos con esos scenarios completos.

## Revisión de los siete ex-bloqueadores

| ID | Estado | Evidencia |
|---|---|---|
| C1 | ✅ Corregido | `controllers.js` y `door-controller.js` desempaquetan los envelopes; los tests Node con shapes Axum pasan. |
| C2 | ❌ Parcial | La ruta envía `event_id`, pero `chat-state.js::resolveTurn` solo consume `active_presentation_id` y omite `response.active_event_id`. |
| C3 | ❌ Parcial | `TicketingError` convierte errores de dominio, pero los extractores directos `Json<SellTicketsRequest>`/`Json<CheckInRequest>` evitan el wrapper; por ejemplo `cantidad: -2` hacia `u32` mantiene el rechazo Axum 422 en vez de 400. |
| C4 | ❌ Parcial | Se comprueban tipos y capacidad, pero el `SELECT` y el `UPDATE` no están en una transacción `IMMEDIATE`; una venta concurrente puede entrar entre ambos. Además, `eq_ignore_ascii_case` permite renombrar un tipo vendido cambiando solo el casing. |
| C5 | ❌ Parcial | Hay reconciliación y `cleanup_pending` al borrar evento, pero se acepta el MIME no permitido `image/jpg`; el reemplazo sigue ignorando el fallo al borrar el afiche anterior y no lo comunica. La eliminación DB+filesystem tampoco es atómica como exige la spec, sino eventual/recuperable según el diseño. |
| C6 | ✅ Corregido | La consulta contiene `LIMIT 500`; la prueba asociada es débil y no inserta más de 500 filas. |
| C7 | ❌ Parcial | Hay 57 filas de evidencia, pero ninguna usa las columnas RED/GREEN/TRIANGULATE/SAFETY NET/REFACTOR exigidas por la guía strict-TDD; varias filas apuntan a source paths en vez de tests/comandos verificables. |

## Hallazgos CRITICAL y bloqueadores exactos

### 1. La continuidad de contexto de evento de Ressy sigue incompleta

`frontend/js/core/state/chat-state.js:216-229` actualiza contexto únicamente desde `active_presentation_id`. Si una tool crea o selecciona un evento fuera de una ruta de detalle, el backend devuelve `active_event_id`, pero el navegador no lo conserva y el siguiente turno vuelve a enviarse sin ese evento. La prueba nueva solo verifica el envío desde un contexto de ruta ya existente.

### 2. Los rechazos de deserialización de tickets aún pueden devolver 422

`src/handlers/ticketing/tickets.rs:44-48,82-86,116-119` usa extractores `Json<T>` directos. `SellTicketsRequest.cantidad` es `u32` (`dto.rs:277-283`), por lo que el caso de spec `-2` falla durante extracción y nunca llega a `TicketingError`. Eventos, gastos e interesados sí capturan `JsonRejection` y lo convierten a 400; tickets no.

### 3. La invariancia de tipos/capacidad no es atómica

`src/infra/sqlite/ticketing_repository/events.rs:187-235` consulta entradas emitidas y luego actualiza el JSON sin transacción de escritura. Una venta concurrente puede confirmar después del conteo y antes del `UPDATE`, dejando capacidad configurada por debajo de entradas emitidas. La comparación case-insensitive también contradice la prohibición de renombrar tipos vendidos.

### 4. El lifecycle de afiches sigue divergiendo de la spec

`src/handlers/ticketing/events.rs:224-228` admite `image/jpg`, fuera del catálogo cerrado. `src/use_cases/ticketing/posters.rs:49-57` descarta tanto un fallo compensatorio del archivo nuevo como el fallo al retirar el anterior; el reemplazo puede responder éxito con el asset viejo aún presente. La reconciliación de arranque reduce el daño, pero no satisface el borrado inmediato ni la atomicidad escrita en `eventos/spec.md`.

### 5. La evidencia strict-TDD no cumple el contrato de verificación

Strict TDD está activo. `apply-progress.md` incluye una tabla de 57 filas, pero su schema es `Task ID | Phase | Target | Test File / Command | Outcome & Evidence | Safety Net`. La guía instalada exige por tarea columnas verificables RED, GREEN, TRIANGULATE, SAFETY NET y REFACTOR. Resultado: **0/57 filas con evidencia protocolaria completa**. Además, la prueba de C6 solo afirma la constante 500 y una lista vacía, y la prueba de `cleanup_pending` solo verifica `false`; no ejercitan los resultados que dicen demostrar.

## Strict TDD Compliance

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | ✅ | Existe tabla con 50 tareas y 7 remediaciones. |
| Schema protocolario completo | ❌ | 0/57 filas contienen las cinco columnas requeridas. |
| Test files cross-referenced | ⚠️ | Los archivos reportados existen, pero varias filas GREEN referencian directorios/source paths. |
| GREEN actual | ✅ | 326 passed, 0 failed, 1 ignored en `cargo test`. |
| Triangulation adequate | ⚠️ | C6 no prueba >500; C5 no prueba MIME discordante ni `cleanup_pending=true`; C2 no prueba consumo del response. |
| Safety net auditable | ⚠️ | Hay texto por fila, pero no evidencia ejecutable/revisionada de pre-modificación. |

**TDD Compliance: 2/6 checks; CRITICAL.**

## Assertion Quality

- **0 CRITICAL** por tautologías, ghost loops o tests sin llamada a código de producción.
- **7 WARNING**:
  1. `frontend/test/events-ui.test.js:54,65,115,122` acopla assertions a clases CSS (`is-active`, `is-finalized`).
  2. `tests/ticketing_event_repo_tests.rs:265-276` no crea 501+ tickets; afirmar la constante y una lista vacía no demuestra que el SQL acote resultados.
  3. `tests/ticketing_poster_storage_tests.rs:217-248` solo prueba `cleanup_pending == false`, no la rama anunciada `true`.
  4. `tests/agent-chat-integration.test.mjs:534-590` no prueba que `active_event_id` de la respuesta actualice `activeContext` para el turno siguiente.

## Distribución de capas de prueba

Se auditaron 17 archivos relacionados: 12 suites Rust y 5 scripts Node, incluyendo la integración de chat modificada.

| Capa | Casos/grupos | Archivos | Observación |
|---|---:|---:|---|
| Unit | 18 | 4 | Dominio, credenciales y transformaciones puras. |
| Integración/API/concurrencia | 48 | 12 | SQLite temporal, Axum `oneshot`, dispatcher y fake DOM. |
| Harness Node→Cargo | 5 | 1 | Ejecuta scripts Node desde Cargo. |
| E2E vivo | 0 | 0 | Omitido para no tocar `:3100`. |

Coverage analysis skipped — `cargo-llvm-cov` no está disponible.

## Comandos ejecutados

| Comando exacto | Exit | Resultado/hash |
|---|---:|---|
| `env SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-f82a2ba-focused cargo test --test ticketing_api_tests --test ticketing_event_repo_tests --test ticketing_poster_storage_tests --test ticketing_frontend_router_tests --test ticketing_ressy_tests` | 0 | 26 passed; `sha256:72444cf33751a53ae0bf54ce9cfae73b1e6888a98ca51f087ada0327fa90eafe`. |
| `node frontend/test/events-ui.test.js` | 0 | `sha256:6a1c0d580af41c28654f17a61b87ae3c8d1bc28c67ed414ec28c4e779ca3d7fb`. |
| `node tests/agent-chat-integration.test.mjs` | 0 | `sha256:1514445cd3ce4cbcc56f95a9d53469b3b3d55f3de5f4c7af303fff8596c021f5`. |
| `node frontend/test/door-controller.test.js` | 0 | `sha256:afbf1be2dfe4ab2f245a81a8d9a0632536eb20039322dc11e856bdd0cb99ef54`. |
| `env SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-f82a2ba-full cargo test` | 0 | 326 passed, 0 failed, 1 ignored; `sha256:ac6534ac6656c4d26cfcf978952550de2b8c37be72ea7077a96ba32461c1d56d`. |
| `cargo build` | 0 | `sha256:59639588a6f1826f129cbcd121edc07279dab94786f5fa5bde6b3439b2e845cc`. |
| `cargo fmt --check` | 0 | Salida vacía; `sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. |
| `cargo clippy --all-targets --all-features -- -D warnings` | 101 | Fallos de lint en tests, incluidos `tests/agent_tools_tests.rs`; `sha256:1ef80d6f2746dc849a5a4b5789682c7e5bc42d5ca2fae003f39077600f859cb2`. |

El fallo de clippy es WARNING de calidad, no uno de los cinco bloqueadores funcionales/protocolarios. `cargo fmt`, tests y build sí están verdes.

## Diseño y coherencia modular

La dirección de dependencias principal sigue siendo coherente: handlers y Ressy reutilizan casos de uso; SQLite/filesystem permanecen detrás de puertos; el QR secreto no aparece como campo público; controladores UI consumen `api.js`. Las brechas están en límites entre módulos y en coordinación atómica, no en una mezcla general de responsabilidades.

La reconciliación de afiches implementada es una mitigación válida, pero el diseño reconoce semántica eventual mientras la spec todavía promete eliminación atómica. Esa divergencia debe resolverse explícitamente en código o especificación antes de sync.

## Review Workload / frontera de entrega

- `tasks.md` recomendó seis PRs encadenadas y dejó `Chain strategy: pending`; la implementación está en commits directos sobre `main`.
- La remediación `f82a2ba` cambia 1,177 líneas de código/tests excluyendo `apply-progress.md` y el verify report previo, por encima del presupuesto declarado de 500 y del presupuesto canónico de 400.
- No se registró `size:exception` ni aceptación humana de esa excepción. Esto es **WARNING de gobernanza/revisión**; el scope nominal sí permanece concentrado en C1–C7.

## Siguiente recomendación

Volver a `sdd-apply` con una remediación acotada para los cinco bloqueadores; después repetir `sdd-verify`. No ejecutar sync, archive, settle, publish ni deploy.

## Key Learnings

1. Los wrappers de error no interceptan rechazos ocurridos dentro de extractores Axum directos.
2. Las invariantes de capacidad requieren compartir una transacción de escritura con su validación.
3. La reconciliación eventual de archivos no equivale a eliminación atómica prometida por una especificación.
4. Una tabla TDD completa necesita evidencia verificable por ciclo, no solo etiquetas de fase.
