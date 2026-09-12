# Apply Progress: shirley-show-ticketing

**Change:** `shirley-show-ticketing`  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Current Work Unit:** `tix-s6-door-public-wa` (Slice 6, PR 6 of 6 chained stacked PRs - FINAL SLICE)  
**Authority Token:** `sha256:19b4b7ab2a2760b488e48ccb7b0164b2641b3bbc4cf28db8aa62da3c36da10e3`  
**Request:** `t6-acquire-001` (max budget: 3000 lines)  

---

## 1. Cumulative Completed Tasks (50/50 total - ALL IMPLEMENTATION TASKS COMPLETE)

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
- [x] RED: Escribir tests Node en `frontend/test/router.test.js` (o suite equivalente invocada vía Rust) para las rutas `#/eventos`, `#/eventos/:id`, `#/ticket/:token`, `#/puerta/:id` y fallback a repertorios. <!-- sdd-owner: implementation -->
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

---

## 2. TDD Cycle Evidence (Slice 6)

| Phase | Target | Test File / Command | Result |
|---|---|---|---|
| RED | WhatsApp Pure Transformation & Phone Sanitization | `frontend/test/whatsapp.test.js` | FAILED (`ERR_MODULE_NOT_FOUND: Cannot find module whatsapp.js`) |
| GREEN | WhatsApp Link Generation Pure Module | `frontend/js/modules/events/whatsapp.js` | PASSED (`node frontend/test/whatsapp.test.js`: verified 9-digit prefixing, non-digit stripping, URLs for Pagado and Pendiente with price and payment coordination) |
| GREEN | Quick Sale Modal & Result Cards with WhatsApp Links | `frontend/js/modules/events/forms.js`, `controllers.js` | PASSED (`frontend/test/events-ui.test.js`: single-option preselection, courtesy forced to Pagado and disabled status, cards rendered with wa.me link and public ticket link for each emitted ticket) |
| RED | Public Ticket QR Concealment for Pending Tickets | `frontend/test/public-ticket.test.js` | FAILED (`ERR_MODULE_NOT_FOUND: Cannot find module public-ticket-controller.js`) |
| GREEN | Public Ticket Mobile View & Controller | `frontend/js/modules/events/public-ticket-controller.js` | PASSED (`node frontend/test/public-ticket.test.js`: verified prominent vector QR SVG for Pagado, and strict non-existence of QR container or SVG when Pendiente, with amber warning banner and payment instructions) |
| GREEN | Headless Camera QR Scanner with Safe Lifecycle | `frontend/js/modules/events/camera-qr-scanner.js` | PASSED (`node frontend/test/door-controller.test.js`: verified BarcodeDetector detection, stream play, scan loop, and guaranteed stop() track release) |
| GREEN | Door Mode Controller with 3-State Feedback & Autofocus | `frontend/js/modules/events/door-controller.js` | PASSED (`node frontend/test/door-controller.test.js`: verified giant green Válida banner with attendee/type/time, amber Ya Usada banner with prior entry time, red Inválida banner with reason, input autofocus recovery, and desktop notice) |
| REFACTOR | Full Regression & Clippy Verification | `cargo test` + `cargo clippy --lib --test ticketing_* -- -D warnings` | PASSED (All 37 test suites passed with 0 failures; all 5 frontend Node tests passed; 0 clippy warnings on ticketing targets) |

---

## 3. Files Created and Modified in Slice 6

### Created in Slice 6
- `frontend/js/modules/events/whatsapp.js` (Pure transformations for phone sanitization, 51 country code, ticket public URLs, and wa.me message construction)
- `frontend/js/modules/events/public-ticket-controller.js` (Mobile attendee public ticket view and controller with conditional QR SVG rendering vs amber pending banner)
- `frontend/js/modules/events/camera-qr-scanner.js` (Headless camera scanner using native BarcodeDetector API with graceful fallback and media track release)
- `frontend/js/modules/events/door-controller.js` (Mobile door mode access control controller with always-visible manual input, autofocus recovery, and 3-state banners)
- `frontend/test/whatsapp.test.js` (Unit tests for phone cleaning, URL construction, and safe message encoding)
- `frontend/test/public-ticket.test.js` (Tests for public ticket rendering, verified QR vector rendering for paid, and strict concealment of QR for pending)
- `frontend/test/door-controller.test.js` (Tests for camera scanner lifecycle and 3 door check-in states: Válida, Ya Usada, Inválida)

### Modified in Slice 6
- `frontend/js/modules/events/forms.js` (Updated `createSellTicketDialog` / `createQuickSaleDialog` with single-option preselection rule, courtesy invariants, and post-sale cards with wa.me links)
- `frontend/js/modules/events/controllers.js` (Wired `handleSellTickets` with event name and returned tickets, and wired `onTicketWhatsApp` handler)
- `frontend/js/main.js` (Wired `public-ticket` (`#/ticket/:token`) and `door` (`#/puerta/:id`) routes to their controllers)
- `frontend/js/utils/dom/create-element.js` (Hardened style property assignment for fake-dom compatibility)
- `frontend/styles/events/index.css` (Added scoped styling for quick sale ticket cards, public ticket mobile view, and door mode banners)
- `frontend/test/events-ui.test.js` (Added integration tests for quick sale modal and issued ticket cards)
- `tests/fake-dom.mjs` (Hardened `innerHTML = ''` unlinking child nodes)
- `tests/ticketing_frontend_router_tests.rs` (Added tests for whatsapp, public ticket, and door controller)
- `openspec/changes/shirley-show-ticketing/tasks.md` (Marked all 8 tasks in Slice 6 complete; now 50/50 total)

---

## 4. Deviations from Design

None. The implementation strictly fulfills `design.md`, `specs/entradas/spec.md`, `specs/checkin/spec.md`, and `specs/entrada-publica/spec.md`:
- Pure `whatsapp.js` cleanly sanitizes Peruvian 9-digit phone numbers prepending `51` without server-side third-party messaging daemons.
- Public ticket view (`#/ticket/:scan_token`) consumes exclusively the unauthenticated public ticket DTO, rendering vector QR SVG only when `Pagado`, and strictly omitting QR graphic/container when `Pendiente`, displaying the amber reservation banner and instructions instead.
- Camera scanner encapsulates `BarcodeDetector` headless behavior, releasing media tracks on stop.
- Door mode (`#/puerta/:id`) maintains manual input always visible, recovers autofocus after every submission, presents giant visual feedback (green Válida, amber Ya Usada, red Inválida), and advises using a mobile device on wide screens.

---

## 5. Workload and PR Boundary

- **Work unit:** `tix-s6-door-public-wa`
- **Assigned slice:** Slice 6 (PR 6 of 6 chained stacked PRs - FINAL SLICE)
- **Review forecast vs Authored lines:** Slice 6 implements the complete public attendee experience, WhatsApp sharing, camera QR scanning, and door access control (~850 lines across 17 files). As the final slice in the 6-PR stack, all 50 implementation tasks of `shirley-show-ticketing` are now 100% complete. A `size:exception` applies under the approved chained delivery plan.
- **Authority token for settle:** `sha256:19b4b7ab2a2760b488e48ccb7b0164b2641b3bbc4cf28db8aa62da3c36da10e3`, request `t6-acquire-001`, max 3000 lines. Settle was NOT executed per instruction ("Reporta para settle (NO settle)").

---

## 6. Remaining Tasks (0 remaining - 50/50 Complete)

All implementation tasks across Slices 1 through 6 are marked `[x]` in `tasks.md`.  
Next recommended phase: `sdd-verify`.
