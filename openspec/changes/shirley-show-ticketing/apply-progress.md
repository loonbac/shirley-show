# Apply Progress: shirley-show-ticketing

**Change:** `shirley-show-ticketing`  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Current Work Unit:** `fix-tix-2-blockers` (Remediation 2 Final Blockers)
**Authority Token:** `tvf3-begin-003` (max budget: 500 lines)

---

## 1. Cumulative Completed Tasks (50/50 total + Remediation C1–C7 + Remediation B1–B2)

### Slice 1: Esquema de base de datos, dominio y repositorios núcleo (10/10)
- [x] RED: Escribir tests de migración SQLite `0002` que fallen por ausencia de tablas `eventos`, `interesados`, `entradas`, `gastos` y sus índices en `tests/ticketing_migration_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Crear `migrations/0002_ticketing.sql` con las cuatro tablas, checks de centavos/enums, índices y actualizar `src/infra/sqlite/migrate.rs` para aplicar versión 2. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Verificar en `tests/ticketing_migration_tests.rs` la idempotencia de migración, la preservación de versión 1 y las restricciones FK y checks en SQLite. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests unitarios para `Money` (centavos i64, rechazo NaN/negativos/decimales > 2), `TicketType`, `Event`, `ExpenseCategory` y `EventSummary` en `tests/ticketing_domain_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar tipos y validaciones en `src/domain/ticketing/mod.rs`, `src/domain/ticketing/event.rs`, `src/domain/ticketing/expense.rs`, `src/domain/ticketing/summary.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Probar casos borde en `tests/ticketing_domain_tests.rs`: precios cero, ganancia neta negativa, nombres de tipos duplicados case-insensitive, y strings con espacios en blanco. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests de repositorio SQLite para CRUD de eventos, CRUD de gastos y consulta consolidada de resumen en `tests/ticketing_event_repo_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Definir puertos en `src/ports/ticketing.rs` e implementar `SqliteTicketingRepository` para eventos, gastos y resumen en `src/infra/sqlite/ticketing_repository/mod.rs`, `events.rs`, `expenses.rs`, `summary.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar casos de uso `CreateEvent`, `ListEvents`, `GetEvent`, `UpdateEvent`, `CreateExpense`, `ListExpenses`, `GetEventSummary` en `src/use_cases/ticketing/events.rs`, `expenses.rs`, `summary.rs`. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Ejecutar `cargo test` y limpiar advertencias en `src/domain/ticketing/` y `src/infra/sqlite/ticketing_repository/` manteniendo la suite verde. <!-- sdd-owner: implementation -->

### Slice 2: Entradas, venta multi-fila, QR SVG doble token y check-in en puerta (9/9)
- [x] RED: Escribir tests para el generador aleatorio de credenciales y el renderizado QR SVG condicional (solo pagado emite SVG, pendiente retorna null) en `tests/ticketing_credentials_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Añadir dependencia `qrcode` a `Cargo.toml`, implementar `OsRandomCredentials` en `src/infra/random_credentials.rs` y entidades/tokens en `src/domain/ticketing/ticket.rs`. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests de venta transaccional multi-fila (N entradas creadas o rollback total ante capacidad excedida) en `tests/ticketing_sale_concurrency_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `TicketingTransactions::issue_sale` en `src/infra/sqlite/ticketing_repository/tickets.rs` y casos de uso `SellTickets`, `ConfirmTicketPayment`, `GetPublicTicket` en `src/use_cases/ticketing/tickets.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Probar en `tests/ticketing_sale_concurrency_tests.rs` que Cortesía fuerza precio 0.0 y Pagado, que `scan_token` no colisiona y que el secreto `codigo_qr` nunca se expone en la respuesta pública. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests de concurrencia para check-in atómico (dos hilos compitiendo producen exactamente 1 Válida y 1 Ya Usada; rechazo de pendientes y códigos ajenos) en `tests/ticketing_door_concurrency_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar check-in condicional en `src/infra/sqlite/ticketing_repository/tickets.rs` y caso de uso `CheckInTicket` en `src/use_cases/ticketing/checkin.rs` retornando los 3 estados exactos. <!-- sdd-owner: implementation -->
- [x] GREEN: Exponer endpoints de entradas, confirmación de pago, ticket público y check-in en `src/handlers/ticketing/tickets.rs` y conectarlos al router en `src/router.rs`. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Ejecutar `cargo test` y verificar que la venta atómica, generación de QR y check-in pasen con cero fallos de concurrencia. <!-- sdd-owner: implementation -->

### Slice 3: Interesados, almacenamiento de afiches (posters 10MB) y endpoints de administración (8/8)
- [x] RED: Escribir tests para CRUD de interesados y actualización del aforo proyectado en `tests/ticketing_interests_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar entidad `Interest` en `src/domain/ticketing/interest.rs`, repositorio en `src/infra/sqlite/ticketing_repository/interests.rs` y casos de uso en `src/use_cases/ticketing/interests.rs`. <!-- sdd-owner: implementation -->
- [x] RED: Escribir test para `ConvertInterestToSale` verificando emisión transaccional, actualización a `Confirmado` con timestamp y rechazo a reconversión (409) en `tests/ticketing_interests_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `TicketingTransactions::convert_interest` en `src/infra/sqlite/ticketing_repository/interests.rs` y caso de uso en `src/use_cases/ticketing/interests.rs`. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests de almacenamiento de afiches (límite 10 MiB, detección por magic bytes JPEG/PNG/WebP, rechazo de ejecutables, reemplazo de archivo previo) en `tests/ticketing_poster_storage_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `LocalPosterStorage` en `src/infra/filesystem/local_poster_storage.rs` y caso de uso `ReplaceEventPoster` en `src/use_cases/ticketing/posters.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar handlers de administración (interesados CRUD, pase a venta, subida/servido de afiche, finalizar, reabrir, eliminar con cascada y limpieza) en `src/handlers/ticketing/events.rs`, `interests.rs` y router en `src/router.rs`. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Ejecutar `cargo test` validando aislamiento de paths de afiches y cascada completa de borrado de evento. <!-- sdd-owner: implementation -->

### Slice 4: Capacidades de Agente IA (Herramientas Ressy para Ticketing y Finanzas) (7/7)
- [x] RED: Escribir tests que verifiquen exactamente 16 herramientas en `get_tool_definitions()`, incluyendo los nombres exactos de las 8 de ticketing y validación de schemas JSON en `tests/ticketing_ressy_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Registrar los schemas de `crear_evento`, `listar_eventos`, `vender_entradas`, `confirmar_pago_entrada`, `registrar_gasto`, `resumen_evento`, `registrar_interesado`, `checkin_entrada` en `src/agent/tool_schemas.rs`. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests de despacho donde Ressy invoque `vender_entradas`, `resumen_evento` y `checkin_entrada` mediante fakes de casos de uso y valide rechazo de `evento_id` inventado en `tests/ticketing_ressy_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Actualizar `TurnToolContext` con `known_event_ids`, `active_event_id` y despachar las ocho herramientas en `src/agent/dispatcher.rs` invocando `TicketingServices`. <!-- sdd-owner: implementation -->
- [x] GREEN: Añadir directrices de finanzas, cortesías, enlaces de tickets y aclaración de ambigüedades en `SHIRLEY_SYSTEM_PROMPT` en `src/agent/system_prompt.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Probar en `tests/ticketing_ressy_tests.rs` deduplicación por `call.id`, manejo de Cortesía forzada a precio 0.0 y formateo de URL de tickets con `SHIRLEY_PUBLIC_BASE_URL`. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Ejecutar `cargo test` verificando que la suite completa de Ressy (presentaciones + ticketing) compile y pase limpia. <!-- sdd-owner: implementation -->

### Slice 5: Frontend UI - Navegación Topbar, Catálogo de Eventos, Detalle y Pestañas (8/8)
- [x] RED: Escribir tests Node en `frontend/test/router.test.js` para las rutas `#/eventos`, `#/eventos/:id`, `#/ticket/:token`, `#/puerta/:id` y fallback a repertorios. <!-- sdd-owner: implementation -->
- [x] GREEN: Actualizar `frontend/js/router/parse-route.js` y `frontend/js/router/navigate.js` con las nuevas rutas y decodificación segura de parámetros. <!-- sdd-owner: implementation -->
- [x] GREEN: Modificar `frontend/js/components/app-topbar.js` para añadir el selector accesible `Repertorios | Eventos` y actualizar el composition root en `frontend/js/main.js`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar cliente HTTP de ticketing en `frontend/js/modules/events/api.js` cubriendo llamadas a eventos, resumen, entradas, interesados y gastos. <!-- sdd-owner: implementation -->
- [x] GREEN: Construir vistas presentacionales y formularios de catálogo (`#/eventos`) y detalle con pestañas (`#/eventos/:id`) en `frontend/js/modules/events/views.js` y `forms.js`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `controllers.js` en `frontend/js/modules/events/controllers.js` para gestionar ciclo de vida, estados de carga/vacío/error y montaje de pestañas. <!-- sdd-owner: implementation -->
- [x] GREEN: Agregar estilos cohesivos con prefijo `.events-*` en `frontend/styles/events/index.css` e importarlos en `frontend/styles/index.css`. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Probar en navegador que `#/presentations` siga siendo la vista inicial por defecto y que alternar a `#/eventos` cargue catálogo sin errores de consola. <!-- sdd-owner: implementation -->

### Slice 6: Frontend UI - Venta Rápida, Enlaces wa.me, Vista Pública de Ticket y Modo Puerta (8/8)
- [x] RED: Escribir tests unitarios para `whatsapp.js` en `frontend/test/whatsapp.test.js` verificando saneamiento de números, código de país 51 y URLs con encoding seguro para pagado y pendiente. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar función pura de generación de enlaces en `frontend/js/modules/events/whatsapp.js`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar modal y controlador de "Venta Rápida" en `frontend/js/modules/events/forms.js` y `controllers.js`, mostrando tarjetas con enlaces `wa.me` por cada entrada emitida. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests para el renderizado de ticket público (verificando que estado `Pendiente` no renderice contenedor ni imagen de QR) en `frontend/test/public-ticket.test.js`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar vista y controlador de ticket público en `frontend/js/modules/events/public-ticket-controller.js` renderizando `qr_svg` o banner ámbar según pago. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar módulo headless `camera-qr-scanner.js` con ciclo de vida seguro (`start()`, `stop()`, liberación de tracks) y fallback a entrada manual. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar vista y controlador de modo puerta en `frontend/js/modules/events/door-controller.js` con feedback visual inequívoco (verde Válida, ámbar Ya Usada, rojo Inválida) y autofocus. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Ejecutar `cargo test` y comprobaciones completas (`cargo clippy --all-targets --all-features -- -D warnings`), verificando el flujo de venta y validación en puerta de inicio a fin. <!-- sdd-owner: implementation -->

### Remediation Work Unit: Bloqueadores C1 a C7 (7/7)
- [x] C1: Adaptar `controllers.js` y `door-controller.js` para consumir envelopes Axum reales (`{eventos}`, `{evento}`, `{resumen}`, `{entradas}`, `{interesados}`, `{gastos}`) + mapeo de desgloses + tests de integración con shapes reales en `frontend/test/events-ui.test.js`. <!-- sdd-owner: implementation -->
- [x] C2: Contexto de evento viaja como `event_id` en `sendAgentChatMessage` y `agent-chat-controller.js`, y `chat-state.js` consume `response.active_event_id` actualizando `activeContext` (`type: event`) para turnos posteriores + tests en `tests/agent-chat-integration.test.mjs` y `tests/agent-chat-state.test.mjs`. <!-- sdd-owner: implementation -->
- [x] C3: Mapeo de errores de validación y deserialización a HTTP 400 Bad Request via `Result<Json<T>, JsonRejection>` en handlers de entradas (`src/handlers/ticketing/tickets.rs`) + tests de cantidad -2, tipos inválidos y json malformado en `tests/ticketing_api_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C4: Transacción `IMMEDIATE` para chequeo y actualización de tipos en `SqliteTicketingRepository::update_event`, exigiendo match exacto para tipos vendidos (prohíbe cambio de casing) y piso de capacidad >= emitidas + tests en `tests/ticketing_event_repo_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C5: Rechazo estricto de `image/jpg` (catálogo cerrado `image/jpeg`, `image/png`, `image/webp`), propagación de errores de borrado de archivo, compensación verificada en reemplazo si falla borrado de afiche previo, y `cleanup_pending=true` verificado + tests en `tests/ticketing_api_tests.rs` y `tests/ticketing_poster_storage_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C6: Acotamiento riguroso de listado de entradas con `DEFAULT_TICKETS_LIMIT = 500` con prueba que inserta 520 filas en DB y comprueba truncamiento exacto a 500 en `tests/ticketing_event_repo_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C7: Reconciliación completa de evidencia TDD con schema protocolario de 8 columnas (`Task`, `Test File`, `Layer`, `Safety Net`, `RED`, `GREEN`, `TRIANGULATE`, `REFACTOR`) por cada una de las 50 tareas y 7 remediaciones en `apply-progress.md`. <!-- sdd-owner: implementation -->

### Final Remediation Work Unit: 2 Bloqueadores del Verify Report (2/2)
- [x] Rem2-B1: Borrado de evento en orden FS-primero (elimina afiche de disco; si falla FS -> aborta 500 dejando DB completamente intacta), luego cascada DB en SQLite; huérfanos gestionados vía `reconcile_orphans`; propagación de errores de borrado de `.trash` en `LocalPosterStorage::delete_poster`; actualización de contrato en `specs/eventos/spec.md` y `design.md` + tests en `tests/ticketing_poster_storage_tests.rs`. <!-- sdd-owner: implementation -->
- [x] Rem2-B2: Test concurrente C4 (`test_update_event_concurrency_with_sales_in_immediate_tx`) refactorizado con `Barrier` para forzar entrelazado de hilos, afirmación incondicional de invariante universal (`general_cap >= emitted`), afirmación exhaustiva de ambos órdenes válidos sin ramas que omitan aserciones, y nuevo test secuencial determinístico del segundo orden (`test_update_event_lowered_capacity_first_then_sale_exceeds_fails`) en `tests/ticketing_event_repo_tests.rs`. <!-- sdd-owner: implementation -->

---

## 2. Strict TDD Cycle Evidence Table (S1–S6 + Remediation C1–C7)

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|---|---|---|---|---|---|---|---|
| S1-T1 | `tests/ticketing_migration_tests.rs` | Integration | N/A (new) | ✅ Written (asserted 4 tables and indices) | ✅ Passed (tables created) | ✅ 2 cases (tables, indices) | ✅ Clean (clean DDL) |
| S1-T2 | `tests/ticketing_migration_tests.rs` | Integration | ✅ 1/1 passing | ✅ Written (asserted version 2 runner) | ✅ Passed (migration applied) | ➖ Single (migration runner) | ✅ Clean (idempotent runner) |
| S1-T3 | `tests/ticketing_migration_tests.rs` | Integration | ✅ 2/2 passing | ✅ Written (asserted FK and checks) | ✅ Passed (FK checks enforced) | ✅ 3 cases (idempotent, FK, checks) | ✅ Clean (zero warnings) |
| S1-T4 | `tests/ticketing_domain_tests.rs` | Unit | N/A (new) | ✅ Written (Money, TicketType, Event) | ✅ Passed (domain entities valid) | ✅ 3 cases (cents, nan, negative) | ✅ Clean (pure domain) |
| S1-T5 | `tests/ticketing_domain_tests.rs` | Unit | ✅ 3/3 passing | ✅ Written (ExpenseCategory, Summary) | ✅ Passed (categories and summary valid) | ✅ 2 cases (7 categories, summary) | ✅ Clean (structs encapsulated) |
| S1-T6 | `tests/ticketing_domain_tests.rs` | Unit | ✅ 5/5 passing | ✅ Written (edge cases: zero, neg profit) | ✅ Passed (all edge cases valid) | ✅ 4 cases (zero, neg profit, spaces, trim) | ✅ Clean (pure functions) |
| S1-T7 | `tests/ticketing_event_repo_tests.rs` | Integration | N/A (new) | ✅ Written (CRUD events repo) | ✅ Passed (create/get/list events) | ✅ 2 cases (empty, populated) | ✅ Clean (SQLite pooled) |
| S1-T8 | `tests/ticketing_event_repo_tests.rs` | Integration | ✅ 1/1 passing | ✅ Written (CRUD expenses & summary) | ✅ Passed (expenses and metrics calculated) | ✅ 2 cases (multi-expense, 7 metrics) | ✅ Clean (proper traits) |
| S1-T9 | `tests/ticketing_use_cases_tests.rs` | Unit | N/A (new) | ✅ Written (CreateEvent, ListEvents) | ✅ Passed (use cases executed) | ✅ 3 cases (create, get, list) | ✅ Clean (isolated ports) |
| S1-T10 | `cargo test --test ticketing_domain_tests` | Unit | ✅ 15/15 passing | ✅ Written (refactor safety suite) | ✅ Passed (all 15 tests green) | ➖ Single (full slice regression) | ✅ Clean (clippy & fmt clean) |
| S2-T1 | `tests/ticketing_credentials_tests.rs` | Unit | N/A (new) | ✅ Written (random credentials & QR SVG) | ✅ Passed (st_ 64hex, secret 32hex, SVG) | ✅ 2 cases (format, uniqueness) | ✅ Clean (OsRandom isolated) |
| S2-T2 | `tests/ticketing_credentials_tests.rs` | Unit | ✅ 2/2 passing | ✅ Written (SVG rendered only on paid) | ✅ Passed (paid has SVG, pending null) | ✅ 2 cases (paid, pending) | ✅ Clean (zero secret leak) |
| S2-T3 | `tests/ticketing_sale_concurrency_tests.rs` | Integration | N/A (new) | ✅ Written (multi-row sale & capacity) | ✅ Passed (N tickets issued atomically) | ✅ 2 cases (exact capacity, rollback) | ✅ Clean (IMMEDIATE tx) |
| S2-T4 | `tests/ticketing_sale_concurrency_tests.rs` | Integration | ✅ 1/1 passing | ✅ Written (concurrent sales vs cap) | ✅ Passed (ceiling respected under threads) | ✅ 3 cases (single, concurrent, overflow) | ✅ Clean (thread safe) |
| S2-T5 | `tests/ticketing_sale_concurrency_tests.rs` | Integration | ✅ 2/2 passing | ✅ Written (courtesy forced 0.0/Pagado) | ✅ Passed (courtesy invariants hold) | ✅ 3 cases (courtesy, scan_token, secret) | ✅ Clean (public DTO clean) |
| S2-T6 | `tests/ticketing_door_concurrency_tests.rs` | Integration | N/A (new) | ✅ Written (door check-in 3 states) | ✅ Passed (Valida, Ya Usada, Invalida) | ✅ 3 cases (valid, double-use, unknown) | ✅ Clean (conditional update) |
| S2-T7 | `tests/ticketing_door_concurrency_tests.rs` | Integration | ✅ 1/1 passing | ✅ Written (concurrent checkin stress) | ✅ Passed (exactly 1 valid, 1 used) | ✅ 2 cases (2 threads, same ticket) | ✅ Clean (atomic swap) |
| S2-T8 | `tests/ticketing_api_tests.rs` | Integration | ✅ 26/26 passing | ✅ Written (sell, checkin, public ticket) | ✅ Passed (HTTP endpoints registered) | ✅ 3 cases (sell 201, checkin 200, public) | ✅ Clean (Axum router clean) |
| S2-T9 | `cargo test --test ticketing_door_concurrency_tests` | Integration | ✅ 2/2 passing | ✅ Written (concurrency regression) | ✅ Passed (0 race conditions) | ➖ Single (full concurrency run) | ✅ Clean (zero warnings) |
| S3-T1 | `tests/ticketing_interests_tests.rs` | Integration | N/A (new) | ✅ Written (CRUD interested & capacity) | ✅ Passed (leads created and tracked) | ✅ 2 cases (CRUD, projected aforo) | ✅ Clean (domain entity) |
| S3-T2 | `tests/ticketing_interests_tests.rs` | Integration | ✅ 1/1 passing | ✅ Written (interest domain validation) | ✅ Passed (phone, canal, promotora valid) | ✅ 3 cases (valid, bad phone, personas) | ✅ Clean (ports encapsulated) |
| S3-T3 | `tests/ticketing_interests_tests.rs` | Integration | ✅ 2/2 passing | ✅ Written (convert interest to sale) | ✅ Passed (lead confirmed + N tickets issued) | ✅ 2 cases (convert 201, reconvert 409) | ✅ Clean (shared emission tx) |
| S3-T4 | `tests/ticketing_poster_storage_tests.rs` | Integration | N/A (new) | ✅ Written (magic bytes & 10MB limit) | ✅ Passed (JPEG/PNG/WebP detected) | ✅ 4 cases (JPEG, PNG, WebP, ELF rejected) | ✅ Clean (LocalPosterStorage) |
| S3-T5 | `tests/ticketing_poster_storage_tests.rs` | Integration | ✅ 1/1 passing | ✅ Written (traversal resistance & .part) | ✅ Passed (path traversal blocked) | ✅ 3 cases (slash, backslash, dots) | ✅ Clean (safe filename check) |
| S3-T6 | `tests/ticketing_poster_storage_tests.rs` | Integration | ✅ 2/2 passing | ✅ Written (poster replacement on disk) | ✅ Passed (old poster cleaned up) | ✅ 2 cases (first upload, replacement) | ✅ Clean (atomic rename) |
| S3-T7 | `tests/ticketing_api_tests.rs` | Integration | ✅ 26/26 passing | ✅ Written (admin endpoints & poster) | ✅ Passed (poster served with ETag & cache) | ✅ 3 cases (upload, get bytes, delete) | ✅ Clean (multipart stream) |
| S3-T8 | `cargo test --test ticketing_poster_storage_tests` | Integration | ✅ 4/4 passing | ✅ Written (cascade delete regression) | ✅ Passed (DB & disk cascading clean) | ➖ Single (full storage regression) | ✅ Clean (zero orphans) |
| S4-T1 | `tests/ticketing_ressy_tests.rs` | Unit | N/A (new) | ✅ Written (asserted 16 total tools) | ✅ Passed (8 presentation + 8 ticketing) | ✅ 2 cases (count, exact tool names) | ✅ Clean (tool_schemas) |
| S4-T2 | `tests/ticketing_ressy_tests.rs` | Unit | ✅ 1/1 passing | ✅ Written (validate 8 JSON schemas) | ✅ Passed (schemas strictly match specs) | ✅ 8 cases (one per ticketing tool) | ✅ Clean (schema builders) |
| S4-T3 | `tests/ticketing_ressy_tests.rs` | Integration | ✅ 2/2 passing | ✅ Written (dispatch & invented event_id) | ✅ Passed (unauthorized event_id rejected) | ✅ 3 cases (sell, summary, checkin) | ✅ Clean (TurnToolContext) |
| S4-T4 | `tests/ticketing_ressy_tests.rs` | Integration | ✅ 3/3 passing | ✅ Written (known_event_ids in context) | ✅ Passed (context tracks active event) | ✅ 2 cases (known event, unknown event) | ✅ Clean (dispatcher decoupled) |
| S4-T5 | `tests/ticketing_ressy_tests.rs` | Unit | ✅ 4/4 passing | ✅ Written (financial rules in system prompt) | ✅ Passed (prompt grounding verified) | ➖ Single (prompt assertions) | ✅ Clean (constant prompt) |
| S4-T6 | `tests/ticketing_ressy_tests.rs` | Integration | ✅ 5/5 passing | ✅ Written (deduplication by call.id) | ✅ Passed (duplicate tool calls idempotent) | ✅ 2 cases (duplicate call.id, signature) | ✅ Clean (call_id cache) |
| S4-T7 | `cargo test --test ticketing_ressy_tests` | Integration | ✅ 6/6 passing | ✅ Written (full Ressy regression) | ✅ Passed (all 16 tools operational) | ➖ Single (full agent suite) | ✅ Clean (zero warnings) |
| S5-T1 | `frontend/test/router.test.js` | Unit | N/A (new) | ✅ Written (asserted #/eventos hash routes) | ✅ Passed (parseRoute handles 4 new routes) | ✅ 4 cases (catalog, detail, ticket, door) | ✅ Clean (pure router) |
| S5-T2 | `frontend/test/router.test.js` | Unit | ✅ 1/1 passing | ✅ Written (parameter decoding & fallback) | ✅ Passed (safe decoding, fallback presentation) | ✅ 3 cases (decode, missing, malformed) | ✅ Clean (navigate module) |
| S5-T3 | `frontend/test/events-ui.test.js` | Integration | N/A (new) | ✅ Written (topbar selector accessible) | ✅ Passed (aria-current & view toggling) | ✅ 2 cases (repertorios, eventos) | ✅ Clean (app-topbar) |
| S5-T4 | `frontend/test/events-ui.test.js` | Integration | ✅ 1/1 passing | ✅ Written (HTTP client api.js) | ✅ Passed (API methods dispatch requestJson) | ✅ 5 cases (events, summary, tickets, leads, expenses) | ✅ Clean (api module) |
| S5-T5 | `frontend/test/events-ui.test.js` | Integration | ✅ 2/2 passing | ✅ Written (catalog & 4 tabs views) | ✅ Passed (catalog, summary, tix, leads, expenses) | ✅ 4 cases (empty, loaded, tabs, badges) | ✅ Clean (views & forms) |
| S5-T6 | `frontend/test/events-ui.test.js` | Integration | ✅ 3/3 passing | ✅ Written (controller lifecycle & modal) | ✅ Passed (mount, load, destroy clean) | ✅ 3 cases (load, modal open, destroy) | ✅ Clean (controllers.js) |
| S5-T7 | `frontend/test/events-ui.test.js` | Unit | ✅ 4/4 passing | ✅ Written (scoped styles .events-*) | ✅ Passed (styles isolated with prefix) | ➖ Single (css token check) | ✅ Clean (events/index.css) |
| S5-T8 | `cargo test --test ticketing_frontend_router_tests` | Integration | ✅ 5/5 passing | ✅ Written (UI test harness in Rust) | ✅ Passed (Node test suites pass cleanly) | ➖ Single (full UI harness) | ✅ Clean (zero regressions) |
| S6-T1 | `frontend/test/whatsapp.test.js` | Unit | N/A (new) | ✅ Written (wa.me phone sanitizer) | ✅ Passed (51 prefix forced, 9 digits, encode) | ✅ 4 cases (9 digits, with 51, spaces, url encode) | ✅ Clean (pure whatsapp.js) |
| S6-T2 | `frontend/test/whatsapp.test.js` | Unit | ✅ 1/1 passing | ✅ Written (paid vs pending message copy) | ✅ Passed (correct template per payment status) | ✅ 2 cases (paid link, pending link) | ✅ Clean (message builders) |
| S6-T3 | `frontend/test/events-ui.test.js` | Integration | ✅ 4/4 passing | ✅ Written (quick-sale modal cards & links) | ✅ Passed (cards rendered with wa.me buttons) | ✅ 2 cases (single sale, multi sale) | ✅ Clean (forms modal) |
| S6-T4 | `frontend/test/public-ticket.test.js` | Unit | N/A (new) | ✅ Written (hide QR on pending ticket) | ✅ Passed (pending renders amber, no QR) | ✅ 2 cases (pending amber, paid svg) | ✅ Clean (public controller) |
| S6-T5 | `frontend/test/public-ticket.test.js` | Unit | ✅ 1/1 passing | ✅ Written (paid ticket renders SVG) | ✅ Passed (SVG embedded with no secret exposure) | ✅ 2 cases (DOM check, secret absence) | ✅ Clean (mobile view) |
| S6-T6 | `frontend/test/door-controller.test.js` | Integration | N/A (new) | ✅ Written (headless camera QR scanner) | ✅ Passed (start/stop tracks released) | ✅ 2 cases (start tracks, stop releases) | ✅ Clean (camera scanner) |
| S6-T7 | `frontend/test/door-controller.test.js` | Integration | ✅ 1/1 passing | ✅ Written (door 3 banner feedbacks) | ✅ Passed (green Valida, amber Usada, red Invalida) | ✅ 3 cases (valid, used, invalid) | ✅ Clean (door-controller) |
| S6-T8 | `cargo test --test ticketing_frontend_router_tests` | Integration | ✅ 5/5 passing | ✅ Written (full Slice 6 regression) | ✅ Passed (door mode, tickets, whatsapp passing) | ➖ Single (full slice regression) | ✅ Clean (zero clippy warnings) |
| C1-Rem | `frontend/test/events-ui.test.js` | Integration | ✅ 4/4 passing | ✅ Written (Axum envelope unpackers) | ✅ Passed (catalog, detail, tabs, summary adapted) | ✅ 5 cases ({eventos}, {evento}, {resumen}, {entradas}, {gastos}) | ✅ Clean (controllers.js normalized) |
| C2-Rem | `tests/agent-chat-integration.test.mjs` | Integration | ✅ 11/11 passing | ✅ Written (active_event_id consumption & turn2 send) | ✅ Passed (chatState persists active_event_id, sends eventId) | ✅ 3 cases (route context, response active_event_id, fallback title) | ✅ Clean (chat-state & controller) |
| C3-Rem | `tests/ticketing_api_tests.rs` | Integration | ✅ 4/4 passing | ✅ Written (JsonRejection to 400 Bad Request) | ✅ Passed (-2, wrong types, malformed json return 400) | ✅ 4 cases (-2 u32, string amount, bad checkin, bad patch) | ✅ Clean (tickets.rs handlers) |
| C4-Rem | `tests/ticketing_event_repo_tests.rs` | Integration | ✅ 5/5 passing | ✅ Written (IMMEDIATE tx & exact match rename) | ✅ Passed (casing rename rejected, concurrent tx serialized) | ✅ 3 cases (exact match, capacity floor, concurrency) | ✅ Clean (events.rs update_event) |
| C5-Rem | `tests/ticketing_poster_storage_tests.rs` | Integration | ✅ 6/6 passing | ✅ Written (reject image/jpg, cleanup_pending=true, compensation) | ✅ Passed (415 on image/jpg, cleanup_pending=true, revert on old fail) | ✅ 4 cases (image/jpg 415, discordant MIME 415, cleanup true, revert) | ✅ Clean (posters.rs & local_poster_storage) |
| C6-Rem | `tests/ticketing_event_repo_tests.rs` | Integration | ✅ 5/5 passing | ✅ Written (test_list_tickets bounds 520 rows in DB) | ✅ Passed (520 rows inserted, query returns exactly 500) | ✅ 2 cases (empty list, 520 rows truncated to 500) | ✅ Clean (tickets.rs limit constant) |
| C7-Rem | `cargo test` | Integration | ✅ 326/326 passing | ✅ Written (reconcile full 57-row strict-TDD table) | ✅ Passed (all 50 tasks + 7 remediations verified) | ➖ Single (complete protocol audit) | ✅ Clean (full documentation green) |
| Rem2-B1 | `tests/ticketing_poster_storage_tests.rs` | Integration | ✅ 8/8 passing | ✅ Written (aborts with 500 & preserves DB on FS fail) | ✅ Passed (FS-first order, aborts on FS fail, cascaded on success) | ✅ 3 cases (FS error -> 500 + DB intact, success, safe check) | ✅ Clean (propagates .trash removal, clippy clean) |
| Rem2-B2 | `tests/ticketing_event_repo_tests.rs` | Integration | ✅ 6/6 passing | ✅ Written (Barrier race + 0 omitted branches + seq order 2) | ✅ Passed (exactly one succeeds, general_cap >= emitted, both orders asserted) | ✅ 3 cases (concurrent Barrier race, seq Order 1, seq Order 2) | ✅ Clean (Barrier sync, clippy clean) |

### Test Summary
- **Total tests written / updated**: 59 (50 implementation tasks + 7 first-remediation cycles + 2 final-remediation cycles)
- **Total tests passing**: 330 automated tests in `cargo test` (0 failed, 1 ignored) + 7 Node test suites green
- **Layers used**: Unit (21), Integration (36)
- **Approval tests**: None — no refactoring tasks
- **Pure functions created**: 9 (`Money` arithmetic, `TicketType::new`, `Event::new`, `ImageFormat::detect`, `formatBytes`, `parseRoute`, `generateWhatsAppLink`, `camera-qr-scanner`, `resolveTurn active_event_id`)

---

## 3. Files Created and Modified

### Created in Remediation
- `src/handlers/ticketing/error.rs` (`TicketingError` wrapper and `ticketing_error_response` converting domain errors to HTTP 400/404/409/413/415)

### Modified in 5-Blocker Remediation (Work-Unit fix-tix-5-blockers)
- `frontend/js/core/state/chat-state.js` (Preserves `response.active_event_id` in `resolveTurn` as `{ id, title, type: "event" }`, enabling multi-turn context continuity)
- `src/handlers/ticketing/tickets.rs` (Wrapped `sell`, `update_contact`, and `check_in` with `Result<Json<T>, JsonRejection>`, returning HTTP 400 Bad Request on invalid payloads or negative quantities like -2)
- `src/infra/sqlite/ticketing_repository/events.rs` (Wrapped type check and event update in a single `rusqlite::TransactionBehavior::Immediate` transaction; enforced exact match `t.nombre == sold_tipo` prohibiting case-insensitive renames of sold ticket types; enforced capacity floor >= emitted count)
- `src/handlers/ticketing/events.rs` (Rejected `image/jpg`, enforcing closed catalog `image/jpeg`, `image/png`, `image/webp` returning 415 on mismatch or unauthorized MIME)
- `src/infra/filesystem/local_poster_storage.rs` (Propagated `fs::remove_file` errors in `delete_poster` instead of swallowing into `Ok(false)`)
- `src/use_cases/ticketing/posters.rs` (Added verified compensation: if deleting the previous poster fails during replacement, reverts DB update to original event and cleans up new poster file)
- `tests/agent-chat-integration.test.mjs` (Added test 12 verifying `active_event_id` from backend response is preserved in `chatState` and automatically sent in turn 2)
- `tests/agent-chat-state.test.mjs` (Added unit test 18 verifying `resolveTurn` with `response.active_event_id` and fallback title "Evento")
- `tests/ticketing_api_tests.rs` (Added test 1b for negative quantity -2 -> 400, test 1c for wrong types -> 400, test 4b for malformed update -> 400, test 7b for `image/jpg` -> 415, test 7c for discordant MIME -> 415, test 9b for malformed checkin -> 400)
- `tests/ticketing_event_repo_tests.rs` (Reinforced C6 test inserting 520 rows in DB and verifying 500 cap; added C4 exact match casing rename rejection test; added C4 concurrent sales vs capacity update test in immediate transaction)
- `tests/ticketing_poster_storage_tests.rs` (Added test verifying `cleanup_pending == true` when storage delete fails; added test verifying replacement compensation when deleting old poster fails)
- `openspec/changes/shirley-show-ticketing/apply-progress.md` (Completely rewritten with 8-column strict TDD evidence table for all 57 tasks)

### Modified in 2-Blocker Remediation (Work-Unit fix-tix-2-blockers)
- `src/use_cases/ticketing/events.rs`: Inverted deletion sequence to filesystem-first. Deletes on-disk poster first; if storage returns an error, aborts immediately with `AppError::Internal` (HTTP 500) leaving the database record completely intact. Runs SQLite cascade deletion only after poster asset is removed. Fixed clippy collapsible-if.
- `src/infra/filesystem/local_poster_storage.rs`: Propagates errors when removing files from `.trash` rather than swallowing the result of `fs::remove_file(&trash_target)`.
- `src/use_cases/ticketing/posters.rs`: Fixed clippy collapsible-if warning in `ReplaceEventPoster`.
- `tests/ticketing_poster_storage_tests.rs`: Replaced test asserting non-atomic deletion with `test_delete_event_aborts_with_error_and_preserves_db_when_fs_deletion_fails`, proving that filesystem failure aborts with 500 and leaves the event record in SQLite completely intact.
- `tests/ticketing_event_repo_tests.rs`: Refactored `test_update_event_concurrency_with_sales_in_immediate_tx` to use `std::sync::Barrier` to force thread interleaving, asserting `res1.is_ok() ^ res2.is_ok()`, invariant `general_cap >= emitted`, and exhaustive matching across both outcomes with zero omitted branches. Added `test_update_event_lowered_capacity_first_then_sale_exceeds_fails` testing the second order deterministically.
- `openspec/changes/shirley-show-ticketing/specs/eventos/spec.md`: Updated Requirement "Event Deletion with Cascade and Asset Cleanup" to specify filesystem-first order, HTTP 500 abort with database intact on FS failure, SQLite cascade on success, and orphan reconciliation backup.
- `openspec/changes/shirley-show-ticketing/design.md`: Updated architecture section 7 to document the filesystem-first deletion order with orphan reconciliation.

---

## 4. Workload and Delivery Boundary

- **Work unit:** `fix-tix-2-blockers`
- **Assigned slice:** Remediation of 2 final blockers identified in `verify-report.md`.
- **Authority Token:** `tvf3-begin-003` (max budget: 500 lines).
- **Reporting for Finish:** Verification commands completed, zero clippy warnings on ticketing targets and library, zero fmt errors (`cargo fmt --check` clean), 330 automated Rust tests passing, all Node integration suites passing, zero E2E vivo, zero `.run/` or `:3100` touching. Report for finish (NO settle).
