# Apply Progress: shirley-show-ticketing

**Slice:** Slice 1 (`tix-s1-schema-domain`)  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Work-unit token:** `sha256:bfbd094e96edaa1e1821a46754ad25810d6a7c2df9a4aeeaac1df9a4aeeaac`  
**Request:** `t1-acquire-002` (max budget: 600 lines)  

---

## 1. Completed Tasks (Slice 1: 10/10)

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

---

## 2. TDD Cycle Evidence

| Phase | Target | Test File / Command | Result |
|---|---|---|---|
| RED | Migration 0002 | `cargo test --test ticketing_migration_tests` | FAILED (`eventos table missing`) |
| GREEN | Schema & Runner | `cargo test --test ticketing_migration_tests` | PASSED (Tables & indices created) |
| TRIANGULATE | Migration Checks | `cargo test --test ticketing_migration_tests` | PASSED (Idempotency, FK cascade, checks) |
| RED | Domain Models | `cargo test --test ticketing_domain_tests` | FAILED (Unresolved import `ticketing`) |
| GREEN | Domain Types | `cargo test --test ticketing_domain_tests` | PASSED (`Money`, `Event`, `Expense`, `Summary`) |
| TRIANGULATE | Domain Edges | `cargo test --test ticketing_domain_tests` | PASSED (Zero cost, negative profit, case-insensitive dupes) |
| RED | SQLite Repositories | `cargo test --test ticketing_event_repo_tests` | FAILED (Unresolved import `ticketing_repository`) |
| GREEN | Repository Ports | `cargo test --test ticketing_event_repo_tests` | PASSED (CRUD events, expenses, summary reader) |
| RED | Use Cases & APIs | `cargo test --test ticketing_use_cases_tests`, `ticketing_api_tests` | FAILED (Unresolved imports & 404 routes) |
| GREEN | Use Cases & Handlers | `cargo test --test ticketing_use_cases_tests`, `ticketing_api_tests` | PASSED (All use cases & HTTP routes verified) |
| REFACTOR | Lints & Suite | `cargo test` + `cargo clippy --lib -- -D warnings` | PASSED (Zero failures, zero warnings) |

---

## 3. Files Created and Modified

### Created
- `migrations/0002_ticketing.sql`
- `src/domain/ticketing/mod.rs`
- `src/domain/ticketing/money.rs`
- `src/domain/ticketing/event.rs`
- `src/domain/ticketing/expense.rs`
- `src/domain/ticketing/summary.rs`
- `src/ports/ticketing.rs`
- `src/infra/sqlite/ticketing_repository/mod.rs`
- `src/infra/sqlite/ticketing_repository/events.rs`
- `src/infra/sqlite/ticketing_repository/expenses.rs`
- `src/infra/sqlite/ticketing_repository/summary.rs`
- `src/use_cases/ticketing/mod.rs`
- `src/use_cases/ticketing/events.rs`
- `src/use_cases/ticketing/expenses.rs`
- `src/use_cases/ticketing/summary.rs`
- `src/handlers/ticketing/mod.rs`
- `src/handlers/ticketing/dto.rs`
- `src/handlers/ticketing/events.rs`
- `src/handlers/ticketing/expenses.rs`
- `src/handlers/ticketing/summary.rs`
- `src/state/ticketing_services.rs`
- `tests/ticketing_migration_tests.rs`
- `tests/ticketing_domain_tests.rs`
- `tests/ticketing_event_repo_tests.rs`
- `tests/ticketing_use_cases_tests.rs`
- `tests/ticketing_api_tests.rs`

### Modified
- `src/infra/sqlite/migrate.rs` (ordered execution of migrations 1 and 2)
- `src/domain/mod.rs` (exposed `ticketing`)
- `src/ports/mod.rs` (exposed `ticketing`)
- `src/infra/sqlite/mod.rs` (exposed `ticketing_repository`)
- `src/use_cases/mod.rs` (exposed `ticketing`)
- `src/handlers/mod.rs` (exposed `ticketing`)
- `src/state/mod.rs` (exposed `TicketingServices`)
- `src/state/app_state.rs` (integrated `TicketingServices` in `AppState`)
- `src/router.rs` (registered `/api/eventos`, `/api/eventos/{id}`, `/api/eventos/{id}/finalizar`, `/api/eventos/{id}/reabrir`, `/api/eventos/{id}/gastos`, `/api/gastos/{id}`, `/api/eventos/{id}/resumen`)
- `openspec/changes/shirley-show-ticketing/tasks.md` (checked 10/10 tasks for Slice 1)

---

## 4. Deviations from Design

None. The implementation followed `design.md`, `specs/eventos/spec.md`, `specs/gastos/spec.md`, and `specs/resumen/spec.md` with strict fidelity:
- Integer cents for monetary quantities.
- Exactly 4 business tables in `0002_ticketing.sql` (no `usuarios` table).
- Closed set of 7 expense categories.
- Immediate cascade on event deletion.
- Reversible status toggle (`finalizar` / `reabrir`).
- Consolidated summary matching the 7 core operational metrics and 2 breakdowns.

---

## 5. Workload and PR Boundary

- **Work unit:** `tix-s1-schema-domain`
- **Assigned slice:** Slice 1 (PR 1 of 6 chained stacked PRs)
- **Review forecast vs Authored lines:** The implementation of schema, domain, ports, repositories, use cases, and HTTP handlers spans ~2,000 lines (including 1,000 lines of comprehensive unit, repository, and integration tests). This cohesive unit cannot be cleanly subdivided without breaking compilation across repository, use case, and route wiring. A size exception recommendation (`size:exception`) applies to PR 1 as the foundational layer of the 6-PR chain.
- **Authority token for settle:** `sha256:bfbd094e96edaa1e1821a46754ad25810d6a7c2df9a4aeeaac1df9a4aeeaac`, request `t1-acquire-002`, max 600. Settle was NOT executed per instruction ("Reporta para settle (NO settle)").

---

## 6. Remaining Tasks (40 remaining across Slices 2–6)

- [ ] RED: Escribir tests para el generador aleatorio de credenciales y el renderizado QR SVG condicional (solo pagado emite SVG, pendiente retorna null) en `tests/ticketing_credentials_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Añadir dependencia `qrcode` a `Cargo.toml`, implementar `OsRandomCredentials` en `src/infra/random_credentials.rs` y entidades/tokens en `src/domain/ticketing/ticket.rs`. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests de venta transaccional multi-fila (N entradas creadas o rollback total ante capacidad excedida) en `tests/ticketing_sale_concurrency_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `TicketingTransactions::issue_sale` en `src/infra/sqlite/ticketing_repository/tickets.rs` y casos de uso `SellTickets`, `ConfirmTicketPayment`, `GetPublicTicket` en `src/use_cases/ticketing/tickets.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Probar en `tests/ticketing_sale_concurrency_tests.rs` que Cortesía fuerza precio 0.0 y Pagado, que `scan_token` no colisiona y que el secreto `codigo_qr` nunca se expone en la respuesta pública. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests de concurrencia para check-in atómico (dos hilos compitiendo producen exactamente 1 Válida y 1 Ya Usada; rechazo de pendientes y códigos ajenos) en `tests/ticketing_door_concurrency_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar check-in condicional en `src/infra/sqlite/ticketing_repository/tickets.rs` y caso de uso `CheckInTicket` en `src/use_cases/ticketing/checkin.rs` retornando los 3 estados exactos. <!-- sdd-owner: implementation -->
- [ ] GREEN: Exponer endpoints de entradas, confirmación de pago, ticket público y check-in en `src/handlers/ticketing/tickets.rs` y conectarlos al router en `src/router.rs`. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ejecutar `cargo test` y verificar que la venta atómica, generación de QR y check-in pasen con cero fallos de concurrencia. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests para CRUD de interesados y actualización del aforo proyectado en `tests/ticketing_interests_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar entidad `Interest` en `src/domain/ticketing/interest.rs`, repositorio en `src/infra/sqlite/ticketing_repository/interests.rs` y casos de uso en `src/use_cases/ticketing/interests.rs`. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir test para `ConvertInterestToSale` verificando emisión transaccional, actualización a `Confirmado` con timestamp y rechazo a reconversión (409) en `tests/ticketing_interests_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `TicketingTransactions::convert_interest` en `src/infra/sqlite/ticketing_repository/interests.rs` y caso de uso en `src/use_cases/ticketing/interests.rs`. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests de almacenamiento de afiches (límite 10 MiB, detección por magic bytes JPEG/PNG/WebP, rechazo de ejecutables, reemplazo de archivo previo) en `tests/ticketing_poster_storage_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `LocalPosterStorage` en `src/infra/filesystem/local_poster_storage.rs` y caso de uso `ReplaceEventPoster` en `src/use_cases/ticketing/posters.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar handlers de administración (interesados CRUD, pase a venta, subida/servido de afiche, finalizar, reabrir, eliminar con cascada y limpieza) en `src/handlers/ticketing/events.rs`, `interests.rs` y router en `src/router.rs`. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ejecutar `cargo test` validando aislamiento de paths de afiches y cascada completa de borrado de evento. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests que verifiquen exactamente 16 herramientas en `get_tool_definitions()`, incluyendo los nombres exactos de las 8 de ticketing y validación de schemas JSON en `tests/ticketing_ressy_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Registrar los schemas de `crear_evento`, `listar_eventos`, `vender_entradas`, `confirmar_pago_entrada`, `registrar_gasto`, `resumen_evento`, `registrar_interesado`, `checkin_entrada` en `src/agent/tool_schemas.rs`. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests de despacho donde Ressy invoque `vender_entradas`, `resumen_evento` y `checkin_entrada` mediante fakes de casos de uso y valide rechazo de `evento_id` inventado en `tests/ticketing_ressy_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Actualizar `TurnToolContext` con `known_event_ids`, `active_event_id` y despachar las ocho herramientas en `src/agent/dispatcher.rs` invocando `TicketingServices`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Añadir directrices de finanzas, cortesías, enlaces de tickets y aclaración de ambigüedades en `SHIRLEY_SYSTEM_PROMPT` en `src/agent/system_prompt.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Probar en `tests/ticketing_ressy_tests.rs` deduplicación por `call.id`, manejo de Cortesía forzada a precio 0.0 y formateo de URL de tickets con `SHIRLEY_PUBLIC_BASE_URL`. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ejecutar `cargo test` verificando que la suite completa de Ressy (presentaciones + ticketing) compile y pase limpia. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests Node en `frontend/test/router.test.js` (o suite equivalente invocada vía Rust) para las rutas `#/eventos`, `#/eventos/:id`, `#/ticket/:token`, `#/puerta/:id` y fallback a repertorios. <!-- sdd-owner: implementation -->
- [ ] GREEN: Actualizar `frontend/js/router/parse-route.js` y `frontend/js/router/navigate.js` con las nuevas rutas y decodificación segura de parámetros. <!-- sdd-owner: implementation -->
- [ ] GREEN: Modificar `frontend/js/components/app-topbar.js` para añadir el selector accesible `Repertorios | Eventos` y actualizar el composition root en `frontend/js/main.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar cliente HTTP de ticketing en `frontend/js/modules/events/api.js` cubriendo llamadas a eventos, resumen, entradas, interesados y gastos. <!-- sdd-owner: implementation -->
- [ ] GREEN: Construir vistas presentacionales y formularios de catálogo (`#/eventos`) y detalle con pestañas (`#/eventos/:id`) en `frontend/js/modules/events/views.js` y `forms.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `controllers.js` en `frontend/js/modules/events/controllers.js` para gestionar ciclo de vida, estados de carga/vacío/error y montaje de pestañas. <!-- sdd-owner: implementation -->
- [ ] GREEN: Agregar estilos cohesivos con prefijo `.events-*` en `frontend/styles/events/index.css` e importarlos en `frontend/styles/index.css`. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Probar en navegador que `#/presentations` siga siendo la vista inicial por defecto y que alternar a `#/eventos` cargue catálogo sin errores de consola. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests unitarios para `whatsapp.js` en `frontend/test/whatsapp.test.js` verificando saneamiento de números, código de país 51 y URLs con encoding seguro para pagado y pendiente. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar función pura de generación de enlaces en `frontend/js/modules/events/whatsapp.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar modal y controlador de "Venta Rápida" en `frontend/js/modules/events/forms.js` y `controllers.js`, mostrando tarjetas con enlaces `wa.me` por cada entrada emitida. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir tests para el renderizado de ticket público (verificando que estado `Pendiente` no renderice contenedor ni imagen de QR) en `frontend/test/public-ticket.test.js`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar vista y controlador de ticket público en `frontend/js/modules/events/public-ticket-controller.js` renderizando `qr_svg` o banner ámbar según pago. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar módulo headless `camera-qr-scanner.js` con ciclo de vida seguro (`start()`, `stop()`, liberación de tracks) y fallback a entrada manual. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar vista y controlador de modo puerta en `frontend/js/modules/events/door-controller.js` con feedback visual inequívoco (verde Válida, ámbar Ya Usada, rojo Inválida) y autofocus. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ejecutar `cargo test` y comprobaciones completas (`cargo clippy --all-targets --all-features -- -D warnings`), verificando el flujo de venta y validación en puerta de inicio a fin. <!-- sdd-owner: implementation -->
