# Apply Progress: shirley-show-ticketing

**Change:** `shirley-show-ticketing`  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Current Work Unit:** `tix-s2-tickets-qr-checkin` (Slice 2, PR 2 of 6 chained stacked PRs)  
**Authority Token:** `sha256:6d379bb138d0c43abe5ee94b4157cd66604829736f9d65228f9be47db08453fe`  
**Request:** `t2-acquire-003` (max budget: 3000 lines)  

---

## 1. Cumulative Completed Tasks (19/50 total)

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

---

## 2. TDD Cycle Evidence (Slice 2)

| Phase | Target | Test File / Command | Result |
|---|---|---|---|
| RED | Credentials & QR SVG | `cargo test --test ticketing_credentials_tests` | FAILED (`unresolved import ticket, OsRandomCredentials, CredentialGenerator`) |
| GREEN | Credentials & Domain | `cargo test --test ticketing_credentials_tests` | PASSED (`test_random_credentials_generation_format_and_uniqueness`, `test_ticket_domain_validation_and_courtesy_invariants`, `test_qr_svg_rendered_only_when_ticket_is_paid`) |
| RED | Multi-row Sale & Rollback | `cargo test --test ticketing_sale_concurrency_tests` | FAILED (`unresolved imports TicketRepository, SellTickets, GetPublicTicket`) |
| GREEN | Transactions & Tickets Use Cases | `cargo test --test ticketing_sale_concurrency_tests` | PASSED (`issue_sale` IMMEDIATE transaction, capacity ceiling guard, rollback on overflow) |
| TRIANGULATE | Courtesy & Public Concealment | `cargo test --test ticketing_sale_concurrency_tests` | PASSED (Courtesy forces price 0.0 & Pagado; public ticket exposes SVG only if paid; never leaks `codigo_qr`, `id`, or `evento_id`) |
| RED | Atomic Check-in Concurrency | `cargo test --test ticketing_door_concurrency_tests` | FAILED (`unresolved import CheckInTicket`) |
| GREEN | Door Check-in 3 States | `cargo test --test ticketing_door_concurrency_tests` | PASSED (Atomic conditional write: exactly 1 `Valida` and 1 `Ya Usada` in concurrent race; rejection of `st_*`, cross-event, and pending) |
| GREEN | HTTP Endpoints & Router | `cargo test --test ticketing_api_tests` | PASSED (Verified `/api/eventos/{id}/entradas`, `/api/entradas/{id}`, `/api/entradas/{id}/confirmar-pago`, `/api/entradas/checkin`, `/api/public/ticket/{scan_token}` with `no-store`) |
| REFACTOR | Concurrency & Lints | `cargo test` + `cargo clippy --lib --test ticketing_credentials_tests --test ticketing_sale_concurrency_tests --test ticketing_door_concurrency_tests --test ticketing_api_tests -- -D warnings` | PASSED (All 33 test suites green, zero compiler warnings, zero clippy warnings) |

---

## 3. Files Created and Modified (Cumulative)

### Created in Slice 2
- `src/domain/ticketing/ticket.rs` (Domain entity `Ticket`, `ScanToken`, `QrSecret`, `PaymentMethod`, `PaymentStatus`, `CheckinStatus`, SVG renderer)
- `src/infra/random_credentials.rs` (`OsRandomCredentials` generating independent `st_` + 64 hex and 32 hex secrets via `OsRng`)
- `src/infra/sqlite/ticketing_repository/tickets.rs` (`TicketRepository` queries/filters and `TicketingTransactions` with atomic `IMMEDIATE` sale and check-in)
- `src/use_cases/ticketing/tickets.rs` (`SellTickets`, `ConfirmTicketPayment`, `GetPublicTicket`, `ListTickets`, `GetTicket`, `UpdateTicketContact`, `DeleteTicket`)
- `src/use_cases/ticketing/checkin.rs` (`CheckInTicket` with 3 discrete outcomes: `Valida`, `Ya Usada`, `Invalida`)
- `src/handlers/ticketing/tickets.rs` (Axum handlers for ticket sales, queries, public attendee view with `no-store`, and door check-in)
- `tests/ticketing_credentials_tests.rs` (Unit tests for random credentials, SVG conditional rendering, and domain invariants)
- `tests/ticketing_sale_concurrency_tests.rs` (Concurrency and property tests for multi-row issuance, capacity rollback, and secrecy)
- `tests/ticketing_door_concurrency_tests.rs` (Concurrency race tests for atomic check-in and invalid reason classification)

### Modified in Slice 2
- `Cargo.toml` & `Cargo.lock` (added `rand = "0.8"` and `qrcode = { version = "0.14", default-features = false, features = ["svg"] }`)
- `src/domain/ticketing/mod.rs` (exposed `ticket` module and public types)
- `src/ports/ticketing.rs` (added `CredentialGenerator`, `GeneratedCredentials`, `SellTicketsInput`, `CheckInOutcome`, `TicketRepository`, `TicketingTransactions`)
- `src/infra/mod.rs` (exposed `random_credentials`)
- `src/infra/sqlite/ticketing_repository/mod.rs` (exposed `tickets`)
- `src/use_cases/ticketing/mod.rs` (exposed `tickets` and `checkin`)
- `src/state/ticketing_services.rs` (wired `sell_tickets`, `list_tickets`, `get_ticket`, `update_ticket_contact`, `delete_ticket`, `confirm_ticket_payment`, `get_public_ticket`, `check_in_ticket`)
- `src/handlers/ticketing/dto.rs` (added `TicketDto`, `SellTicketsRequest`, `SellTicketsResponse`, `ListTicketsResponse`, `TicketResponse`, `UpdateTicketContactRequest`, `CheckInRequest`)
- `src/handlers/ticketing/mod.rs` (exposed `tickets`)
- `src/router.rs` (registered `/api/eventos/{id}/entradas`, `/api/entradas/{id}`, `/api/entradas/{id}/confirmar-pago`, `/api/entradas/checkin`, `/api/public/ticket/{scan_token}`)
- `tests/ticketing_api_tests.rs` (added end-to-end integration test `test_tickets_and_checkin_api_endpoints`)
- `openspec/changes/shirley-show-ticketing/tasks.md` (checked 9/9 tasks for Slice 2, now 19/50 total)

---

## 4. Deviations from Design

None. The implementation followed `design.md`, `specs/entradas/spec.md`, `specs/checkin/spec.md`, and `specs/entrada-publica/spec.md` with absolute fidelity:
- Dual-token model: `scan_token` (`st_` + 64 hex) exclusively for public view, `codigo_qr` (32 lowercase hex) exclusively for door check-in.
- SVG QR generated via `qrcode` crate with `svg` feature only, omitting binary/image backends.
- Strict QR concealment: Pending tickets produce `null` for `qr_svg`, while paid tickets deliver vector SVG markup.
- Atomic conditional check-in write prevents duplicate concurrent entry registrations with zero race conditions.
- `PublicTicketDto` strips internal IDs, `scan_token`, and the plaintext `codigo_qr` secret, and delivers `Cache-Control: no-store`.

---

## 5. Workload and PR Boundary

- **Work unit:** `tix-s2-tickets-qr-checkin`
- **Assigned slice:** Slice 2 (PR 2 of 6 chained stacked PRs)
- **Review forecast vs Authored lines:** The implementation of credentials generation, dual-token security, transactional sale, atomic door check-in, use cases, Axum handlers, and concurrency test suites spans ~1,700 lines. Like Slice 1, this work unit cannot be partitioned further without breaking compile-time and transactional integrity across domain, ports, repositories, use cases, and HTTP routing. A `size:exception` applies to PR 2 as part of the authorized stacked review sequence.
- **Authority token for settle:** `sha256:6d379bb138d0c43abe5ee94b4157cd66604829736f9d65228f9be47db08453fe`, request `t2-acquire-003`, max 3000. Settle was NOT executed per instruction ("Reporta para settle (NO settle)").

---

## 6. Remaining Tasks (31 remaining across Slices 3–6)

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
