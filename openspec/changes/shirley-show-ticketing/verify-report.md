```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:22400c9a069130eb4e7cdc78eb2368536632c31ab089fcbfb2630029eb2f9713
verdict: fail
blockers: 7
critical_findings: 7
requirements: 16/31
scenarios: 52/66
test_command: cargo test
test_exit_code: 0
test_output_hash: sha256:1bb014218409d318085c83fef7cfdd5f799808124aa8355df7002419d7455277
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:dd9fcbb9768b877e78fe421d2b352d3a27cce51b996f226e1076c96105500df4
```

# Verify Report: `shirley-show-ticketing`

## Veredicto

**FAIL — no está listo para sync ni archive.** La compilación y las suites automatizadas están verdes, pero la revisión estática encontró siete bloqueadores: integración frontend/API rota, contexto de evento de Ressy mal enviado, estados HTTP incompatibles con las specs, edición insegura de tipos vendidos, ciclo de vida de afiches incompleto, listado de entradas no acotado y evidencia strict-TDD incompleta.

## Estado estructurado y alcance

- Estado SDD recibido: `ready`; cambio activo único `shirley-show-ticketing`.
- `actionContext.mode`: `repo-local`.
- Workspace autoritativo y raíz permitida: `/home/loonbac/Proyectos/shirley-show`.
- Implementación atribuible a seis commits consecutivos en `main`: `bd54faa`, `03578a4`, `9af8f40`, `d4cd83f`, `b367923`, `03fa345`.
- El árbol tenía `?? production-data/` antes y después de verificar. No se leyó ni modificó ese directorio.
- No se inició ni se tocó el servidor de usuario en `:3100`; no se ejecutó E2E vivo; no se leyó ni escribió `.run/`.
- La authority `sha256:1fbf8183975f82f2bc97774ecccfc2e9594b2cf28056888092f6d4846336a382` se usó solo como contexto del intento; no se ejecutó settle.

## Completitud de tareas

- `tasks.md`: **50/50** marcadas `[x]`.
- Marcadores de implementación sin marcar que coincidan con `^\s*- \[ \]`: **ninguno**.
- No obstante, las casillas no prueban completitud: la tarea final declara comprobaciones completas de clippy, pero el comando completo falla actualmente; además, strict TDD carece de evidencia completa para 42 de 50 tareas.
- Por los hallazgos CRITICAL siguientes, archive sigue bloqueado aunque no haya checkboxes pendientes.

## Cobertura de specs

Conteo canónico recuperado de las ocho specs: **31 requirements y 66 scenarios**.

| Dominio | Requirements completos | Scenarios completos | Resultado |
|---|---:|---:|---|
| `checkin` | 3/3 | 8/8 | Doble uso, código secreto y concurrencia cubiertos. |
| `entrada-publica` | 3/4 | 5/7 | DTO/QR seguros; entrega `wa.me` no funciona en la UI integrada por el contrato frontend/API roto. |
| `entradas` | 3/6 | 12/14 | Emisión N, doble token, pago y QR cubiertos; estados 400 y lista acotada incumplidos. |
| `eventos` | 0/5 | 9/13 | Núcleo presente, pero cada requirement conserva al menos una brecha de contrato, seguridad o lifecycle. |
| `gastos` | 1/3 | 4/7 | CRUD/cálculo presentes; errores de validación devuelven 422 en vez de 400. |
| `interesados` | 2/3 | 6/7 | Conversión atómica y aforo cubiertos; validación HTTP incompatible. |
| `ressy-ticketing` | 3/4 | 6/6 | Ocho tools y dispatcher cubiertos; el frontend no transmite el contexto de evento. |
| `resumen` | 1/3 | 2/4 | Las siete métricas se calculan; desgloses y UI no respetan el contrato canónico integrado. |
| **Total** | **16/31** | **52/66** | **Incompleto** |

## Hallazgos CRITICAL

### C1. El frontend administrativo no consume los envelopes reales de la API

`requestJson()` devuelve el JSON completo (`frontend/js/data/request-json.js:69`) y Axum responde envelopes como `{eventos}`, `{evento}`, `{resumen}`, `{entradas}`, `{interesados}` y `{gastos}` (`src/handlers/ticketing/dto.rs:82-88, 140-142, 226-228, 287-298, 376-378`). Sin embargo, `controllers.js` asigna esas respuestas como arrays/entidades directas (`controllers.js:113,272-275`) y devuelve la respuesta `{entradas}` completa al modal (`controllers.js:377`). Consecuencias:

- catálogo: `events.filter(...)` recibe un objeto y falla;
- detalle y puerta: `event.nombre` queda indefinido;
- tabs: entradas/interesados/gastos reciben objetos, no arrays;
- venta rápida: el resultado no es array y no muestra tarjetas ni enlaces `wa.me`;
- resumen: la vista recibe `{resumen: ...}` y no las métricas directas.

Los tests UI usan fakes con shapes directos, por lo que pasan sin verificar la integración con los DTO Axum reales.

### C2. La UI de evento envía el ID como `presentation_id`, no como `event_id`

El detalle publica `{id, title, type: 'event'}` en `activePresentationContextSignal` (`frontend/js/modules/events/controllers.js:279-283`), pero `agent-chat-controller.js:130-137` toma cualquier contexto como `presentationId`. `sendAgentChatMessage()` no acepta `eventId` y construye únicamente `presentation_id` (`frontend/js/data/agent-chat-api.js:54-107`). El backend sí admite `event_id`, pero nunca lo recibe desde esta UI. Ressy pierde `active_event_id`, no queda grounded al evento visible y la continuidad de las tools de ticketing no cumple el diseño.

### C3. Los estados HTTP de validación no cumplen las specs de ticketing

`AppError::Validation` se traduce globalmente a **422** (`src/error.rs:110-115`) y ventas sobre evento finalizado o capacidad excedida producen `AppError::Conflict`, por tanto **409** (`tickets.rs:314-316,350-353`; `interests.rs:247-249,278-281`). Las specs nuevas exigen **400** para payloads/fecha/importes/enums/cantidad inválidos, evento finalizado, tipo inexistente y capacidad insuficiente. El propio test API fija el comportamiento divergente con `UNPROCESSABLE_ENTITY` (`tests/ticketing_api_tests.rs:57,238`). Esto rompe paridad contractual aun con la suite verde.

### C4. Editar tipos de entrada puede invalidar ventas históricas y la capacidad

`UpdateEvent` reemplaza toda la colección y `EventRepository::update_event` escribe directamente `tipos_de_entrada` (`src/infra/sqlite/ticketing_repository/events.rs:188-205`) sin consultar entradas emitidas. Se puede renombrar/eliminar un tipo ya vendido o reducir su capacidad por debajo de las filas existentes. Una venta posterior cuenta solo el nombre nuevo, de modo que la protección de capacidad puede omitir tickets históricos. El diseño exige preservar nombres vendidos y no bajar capacidad por debajo de emitidas.

### C5. Afiches no cumplen validación MIME ni recuperación/limpieza garantizada

`upload_poster` llama `field.bytes()` y no inspecciona el `Content-Type` declarado (`src/handlers/ticketing/events.rs:172-188`), por lo que acepta bytes JPEG/PNG/WebP aun si el MIME es incompatible. Además, reemplazo y borrado ignoran errores de `delete_poster` (`src/use_cases/ticketing/posters.rs:52,57`; `src/use_cases/ticketing/events.rs:182`), el endpoint de borrado no devuelve `cleanup_pending`, y no existe reconciliación de `.part`, `.trash` u huérfanos al arrancar. Esto contradice la seguridad y recuperabilidad documentadas.

### C6. El listado de entradas no está acotado

`TicketRepository::list_tickets` termina con `ORDER BY` sin `LIMIT` ni paginación (`src/infra/sqlite/ticketing_repository/tickets.rs:126`). Incumple el MUST de lista paginada o completa pero acotada y permite consumo no limitado de memoria/respuesta en un endpoint administrativo sin auth.

### C7. Evidencia strict-TDD incompleta

`openspec/config.yaml` activa strict TDD. `apply-progress.md` contiene una tabla, pero solo para Slice 6 (`apply-progress.md:79`) y con columnas de fases separadas, no una fila completa por tarea con RED/GREEN/TRIANGULATE/SAFETY NET/REFACTOR. No hay evidencia protocolaria para Slices 1-5 (42 tareas) ni safety-net por archivos modificados. Resultado: **0/50 tareas con evidencia TDD completa en el formato exigido**, aunque los archivos reportados de Slice 6 existen y hoy están GREEN. La falta de evidencia completa es CRITICAL bajo strict TDD.

## Paridad funcional confirmada

- Router Axum registra las familias de eventos, entradas, pago, ticket público, check-in, interesados, gastos, resumen y afiches.
- Venta crea N filas en `BEGIN IMMEDIATE`; la suite verifica rollback y techo concurrente.
- `OsRandomCredentials` genera independientemente `st_` + 64 hex y secreto QR de 32 hex.
- QR SVG solo se genera para `Pagado`; el DTO público no expone IDs, contacto, `scan_token` ni `codigo_qr`.
- Check-in solo acepta 32 hex, usa update condicional y diferencia `Valida`, `Ya Usada`, `Invalida`.
- Aforo y siete métricas se calculan en centavos; `ganancia_neta = cobrado - gastos`; existen las siete categorías.
- Conversión interesado→venta comparte la transacción de emisión y bloquea reconversión.
- Ressy registra exactamente 16 tools totales, ocho de ticketing, con allowlist, deduplicación, IDs conocidos y reglas contables en prompt.
- Topbar, rutas hash, vista pública, scanner con cleanup y feedback de puerta existen y sus pruebas aisladas pasan.

## Hallazgos WARNING / informativos

1. `GET /api/eventos?incluir_finalizados=false` ignora el query; el handler siempre lista activos y finalizados, aunque la tool Ressy sí filtra.
2. El resumen HTTP usa `desglose_metodos_pago`/`desglose_gastos` con campos aplanados (`dto.rs:204-205`), mientras spec, JSDoc y vista esperan `por_medio_pago`/`gastos_por_categoria`; esto agrava C1 y vacía los desgloses en UI.
3. El schema de `checkin_entrada.codigo` declara solo `type: string`, sin `pattern`, `minLength` ni `maxLength` para 32 hex; el dominio sí rechaza formatos inválidos.
4. `SHIRLEY_PUBLIC_BASE_URL` se acepta con una comprobación de prefijo HTTP(S), no como origen URL estricto; paths, credenciales u orígenes malformados no se validan según diseño.
5. No existe reintento de hasta tres lotes ante colisión de credenciales, aunque el diseño lo prometía; la unicidad de SQLite sí hace rollback.
6. `cargo fmt --check` falla en archivos cambiados y `cargo clippy --all-targets --all-features -- -D warnings` falla con errores tanto heredados como en tests tocados por este cambio. Son informativos por instrucción del usuario, pero contradicen la casilla REFACTOR final.

## Strict TDD Compliance

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | ❌ | Solo Slice 6 y formato incompleto. |
| All tasks have protocol evidence | ❌ | 0/50 completas; 8 filas parciales de fase para Slice 6. |
| RED files exist | ✅ parcial | Los archivos nombrados para Slice 6 existen. |
| GREEN still true | ✅ | Suite focused 45/45 y full 322/322 pasan. |
| Triangulation adequate | ⚠️ | Buenas variantes en concurrencia/QR/finanzas, pero no se documenta por tarea para Slices 1-5. |
| Safety net | ❌ | No está registrada por tarea/archivo modificado. |

**TDD Compliance: 2/6 checks, CRITICAL.** El historial contiene un commit por slice, por lo que no permite reconstruir RED/GREEN de manera independiente.

## Distribución de capas de prueba

Auditoría de los 17 archivos dedicados a ticketing (12 Rust + 5 Node):

| Capa | Casos/grupos | Archivos | Observación |
|---|---:|---:|---|
| Unit | 18 | 4 | Dominio/credenciales y router/WhatsApp puros. |
| Integración/API/concurrencia | 42 | 12 | SQLite temporal, Axum `oneshot`, dispatcher y fake DOM. |
| Harness Node→Cargo | 5 | 1 | Wrappers Rust que ejecutan los cinco scripts Node. |
| E2E vivo | 0 | 0 | Omitido expresamente; no se tocó `:3100`. |

No hay herramienta de cobertura disponible (`cargo-llvm-cov: not available`); análisis de cobertura omitido sin convertirlo en fallo.

## Assertion Quality

- **0 CRITICAL**: no se encontraron tautologías, loops fantasma, tests sin llamada a producción ni smoke-only aislados.
- **4 WARNING**: `frontend/test/events-ui.test.js:51,62,112,119` afirma clases CSS (`is-active`, `is-finalized`), acoplando pruebas a detalles de implementación. Las demás aserciones verifican valores, resultados, persistencia o comportamiento real.
- Los loops sobre entradas/tools tienen precondiciones de longitud o colecciones deterministas, por lo que no son ghost loops.

## Comandos ejecutados

| Comando exacto | Exit | Evidencia |
|---|---:|---|
| `cargo test --test ticketing_api_tests --test ticketing_credentials_tests --test ticketing_domain_tests --test ticketing_door_concurrency_tests --test ticketing_event_repo_tests --test ticketing_frontend_router_tests --test ticketing_interests_tests --test ticketing_migration_tests --test ticketing_poster_storage_tests --test ticketing_ressy_tests --test ticketing_sale_concurrency_tests --test ticketing_use_cases_tests` | 0 | 45 passed; hash `sha256:0cfb424ca178615c123403ce7bf38b02094e55f1a243cf93b1287107eaef3a34`. |
| `cargo test` | 0 | 322 passed, 0 failed, 1 ignored (GPU real manual); hash `sha256:1bb014218409d318085c83fef7cfdd5f799808124aa8355df7002419d7455277`. |
| `cargo build` | 0 | Build dev exitoso; hash `sha256:dd9fcbb9768b877e78fe421d2b352d3a27cce51b996f226e1076c96105500df4`. |
| `cargo fmt --check` | 1 | Diffs en código/tests cambiados; hash `sha256:f15b1baca999e3b5a683b64787c9acf150b19a58dc61028dbb064df51f83a029`. |
| `cargo clippy --all-targets --all-features -- -D warnings` | 101 | Fallos de lint; hash `sha256:2151ed9d5c30a3ab7335c37fe3087461a47327cd76ead05ac65bcabcf1bbde3c`. |

## Diseño, secretos y cohesión

La dirección principal de dependencias es coherente: handlers, Ressy y UI apuntan a casos de uso; SQLite, filesystem y aleatoriedad están detrás de puertos; el ticket público usa DTO mínimo y el QR se construye con el secreto solo en backend. Los módulos UI presentacionales no hacen fetch directo y el scanner libera tracks. No se encontraron exposiciones de `codigo_qr` en DTOs públicos o administrativos.

La coherencia queda rota en los límites: envelopes API/UI, contexto `event_id`, nombres del resumen, invariantes de actualización de tipos y compensación/reconciliación de afiches. Son fallos de contrato entre módulos, no ausencia de piezas.

## Review Workload / límites de entrega

- Los seis commits corresponden nominalmente a los seis slices, por lo que no se observó mezcla de otro cambio funcional.
- Cada commit excede ampliamente el presupuesto canónico de 400 líneas: 5,423; 2,531; 2,680; 1,948; 4,152; 1,985 inserciones respectivamente (incluyen artefactos en Slice 1).
- `tasks.md` conserva `Chain strategy: pending`; `apply-progress.md` afirma una estrategia encadenada aprobada y registra `size:exception` para Slice 6, pero no contiene evidencia de aceptación humana explícita de la excepción. Esto es **WARNING de gobernanza/revisión**, no un override a los bloqueadores funcionales.
- El resultado entregado son seis commits en `main`, no evidencia de seis PRs revisables independientes.

## Bloqueadores exactos

1. Corregir y probar la adaptación de envelopes entre `api.js`/controladores y los DTO Axum, incluyendo catálogo, detalle, tabs, venta rápida, `wa.me`, resumen y puerta.
2. Transportar `event_id` desde el contexto de ruta y procesar `active_event_id` en el estado de chat sin confundirlo con presentaciones.
3. Alinear a 400 los errores nuevos especificados, conservando 422 solo donde el contrato histórico lo requiera.
4. Impedir renombrar/eliminar tipos vendidos y reducir capacidad por debajo de entradas emitidas.
5. Completar MIME compatible, compensación/`cleanup_pending` y reconciliación de afiches.
6. Acotar o paginar el listado de entradas.
7. Completar la evidencia strict-TDD para las 50 tareas en el formato requerido.

**Siguiente recomendación:** volver a `sdd-apply` para correcciones acotadas y repetir `sdd-verify`; no ejecutar sync/archive y no settle todavía.
