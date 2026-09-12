# Apply Progress: shirley-show-ticketing

**Change:** `shirley-show-ticketing`  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Current Work Unit:** `tix-s5-events-ui` (Slice 5, PR 5 of 6 chained stacked PRs)  
**Authority Token:** `sha256:86f746e7cc8cd6266bd290b9f9d16654aab669b6dd00f6dfe9be4cb6c929d3e7`  
**Request:** `t5-acquire-001` (max budget: 3000 lines)  

---

## 1. Cumulative Completed Tasks (42/50 total)

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

---

## 2. TDD Cycle Evidence (Slice 5)

| Phase | Target | Test File / Command | Result |
|---|---|---|---|
| RED | Ticketing Routes & Safe Decoding Fallbacks | `frontend/test/router.test.js` via `cargo test --test ticketing_frontend_router_tests` | FAILED (`AssertionError: Expected actual { view: 'presentations', params: {}, fallback: true } to equal { view: 'events', params: {} }`) |
| GREEN | Hash Router & Safe Param Parsing | `frontend/js/router/parse-route.js` + `navigate.js` | PASSED (`test_router_unit_tests`: verified `#/eventos`, `#/eventos/:id`, `#/ticket/:token`, `#/puerta/:id`, URIError fallbacks, navigation helpers) |
| GREEN | Accessible Topbar Section Selector & Composition Root | `frontend/js/components/organisms/app-topbar.js`, `frontend/js/components/app-topbar.js`, `frontend/js/main.js` | PASSED (`test_events_ui_integration_tests`: verified accessible button group `Repertorios | Eventos` with `aria-current="page"`, clean routing transitions, controller teardown) |
| GREEN | Ticketing API HTTP Client | `frontend/js/modules/events/api.js` | PASSED (Full client covering events CRUD, finalize, reopen, poster upload, summary, tickets sales, payment confirmation, door checkin, public ticket, interests CRUD/conversion, and expenses CRUD) |
| GREEN | Catalog & Detail Presentational Views and Modals | `frontend/js/modules/events/views.js` + `forms.js` | PASSED (`test_events_ui_integration_tests`: verified 7 financial & capacity metrics, overcapacity banner when aforo > total capacity, payment medium table, category breakdown, cards, and modal lifecycle) |
| GREEN | Events Controllers (Catalog & Detail Lifecycle) | `frontend/js/modules/events/controllers.js` | PASSED (`createEventsCatalogController` and `createEventDetailController`: verified data loading, tab switching, agent context grounding, and modal dismissals) |
| GREEN | Cohesive Warm Scoped Styles (`.events-*`) | `frontend/styles/events/index.css` + `frontend/styles/index.css` | PASSED (Mobile-first responsive grid, warm Shirley ivory/gold tokens, badges, metrics cards, hero poster, dialogs) |
| REFACTOR | Regression Verification & Default Initial Route | `cargo test` + `cargo clippy --lib --test ticketing_frontend_router_tests -- -D warnings` | PASSED (All 37 test suites passed cleanly with 0 failures; verified `#/presentations` as default initial route and clean transitions) |

---

## 3. Files Created and Modified (Cumulative)

### Created in Slice 5
- `frontend/test/router.test.js` (Unit tests for routing `#//presentations`, `#/eventos`, `#/eventos/:id`, `#/ticket/:token`, `#/puerta/:id`, and error fallbacks)
- `frontend/test/events-ui.test.js` (Integration tests for topbar selector, catalog view, detail view, 7 metrics, tabs, and modal dialogs)
- `tests/ticketing_frontend_router_tests.rs` (Rust test harness executing Node router and UI tests)
- `frontend/js/components/app-topbar.js` (Bridge export for topbar organism)
- `frontend/js/modules/events/api.js` (Cohesive HTTP client for events, summary, tickets, leads, and expenses)
- `frontend/js/modules/events/forms.js` (Modal dialogs for event creation/editing, expense recording, lead management, ticket selling, and conversion)
- `frontend/js/modules/events/views.js` (Catalog view with upcoming/finalized sections and detail view with 7 metrics and 4 tabs)
- `frontend/js/modules/events/controllers.js` (Orchestration controllers for catalog and detail with state, tabs, and chat grounding)
- `frontend/styles/events/index.css` (Scoped CSS styles for `.events-*` components and dialogs)

### Modified in Slice 5
- `frontend/js/router/parse-route.js` (Added safe URI decoding and route resolution for `events`, `event-detail`, `public-ticket`, and `door`)
- `frontend/js/router/navigate.js` (Added `toEvents`, `toEvent`, `toPublicTicket`, and `toDoor`)
- `frontend/js/components/organisms/app-topbar.js` (Added accessible section selector `Repertorios | Eventos` with `activeSection` and `onSectionChange`)
- `frontend/styles/organisms/app-topbar.css` (Added styling for `.app-topbar-center`, `.app-topbar-nav`, and `.app-topbar-nav-btn`)
- `frontend/js/controllers/presentations-controller.js` (Added `activeSection: 'presentations'` and navigation on section change)
- `frontend/js/controllers/presentation-detail-controller.js` (Added `activeSection: 'presentations'` and navigation on section change)
- `frontend/js/main.js` (Wired `events` and `event-detail` routes to ticketing controllers, controller teardown, and agent context)
- `frontend/styles/index.css` (Imported `./events/index.css`)
- `openspec/changes/shirley-show-ticketing/tasks.md` (Marked all 8 tasks for Slice 5 complete, now 42/50 total)

---

## 4. Deviations from Design

None. The implementation strictly fulfills `design.md`, `specs/eventos/spec.md`, `specs/resumen/spec.md`, `specs/gastos/spec.md`, and `specs/interesados/spec.md`:
- `parse-route.js` recognizes routes in the exact prioritized order specified in design, safely handling malformed URI components.
- Topbar navigation provides an accessible `Repertorios | Eventos` selector with `aria-current="page"`, positioning repertories first by default.
- Events catalog cleanly separates active and finalized events, with responsive cards, poster previews, and creation modals.
- Detail view renders the 7 production-parity financial and capacity metrics, alerts when projected attendance exceeds venue capacity, and breaks down payment methods and expenses by category.
- Tabs (Resumen, Entradas, Interesados, Gastos) provide complete administrative control, transactional lead conversion, and audio stop on entry.
- Agent chat context is grounded with `{ id, title, type: 'event' }` when viewing event detail.

---

## 5. Workload and PR Boundary

- **Work unit:** `tix-s5-events-ui`
- **Assigned slice:** Slice 5 (PR 5 of 6 chained stacked PRs)
- **Review forecast vs Authored lines:** The integration of the hash router, topbar selector, HTTP client, catalog views, detail tab panels, 7 metrics display, modal dialogs, and comprehensive tests comprises ~1,100 lines across 16 files. As with previous slices, this is a cohesive frontend capability unit that delivers the complete ticketing management interface and cannot be split further without separating views from their controllers and styles. A `size:exception` applies to PR 5 under the authorized chained delivery strategy.
- **Authority token for settle:** `sha256:86f746e7cc8cd6266bd290b9f9d16654aab669b6dd00f6dfe9be4cb6c929d3e7`, request `t5-acquire-001`, max 3000. Settle was NOT executed per instruction ("Reporta para settle (NO settle)").

---

## 6. Remaining Tasks (8 remaining in Slice 6)

### Slice 6: Frontend UI - Venta Rápida, Enlaces wa.me, Vista Pública de Ticket y Modo Puerta (8 remaining)
- [ ] RED: Escribir tests unitarios para `whatsapp.js` en `frontend/test/whatsapp.test.js` verificando saneamiento de números, código de país 51 y URLs con encoding seguro para pagado y pendiente. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar función pura de generación de enlaces en `frontend/js/modules/events/whatsapp.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar modal y controlador de "Venta Rápida" en `frontend/js/modules/events/forms.js` y `controllers.js`, mostrando tarjetas con enlaces `wa.me` por cada entrada emitida. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests para el renderizado de ticket público (verificando que estado `Pendiente` no renderice contenedor ni imagen de QR) en `frontend/test/public-ticket.test.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar vista y controlador de ticket público en `frontend/js/modules/events/public-ticket-controller.js` renderizando `qr_svg` o banner ámbar según pago. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar módulo headless `camera-qr-scanner.js` con ciclo de vida seguro (`start()`, `stop()`, liberación de tracks) y fallback a entrada manual. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar vista y controlador de modo puerta en `frontend/js/modules/events/door-controller.js` con feedback visual inequívoco (verde Válida, ámbar Ya Usada, rojo Inválida) y autofocus. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ejecutar `cargo test` y comprobaciones completas (`cargo clippy --all-targets --all-features -- -D warnings`), verificando el flujo de venta y validación en puerta de inicio a fin. <!-- sdd-owner: implementation -->
