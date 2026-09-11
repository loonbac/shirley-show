# Apply Progress: shirley-show-ticketing

**Change:** `shirley-show-ticketing`  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Current Work Unit:** `tix-s4-ressy-tools` (Slice 4, PR 4 of 6 chained stacked PRs)  
**Authority Token:** `sha256:ba4fdcf41ba8b50c84c3687fd2bceb079d1f0237312b82c283d36baf50da2339`  
**Request:** `t4-acquire-001` (max budget: 3000 lines)  

---

## 1. Cumulative Completed Tasks (34/50 total)

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

---

## 2. TDD Cycle Evidence (Slice 4)

| Phase | Target | Test File / Command | Result |
|---|---|---|---|
| RED | 16 Tool Definitions & Ticketing Schemas | `cargo test --test ticketing_ressy_tests` | FAILED (`assertion left == right failed: get_tool_definitions() must expose exactly 16 tools, left: 8, right: 16`) |
| GREEN | Register 8 Strict Ticketing Tool Schemas | `cargo test --test ticketing_ressy_tests` | PASSED (`test_tool_schemas_strictly_sixteen_tools_with_exact_ticketing_schemas`: verified 16 total tools, strict `additionalProperties: false`, closed enums, positive minimums) |
| RED | Reject Invented Event ID & Unregistered Tools | `cargo test --test ticketing_ressy_tests` | FAILED (Compilation error: missing fields on `TurnToolContext` and dispatcher unknown tools) |
| GREEN | TurnToolContext & AgentToolDispatcher Dispatch | `cargo test --test ticketing_ressy_tests` | PASSED (`test_dispatcher_rejects_invented_event_id` and `test_dispatcher_eight_ticketing_tools_full_operational_lifecycle`: all 8 tools dispatched via `TicketingServices`, strict context grounding for `evento_id` and `entrada_id`, door checkin 3 states) |
| GREEN | System Prompt Directives & Ambiguity Clarification | `cargo test --test agent_chat_tests` | PASSED (Added section 18 to `SHIRLEY_SYSTEM_PROMPT` prohibiting ID hallucinations, enforcing courtesy price S/ 0.0 + Pagado, requiring parameters prior to sales, separating Cobrado from Pendiente, and reporting ticket URLs) |
| TRIANGULATE | Deduplication, Courtesy S/ 0, & URL Formatting | `cargo test --test ticketing_ressy_tests` | PASSED (`test_dispatcher_triangulate_deduplication_by_call_id`, `test_dispatcher_triangulate_courtesy_forced_zero_price_and_pagado`, `test_dispatcher_triangulate_ticket_url_formatting_with_public_base_url`) |
| REFACTOR | Clean Verification & Clippy Zero Warnings | `cargo test` + `cargo clippy --lib --tests (ticketing suites) -- -D warnings` | PASSED (All 36 test suites passed cleanly with 0 failures; zero warnings across lib and ticketing test suites under `-D warnings`) |

---

## 3. Files Created and Modified (Cumulative)

### Created in Slice 4
- `tests/ticketing_ressy_tests.rs` (Tests for 16 tool schemas, rejection of invented event IDs, full 8-tool operational dispatch lifecycle, `call.id` deduplication, Cortesía invariant enforcement, and `SHIRLEY_PUBLIC_BASE_URL` URL formatting)

### Modified in Slice 4
- `src/agent/tool_schemas.rs` (Registered schemas for `crear_evento`, `listar_eventos`, `vender_entradas`, `confirmar_pago_entrada`, `registrar_gasto`, `resumen_evento`, `registrar_interesado`, `checkin_entrada`)
- `src/agent/tool_dispatcher.rs` (Integrated `TicketingServices`, added helper functions `is_authorized_event`, `format_ticket_url`, `format_whatsapp_url`, `app_error_to_json`, and implemented dispatch for the eight ticketing tools)
- `src/agent/turn_context.rs` (Added `known_event_ids`, `known_ticket_ids`, `active_event_id`, and `with_event_context`)
- `src/agent/system_prompt.rs` (Added Section 18 for ticketing and financial rules to `SHIRLEY_SYSTEM_PROMPT`, and added `active_event_id` grounding to `build_context_system_prompt`)
- `src/domain/agent_tool.rs` (Added optional `event_id` to `ToolAction`)
- `src/use_cases/agent_chat.rs` (Added `event_id` to `AgentChatInput`, `active_event_id` to `AgentChatOutput`, and wired `active_event_id` to turn context)
- `src/handlers/agent/json_request.rs` (Added `event_id` parsing and length validation to JSON agent chat requests)
- `src/handlers/agent/multipart_request.rs` (Added `event_id` parsing and length validation to multipart agent chat requests)
- `src/handlers/agent/chat.rs` (Wired `active_event_id: None` on timeout responses)
- `src/state/app_state.rs` (Wired `TicketingServices` and `SHIRLEY_PUBLIC_BASE_URL` into `AgentToolDispatcher` on startup)
- `tests/agent_tools_tests.rs` (Updated tool count assertion from 8 to 16)
- `tests/agent_chat_tests.rs` & `tests/agent_chat_remote_ux_tests.rs` (Updated `AgentChatInput` initializers with `event_id: None`)
- `openspec/changes/shirley-show-ticketing/tasks.md` (Marked all 7 tasks for Slice 4 complete, now 34/50 total)

---

## 4. Deviations from Design

None. The implementation strictly fulfills `design.md`, `specs/ressy-ticketing/spec.md`, and `tasks.md`:
- Exactly 16 tools exposed by `get_tool_definitions()` (8 for presentations, 8 for ticketing).
- All 8 ticketing tools use strict JSON schemas with `additionalProperties: false`, closed string enums (`Yape` | `Efectivo` | `Cortesía`, `Pagado` | `Pendiente`, closed 7 expense categories), and positive integer minimums.
- `TurnToolContext` strictly checks that `evento_id` exists in the active turn context or was returned by `listar_eventos` before dispatching mutating operations or summaries, rejecting invented IDs with `unauthorized_event_id`.
- Mutating tools are deduplicated by `call.id` and canonical signature; read tools (`listar_eventos`, `resumen_evento`) return cached values.
- Cortesía entries are unconditionally enforced at price S/ 0.0 and status `Pagado`.
- `SHIRLEY_PUBLIC_BASE_URL` is parsed with origin validation and trailing slash normalization; relative path `/#/ticket/{scan_token}` is used when unconfigured.

---

## 5. Workload and PR Boundary

- **Work unit:** `tix-s4-ressy-tools`
- **Assigned slice:** Slice 4 (PR 4 of 6 chained stacked PRs)
- **Review forecast vs Authored lines:** The integration of 8 conversational AI tools with strict schemas, parameter validation, transactional use-case dispatching, context grounding, prompt directives, deduplication, URL generation, and comprehensive hermetic tests comprises ~1,200 lines across 16 files. This slice is a cohesive unit enabling AI operations on ticketing and cannot be broken down further without leaving schemas decoupled from dispatchers and prompt directives. A `size:exception` applies to PR 4 under the authorized chained delivery strategy.
- **Authority token for settle:** `sha256:ba4fdcf41ba8b50c84c3687fd2bceb079d1f0237312b82c283d36baf50da2339`, request `t4-acquire-001`, max 3000. Settle was NOT executed per instruction ("Reporta para settle (NO settle)").

---

## 6. Remaining Tasks (16 remaining across Slices 5–6)

### Slice 5: Frontend UI - Navegación Topbar, Catálogo de Eventos, Detalle y Pestañas (7 remaining)
- [ ] RED: Escribir tests Node en `frontend/test/router.test.js` (o suite equivalente invocada vía Rust) para las rutas `#/eventos`, `#/eventos/:id`, `#/ticket/:token`, `#/puerta/:id` y fallback a repertorios. <!-- sdd-owner: implementation -->
- [ ] GREEN: Actualizar `frontend/js/router/parse-route.js` y `frontend/js/router/navigate.js` con las nuevas rutas y decodificación segura de parámetros. <!-- sdd-owner: implementation -->
- [ ] GREEN: Modificar `frontend/js/components/app-topbar.js` para añadir el selector accesible `Repertorios | Eventos` y actualizar el composition root en `frontend/js/main.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar cliente HTTP de ticketing en `frontend/js/modules/events/api.js` cubriendo llamadas a eventos, resumen, entradas, interesados y gastos. <!-- sdd-owner: implementation -->
- [ ] GREEN: Construir vistas presentacionales y formularios de catálogo (`#/eventos`) y detalle con pestañas (`#/eventos/:id`) en `frontend/js/modules/events/views.js` y `forms.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `controllers.js` en `frontend/js/modules/events/controllers.js` para gestionar ciclo de vida, estados de carga/vacío/error y montaje de pestañas. <!-- sdd-owner: implementation -->
- [ ] GREEN: Agregar estilos cohesivos con prefijo `.events-*` en `frontend/styles/events/index.css` e importarlos en `frontend/styles/index.css`. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Probar en navegador que `#/presentations` siga siendo la vista inicial por defecto y que alternar a `#/eventos` cargue catálogo sin errores de consola. <!-- sdd-owner: implementation -->

### Slice 6: Frontend UI - Venta Rápida, Enlaces wa.me, Vista Pública de Ticket y Modo Puerta (8 remaining)
- [ ] RED: Escribir tests unitarios para `whatsapp.js` en `frontend/test/whatsapp.test.js` verificando saneamiento de números, código de país 51 y URLs con encoding seguro para pagado y pendiente. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar función pura de generación de enlaces en `frontend/js/modules/events/whatsapp.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar modal y controlador de "Venta Rápida" en `frontend/js/modules/events/forms.js` y `controllers.js`, mostrando tarjetas con enlaces `wa.me` por cada entrada emitida. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests para el renderizado de ticket público (verificando que estado `Pendiente` no renderice contenedor ni imagen de QR) en `frontend/test/public-ticket.test.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar vista y controlador de ticket público en `frontend/js/modules/events/public-ticket-controller.js` renderizando `qr_svg` o banner ámbar según pago. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar módulo headless `camera-qr-scanner.js` con ciclo de vida seguro (`start()`, `stop()`, liberación de tracks) y fallback a entrada manual. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar vista y controlador de modo puerta en `frontend/js/modules/events/door-controller.js` con feedback visual inequívoco (verde Válida, ámbar Ya Usada, rojo Inválida) y autofocus. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ejecutar `cargo test` y comprobaciones completas (`cargo clippy --all-targets --all-features -- -D warnings`), verificando el flujo de venta y validación en puerta de inicio a fin. <!-- sdd-owner: implementation -->
