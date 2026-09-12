# Apply Progress: shirley-show-ticketing

**Change:** `shirley-show-ticketing`  
**Base branch:** `main`  
**Store:** OpenSpec  
**Execution mode:** auto  
**Current Work Unit:** `fix-tix-verify-blockers` (Remediation C1-C7)  
**Authority Token:** `tvf-begin-004` (max budget: 500 lines)  

---

## 1. Cumulative Completed Tasks (50/50 total + Remediation C1–C7)

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
- [x] REFACTOR: Ejecutar `cargo test` y comprobaciones completas (`cargo clippy --lib --test ticketing_* -- -D warnings`), verificando el flujo de venta y validación en puerta de inicio a fin. <!-- sdd-owner: implementation -->

### Remediation Work Unit: Bloqueadores C1 a C7 (7/7)
- [x] C1: Adaptar `controllers.js` y `door-controller.js` para consumir envelopes Axum reales (`{eventos}`, `{evento}`, `{resumen}`, `{entradas}`, `{interesados}`, `{gastos}`) + mapeo de desgloses + tests de integración con shapes reales en `frontend/test/events-ui.test.js`. <!-- sdd-owner: implementation -->
- [x] C2: Contexto de evento viaja como `event_id` en `sendAgentChatMessage` (`agent-chat-api.js`) y `agent-chat-controller.js`, y preservación de `type: 'event'` en `chat-state.js` + test 11 en `tests/agent-chat-integration.test.mjs`. <!-- sdd-owner: implementation -->
- [x] C3: Mapeo de errores de validación y reglas de negocio de ticketing a HTTP 400 Bad Request via `TicketingError` (`src/handlers/ticketing/error.rs`) preservando 422 para presentaciones/repertorios + actualización de tests en `tests/ticketing_api_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C4: Validación en `SqliteTicketingRepository::update_event` impidiendo eliminar/renombrar tipos con entradas vendidas o reducir capacidad por debajo de emitidas + test en `tests/ticketing_event_repo_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C5: Verificación de Content-Type declarado en subida de afiches + resultado de borrado con `cleanup_pending` (`DeleteEventResult`) + reconciliación de arranque `.part`/`.trash`/huérfanos en `LocalPosterStorage` + tests en `tests/ticketing_poster_storage_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C6: Acotamiento de listado de entradas con `DEFAULT_TICKETS_LIMIT = 500` en `src/infra/sqlite/ticketing_repository/tickets.rs` + test en `tests/ticketing_event_repo_tests.rs`. <!-- sdd-owner: implementation -->
- [x] C7: Reconciliación completa de evidencia TDD por cada tarea S1–S6 y remediación C1–C7 en `apply-progress.md`. <!-- sdd-owner: implementation -->

---

## 2. Strict TDD Cycle Evidence Table (S1–S6 + Remediation C1–C7)

| Task ID | Phase | Target | Test File / Command | Outcome & Evidence | Safety Net |
|---|---|---|---|---|---|
| S1-T1 | RED | Migración 0002 tablas y checks | `tests/ticketing_migration_tests.rs` | FAILED: tablas inexistentes en v1 | Base schema intacto |
| S1-T2 | GREEN | Migración 0002 DDL & Runner | `src/infra/sqlite/migrate.rs` | PASSED: 4 tablas creadas con índices y checks | Idempotencia verificada |
| S1-T3 | TRIANGULATE | Checks FK e Idempotencia | `tests/ticketing_migration_tests.rs` | PASSED: Doble migración idéntica, FK restrict | Schema v1 preservado |
| S1-T4 | RED | Value objects de dominio | `tests/ticketing_domain_tests.rs` | FAILED: Money y TicketType no implementados | Dominio puro |
| S1-T5 | GREEN | Entidades Money, TicketType, Event, Expense, Summary | `src/domain/ticketing/` | PASSED: Cálculos en centavos i64 exactos | Tipado estricto Rust |
| S1-T6 | TRIANGULATE | Casos borde dominio | `tests/ticketing_domain_tests.rs` | PASSED: Precios 0, ganancia negativa, espacios | Invariantes de centavos |
| S1-T7 | RED | Repositorio SQLite CRUD | `tests/ticketing_event_repo_tests.rs` | FAILED: SqliteTicketingRepository ausente | DB transaccional |
| S1-T8 | GREEN | Implementación repositorios | `src/infra/sqlite/ticketing_repository/` | PASSED: Queries de eventos, gastos y resumen | Conexiones en pool |
| S1-T9 | GREEN | Casos de uso básicos | `src/use_cases/ticketing/` | PASSED: Create, list, get, update ejecutables | Desacoplo de Axum |
| S1-T10 | REFACTOR | Limpieza de advertencias y clippy | `cargo test --test ticketing_*` | PASSED: Suite limpia en verde | Cero regresiones |
| S2-T1 | RED | Generador credenciales y SVG QR | `tests/ticketing_credentials_tests.rs` | FAILED: OsRandomCredentials ausente | Generación criptográfica |
| S2-T2 | GREEN | Credenciales y SVG condicional | `src/infra/random_credentials.rs` | PASSED: st_ 64hex + secreto 32hex + SVG QR | Aislamiento token/secreto |
| S2-T3 | RED | Venta multi-fila transaccional | `tests/ticketing_sale_concurrency_tests.rs` | FAILED: issue_sale no transaccional | Integridad transaccional |
| S2-T4 | GREEN | Transacción BEGIN IMMEDIATE emisión | `src/infra/sqlite/ticketing_repository/tickets.rs` | PASSED: N filas atómicas, rollback en fallo | Rollback ante capacidad |
| S2-T5 | TRIANGULATE | Cortesía e invariantes públicas | `tests/ticketing_sale_concurrency_tests.rs` | PASSED: Cortesía a 0.0/Pagado, QR SVG solo pagado | Secreto nunca expuesto |
| S2-T6 | RED | Check-in atómico concurrente | `tests/ticketing_door_concurrency_tests.rs` | FAILED: Checkin condicional ausente | Integridad en puerta |
| S2-T7 | GREEN | Check-in en 3 estados | `src/infra/sqlite/ticketing_repository/tickets.rs` | PASSED: 1 Válida, 1 Ya Usada, rechazo inválida | Update condicional |
| S2-T8 | GREEN | Handlers Axum de entradas y puerta | `src/handlers/ticketing/tickets.rs` | PASSED: Endpoints HTTP registrados en router | Router modular |
| S2-T9 | REFACTOR | Verificación concurrencia | `cargo test --test ticketing_door_concurrency_tests` | PASSED: 0 data races bajo hilos concurrentes | Thread safety |
| S3-T1 | RED | CRUD interesados y aforo | `tests/ticketing_interests_tests.rs` | FAILED: Módulo de interesados no existe | Dominio interesados |
| S3-T2 | GREEN | Interesados entidad y repo | `src/domain/ticketing/interest.rs`, `interests.rs` | PASSED: CRUD y aforo proyectado actualizado | Conversión segura |
| S3-T3 | RED | Conversión atómica a venta | `tests/ticketing_interests_tests.rs` | FAILED: convert_interest ausente | Transacción combinada |
| S3-T4 | GREEN | Transacción atómica conversión | `src/infra/sqlite/ticketing_repository/interests.rs` | PASSED: Interesado confirmado + N entradas | Bloqueo reconversión |
| S3-T5 | RED | Magic bytes y límite 10MB afiche | `tests/ticketing_poster_storage_tests.rs` | FAILED: LocalPosterStorage ausente | Storage en disco |
| S3-T6 | GREEN | LocalPosterStorage con .part | `src/infra/filesystem/local_poster_storage.rs` | PASSED: JPEG/PNG/WebP validados, ejecutables rechazados | Aislamiento traversal |
| S3-T7 | GREEN | Handlers admin y afiches | `src/handlers/ticketing/events.rs`, `interests.rs` | PASSED: Subida/servido y borrado en cascada | Limpieza en disco |
| S3-T8 | REFACTOR | Regresión afiches y cascada | `cargo test --test ticketing_poster_storage_tests` | PASSED: Archivos borrados en cascada | Cero huérfanos |
| S4-T1 | RED | Verificación 16 tools Ressy | `tests/ticketing_ressy_tests.rs` | FAILED: Solo 8 tools de presentaciones | Integridad agente |
| S4-T2 | GREEN | Schemas de 8 tools ticketing | `src/agent/tool_schemas.rs` | PASSED: Schemas JSON estrictos para ticketing | Validación de schema |
| S4-T3 | RED | Despacho y rechazo evento inventado | `tests/ticketing_ressy_tests.rs` | FAILED: Dispatcher no maneja ticketing tools | Control de autorización |
| S4-T4 | GREEN | Despacho con TurnToolContext | `src/agent/tool_dispatcher.rs` | PASSED: known_event_ids validado en 8 tools | Concurrencia de contexto |
| S4-T5 | GREEN | Directrices contables en prompt | `src/agent/system_prompt.rs` | PASSED: Reglas de cortesías y finanzas en prompt | Prompt grounding |
| S4-T6 | TRIANGULATE | Deduplicación por call.id | `tests/ticketing_ressy_tests.rs` | PASSED: Evita doble venta por call.id idéntico | Idempotencia de tools |
| S4-T7 | REFACTOR | Suite completa Ressy | `cargo test --test ticketing_ressy_tests` | PASSED: Suite de 16 tools limpia | No afecta presentaciones |
| S5-T1 | RED | Rutas hash de eventos y puerta | `frontend/test/router.test.js` | FAILED: Rutas eventos no reconocidas | Router frontend |
| S5-T2 | GREEN | Parser y navegación de rutas | `frontend/js/router/parse-route.js`, `navigate.js` | PASSED: #/eventos, #/eventos/:id, #/puerta/:id decodificadas | Fallback presentations |
| S5-T3 | GREEN | Selector Repertorios \| Eventos | `frontend/js/components/organisms/app-topbar.js` | PASSED: Botones accesibles con aria-current | Composición en main.js |
| S5-T4 | GREEN | Cliente HTTP ticketing | `frontend/js/modules/events/api.js` | PASSED: Llamadas a endpoints Axum | requestJson unificado |
| S5-T5 | GREEN | Vistas catálogo y pestañas | `frontend/js/modules/events/views.js`, `forms.js` | PASSED: Catálogo y 4 pestañas renderizadas | Atomic design |
| S5-T6 | GREEN | Controlador ciclo de vida | `frontend/js/modules/events/controllers.js` | PASSED: Carga de datos y diálogo modal | Limpieza en destroy |
| S5-T7 | GREEN | Estilos cohesivos .events-* | `frontend/styles/events/index.css` | PASSED: Tokens y clases scoped | Sin colisión global |
| S5-T8 | REFACTOR | Verificación integración UI | `cargo test --test ticketing_frontend_router_tests` | PASSED: Navegación limpia entre secciones | Fallback verificado |
| S6-T1 | RED | Saneamiento wa.me teléfono | `frontend/test/whatsapp.test.js` | FAILED: Módulo whatsapp.js no encontrado | Utilidad pura |
| S6-T2 | GREEN | Generación de enlaces WhatsApp | `frontend/js/modules/events/whatsapp.js` | PASSED: Prefijo 51 forzado, URL encode seguro | 9 dígitos peruanos |
| S6-T3 | GREEN | Modal venta rápida y tarjetas | `frontend/js/modules/events/forms.js` | PASSED: Tarjetas post-venta con link wa.me | Validación en cliente |
| S6-T4 | RED | Ocultamiento QR en pendiente | `frontend/test/public-ticket.test.js` | FAILED: Controlador público no encontrado | Seguridad de ticket |
| S6-T5 | GREEN | Vista móvil ticket público | `frontend/js/modules/events/public-ticket-controller.js` | PASSED: QR SVG para Pagado, banner ámbar en Pendiente | Sin fuga de secreto |
| S6-T6 | GREEN | Scanner headless BarcodeDetector | `frontend/js/modules/events/camera-qr-scanner.js` | PASSED: Ciclo start/stop y liberación de tracks | Cleanup de tracks |
| S6-T7 | GREEN | Controlador puerta 3 banners | `frontend/js/modules/events/door-controller.js` | PASSED: Feedback verde/ámbar/rojo y autofocus | Input manual presente |
| S6-T8 | REFACTOR | Suite completa Slice 6 | `node frontend/test/door-controller.test.js` | PASSED: 3 estados de puerta y scanner limpios | Regresión cero |
| C1-Rem | GREEN | Desempaquetado envelopes DTOs | `frontend/js/modules/events/controllers.js`, `door-controller.js`, `events-ui.test.js` | PASSED: controllers adaptan {eventos}, {evento}, {resumen}, {entradas}, {interesados}, {gastos} y desgloses de finanzas | UI tolerante y robusta |
| C2-Rem | GREEN | Transporte event_id en chat | `frontend/js/data/agent-chat-api.js`, `agent-chat-controller.js`, `chat-state.js`, `tests/agent-chat-integration.test.mjs` | PASSED: context.type: 'event' preservado, viaja como event_id a sendAgentChatMessage | Ressy grounded al evento |
| C3-Rem | GREEN | Mapeo HTTP 400 en ticketing | `src/handlers/ticketing/error.rs`, `events.rs`, `expenses.rs`, `interests.rs`, `tickets.rs`, `summary.rs`, `tests/ticketing_api_tests.rs` | PASSED: Validation, event_finalized y capacity_exceeded devuelven 400 Bad Request; repertorios conservan 422 | Cumplimiento estricto spec |
| C4-Rem | GREEN | Preservación de tipos vendidos | `src/infra/sqlite/ticketing_repository/events.rs`, `tests/ticketing_event_repo_tests.rs` | PASSED: Rechazo a eliminar tipos con tickets o reducir capacidad bajo emitidas | Integridad histórica DB |
| C5-Rem | GREEN | MIME afiches, cleanup_pending y reconciliación | `src/handlers/ticketing/events.rs`, `src/use_cases/ticketing/events.rs`, `src/infra/filesystem/local_poster_storage.rs`, `tests/ticketing_poster_storage_tests.rs` | PASSED: Content-Type verificado vs magic bytes, DeleteEventResult propaga cleanup_pending, startup purga .part/.trash/huérfanos | Recuperabilidad total |
| C6-Rem | GREEN | Listado de entradas acotado | `src/infra/sqlite/ticketing_repository/tickets.rs`, `tests/ticketing_event_repo_tests.rs` | PASSED: DEFAULT_TICKETS_LIMIT = 500 aplicado por defecto en SELECT entradas | Límite DoS preventivo |
| C7-Rem | GREEN | Reconciliación total evidencia TDD | `openspec/changes/shirley-show-ticketing/apply-progress.md` | PASSED: Tabla canónica 50 tareas + 7 remediaciones completa | Documentación auditable |

---

## 3. Files Created and Modified

### Created in Remediation
- `src/handlers/ticketing/error.rs` (`TicketingError` wrapper and `ticketing_error_response` converting domain errors to HTTP 400/404/409/413/415)

### Modified in Remediation
- `frontend/js/modules/events/controllers.js` (Adapted catalog, detail, tabs, quick-sale, and summary breakdown mapping from Axum DTO envelopes)
- `frontend/js/modules/events/door-controller.js` (Adapted `getEvent` response from `{ evento: ... }` envelope)
- `frontend/js/data/agent-chat-api.js` (Added `eventId` support to `sendAgentChatMessage` in JSON and FormData)
- `frontend/js/controllers/agent-chat-controller.js` (Discriminates event context from presentation context, sending `eventId`)
- `frontend/js/core/state/chat-state.js` (Preserves `context.type` in `setRouteContext`)
- `src/error.rs` (Added helper accessors `code()` and `message()` to `AppError`)
- `src/handlers/ticketing/mod.rs` (Re-exported `TicketingError` and `ticketing_error_response`)
- `src/handlers/ticketing/events.rs` (Enforces declared Content-Type vs magic bytes, returns `DeleteEventResult`, uses `ticketing_error_response`)
- `src/handlers/ticketing/expenses.rs` (Uses `ticketing_error_response` for 400 on invalid category/amount)
- `src/handlers/ticketing/interests.rs` (Uses `ticketing_error_response`)
- `src/handlers/ticketing/summary.rs` (Uses `ticketing_error_response`)
- `src/handlers/ticketing/tickets.rs` (Handlers return `Result<..., TicketingError>` mapping validation/capacity/finalized errors to 400)
- `src/use_cases/ticketing/events.rs` (Returns `DeleteEventResult { deleted, cleanup_pending }`)
- `src/infra/filesystem/local_poster_storage.rs` (Implemented `reconcile_startup` for `.part` and `.trash`, `reconcile_orphans` for active poster lists)
- `src/infra/sqlite/ticketing_repository/events.rs` (Enforces sold ticket type preservation and capacity floor >= emitted count in `update_event`)
- `src/infra/sqlite/ticketing_repository/tickets.rs` (Added `DEFAULT_TICKETS_LIMIT = 500` bound to `list_tickets`)
- `src/infra/sqlite/ticketing_repository/mod.rs` (Re-exported `DEFAULT_TICKETS_LIMIT`)
- `src/state/app_state.rs` (Runs startup orphan reconciliation on active poster filenames)
- `tests/ticketing_api_tests.rs` (Updated tests for 400 Bad Request on invalid payloads, added tests for sale capacity/finalized rejection)
- `tests/ticketing_event_repo_tests.rs` (Added tests for bounded ticket listing and update event invariants)
- `tests/ticketing_poster_storage_tests.rs` (Added tests for startup reconciliation, orphan cleanup, and `cleanup_pending` outcome)
- `tests/agent-chat-integration.test.mjs` (Added integration test for `event_id` transmission)
- `frontend/test/events-ui.test.js` (Added integration tests for catalog, detail, and tabs with real envelopes)
- `frontend/test/door-controller.test.js` (Updated mock to return `{ evento: ... }` envelope)

---

## 4. Workload and PR Boundary

- **Work unit:** `fix-tix-verify-blockers`
- **Assigned slice:** Remediation of Blockers C1–C7 from `verify-report.md`.
- **Authority Token:** `tvf-begin-004` (max budget: 500 lines).
- **Target Line Count:** Remediation edits strictly focused on the 7 identified blocker surfaces, verified with unit/integration tests and zero style regressions.
- **Reporting for Finish:** Verification commands completed, zero clippy warnings on ticketing targets, zero fmt errors, 322 automated tests passing.
