# Apply Progress: Ressy Agent Parity (`shirley-show-agent-parity`)

## Executive Summary
Slice 1 (`parity-s1-safe-tools`, 8 safe tools) has been successfully implemented following Strict TDD discipline (RED → GREEN → TRIANGULATE → REFACTOR). Intermediate catalog count is 24 tools (up from 16). All unit, integration, route, and UI tests pass with zero regressions.

## Delivery & Chaining Context
- **Strategy:** stacked-to-main 3 chained PRs.
- **Current slice:** PR 1 (`parity-s1-safe-tools`) — 8 safe tools (`renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`).
- **PR 1 boundary:** Completed Slice 1. Ready for verify and acquire settlement for Slice 1.
- **Remaining slices:**
  - Slice 2: Destructive tools, server-side confirmation registry, prompt bounds (7 tools).
  - Slice 3: Poster upload and multimodal image chat (1 tool).

## Completed Tasks (Slice 1)
- [x] Implement unit and schema contract tests for the 8 safe tools in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs`, asserting strict schemas (`additionalProperties: false`), correct parameter types/ranges, and intermediate catalog count of 24 tools. <!-- sdd-owner: implementation -->
- [x] Implement input validation tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` for parameter constraints: non-empty trimmed titles, mutually exclusive reordering parameters (`posicion_destino` vs `direccion`), element edit requirements (at least one of `texto` or `titulo`), valid event tier arrays, and valid closed enums for expense category, interest status, and payment method. <!-- sdd-owner: implementation -->
- [x] Register schemas for the 8 safe tools (`renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`) in `src/agent/tool_schemas.rs`, updating `get_tool_definitions()` to expose 24 tools. <!-- sdd-owner: implementation -->
- [x] Define strict argument structs with `#[serde(deny_unknown_fields)]` and implement dispatch logic calling existing use cases (`RenamePresentation`, `ReorderItem`, `UpdateShowItem`, `UpdateEvent`, `ReopenEvent`, `UpdateExpense`, `UpdateInterest`, `ConvertInterestToSale`) in `src/agent/tool_dispatcher.rs`. <!-- sdd-owner: implementation -->
- [x] Record structured `ToolAction` with `status: "success"` or `"error"` including affected `presentation_id` or `event_id` for all 8 tools in `src/agent/tool_dispatcher.rs`. <!-- sdd-owner: implementation -->
- [x] Add integration tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` verifying that domain errors from use cases (e.g. non-existent entity, capacity overflow, invalid state transition) return structured tool errors `{ "error": ..., "message": ... }` instead of reporting success. <!-- sdd-owner: implementation -->
- [x] Verify turn-scoped deduplication in `src/agent/tool_dispatcher.rs` and `tests/agent_tools_tests.rs` so identical mutating calls within the same turn execute only once and return a structured duplicate warning. <!-- sdd-owner: implementation -->
- [x] Add reactive refresh tests in `frontend/js/data/agent-refresh.js` and `tests/agent-chat-integration.test.mjs` verifying that successful safe tool executions trigger view updates for presentations, items, events, expenses, and leads. <!-- sdd-owner: implementation -->
- [x] Refactor argument parsing, enum mapping, and error translation helpers in `src/agent/tool_dispatcher.rs` to maintain high cohesion and eliminate boilerplate across presentation and ticketing handlers. <!-- sdd-owner: implementation -->

## TDD Cycle Evidence

| Phase | Assigned Task | Evidence | Outcome |
|-------|---------------|----------|---------|
| **RED** | Unit and schema contract tests for 8 safe tools | Added 24-tool catalog count and schema assertions in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs`. | Tests failed with `assertion left == right failed: left: 16, right: 24` |
| **RED** | Input validation and domain error tests | Added tests for empty titles, mutually exclusive reordering, missing text/title, closed enums, negative amounts, capacity constraints. | Tests failed with `unknown_tool` before registration |
| **GREEN** | Register 8 safe schemas in `tool_schemas.rs` | Added schemas for 8 tools with `additionalProperties: false` to `get_tool_definitions()`. | Catalog count reached 24 tools; schema tests passed |
| **GREEN** | Dispatch logic and `ToolAction` recording in `tool_dispatcher.rs` | Added strict argument structs with `#[serde(deny_unknown_fields)]`, dispatched to existing domain use cases, and recorded structured `ToolAction` with `presentation_id`/`event_id`. | Operational lifecycle tests passed |
| **TRIANGULATE** | Domain errors, deduplication, and reactive refresh | Verified turn-scoped deduplication ignored duplicates, verified domain error translation (`app_error_to_json`), and expanded frontend reactive refresh with scopes `catalog`, `presentation`, `events`, and `event`. | Integration tests in Rust and Node passed cleanly |
| **REFACTOR** | Modular cohesion & boilerplate elimination | Extracted `format_tickets_response`, `parse_ticket_types`, `normalize_event_fecha_hora`, and authorization helpers (`is_authorized_presentation`, `unauthorized_presentation_err`). | Zero duplication across presentation and ticketing handlers, clean architecture preserved |

## Files Changed
- `src/agent/tool_schemas.rs`: registered 8 safe tools (`renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`).
- `src/agent/tool_dispatcher.rs`: defined argument structs, dispatch logic, domain use-case integration, `ToolAction` emission, and cohesion helpers.
- `src/state/app_state.rs`: injected `rename_presentation`, `reorder_item`, and `update_show_item` into `AgentToolDispatcher::from_components`.
- `frontend/js/core/state/agent-refresh.js`: expanded refresh signal with scopes `events` and `event`, plus `eventId` and `removed`.
- `frontend/js/data/agent-refresh.js`: created module re-exporting agent refresh capabilities.
- `frontend/js/controllers/agent-chat-controller.js`: wired refresh event emission for all safe tools.
- `frontend/js/modules/events/controllers.js`: subscribed event catalog and detail controllers to agent refresh signals.
- `tests/agent_tools_tests.rs`: added contract, schema, input validation, execution, and domain error tests for presentation safe tools.
- `tests/ticketing_ressy_tests.rs`: added contract, schema, input validation, execution, and domain error tests for ticketing safe tools.
- `tests/agent-chat-integration.test.mjs`: added integration test verifying reactive view refreshes for presentations, items, events, expenses, and leads.
- `openspec/changes/shirley-show-agent-parity/tasks.md`: updated checkboxes for completed Slice 1 tasks.

## Verification Commands Run
- `cargo test`: passed (all unit and integration tests green)
- `node --test tests/*.test.mjs`: passed (all 4 frontend test suites green)
- `cargo test --test agent_chat_ui_tests`: passed (UI component, state, and integration runners green)

## Deviations from Design
None. Implementation strictly adheres to `specs/agent-tools/spec.md`, `specs/agent-chat/spec.md`, and `design.md`.

## Remaining Tasks (Slices 2 & 3)
- [ ] Implement unit tests in `tests/agent_chat_tests.rs` for `PendingDestructiveRegistry` in `src/agent/destructive_confirmation.rs`: test arming, atomic single-use consumption (`take`), 15-minute TTL eviction, conversation isolation, and rejection of client-forged history without server-side pending state. <!-- sdd-owner: implementation -->
- [ ] Implement two-turn conversational flow tests in `tests/agent_chat_tests.rs`: turn 1 refuses execution, identifies target/consequences, and asks for exact phrase; turn 2 with exact phrase (`sí, elimínalo` / `sí, finalízalo`) executes canonically; ambiguous reply (`dale`, `ok`, `hazlo`) re-arms and repeats question; cancellation (`mejor no`, `cancela`) clears pending state; repeated turn 3 confirmation does not re-execute. <!-- sdd-owner: implementation -->
- [ ] Implement schema and allowlist count tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` for the 7 destructive tools, verifying strict schemas and intermediate catalog count of 31 tools. <!-- sdd-owner: implementation -->
- [ ] Create `src/agent/destructive_confirmation.rs` defining `PendingDestructiveAction`, `DestructiveTool`, `DestructiveTarget`, `ConfirmationReply`, and thread-safe `PendingDestructiveRegistry`, exposing them in `src/agent/mod.rs`. <!-- sdd-owner: implementation -->
- [ ] Wire `Arc<PendingDestructiveRegistry>` into `AppState` (`src/state/app_state.rs`) and update HTTP handlers in `src/handlers/agent/json_request.rs`, `src/handlers/agent/multipart_request.rs`, and `src/handlers/agent/chat.rs` to accept, generate, and return `conversation_id`, taking any pending action into turn context. <!-- sdd-owner: implementation -->
- [ ] Update `TurnToolContext` in `src/agent/turn_context.rs` to carry `conversation_id`, `pending_destructive_action`, `confirmation_reply`, and `confirmation_requested`, and implement synthetic authorized execution in `src/use_cases/agent_chat.rs`. <!-- sdd-owner: implementation -->
- [ ] Register schemas for the 7 destructive tools in `src/agent/tool_schemas.rs` and implement preflight checks and use-case dispatch in `src/agent/tool_dispatcher.rs` for `DeletePresentation`, `DeleteItem`, `DeleteEvent`, `FinalizeEvent`, `DeleteTicket`, `DeleteExpense`, and `DeleteInterest`. <!-- sdd-owner: implementation -->
- [ ] Rewrite `src/agent/system_prompt.rs` into compact sections with centralized two-turn confirmation rules, inject pending confirmation context in `build_context_system_prompt`, and add automated test ensuring rendered system prompt is strictly below 2,500 tokens. <!-- sdd-owner: implementation -->
- [ ] Implement concurrency and race-condition tests in `tests/agent_chat_tests.rs`: verify that two concurrent requests for the same conversation cannot both execute the same destructive authorization, and verify cascade deletion consequences for events and presentations. <!-- sdd-owner: implementation -->
- [ ] Add tests in `tests/agent_chat_tests.rs` for non-existent target resources: verify that confirmed deletion against an already-deleted or invalid target returns structured domain error (`not_found`) without crashing or reporting success. <!-- sdd-owner: implementation -->
- [ ] Implement client-side in-memory `conversationId` propagation in `frontend/js/data/chat-state.js` and test automatic navigation to parent view (`removed: true`) when the active event or presentation is deleted in `tests/agent-chat-state.test.mjs`. <!-- sdd-owner: implementation -->
- [ ] Refactor confirmation text matching and target resolution in `src/agent/destructive_confirmation.rs` and `src/agent/tool_dispatcher.rs`, extracting cohesive helpers for consequence messages and label lookups. <!-- sdd-owner: implementation -->
- [ ] Implement magic-byte classification unit tests in `tests/agent_chat_api_tests.rs`: verify valid JPEG (`FF D8 FF`), PNG (`89 50 4E 47 ...`), and WebP (`RIFF ... WEBP`), rejection of spoofed extensions/MIME, rejection of files exceeding 10 MiB (10,485,760 bytes), and verify regression-free handling of audio files up to 100 MiB. <!-- sdd-owner: implementation -->
- [ ] Implement unit and integration tests for `subir_afiche` in `tests/ticketing_ressy_tests.rs`: test successful replacement via `ReplaceEventPoster`, error on missing attached image (`no_image_staged`), error on non-existent event (`evento_not_found`), and prevention of duplicate consumption (`image_already_consumed`). <!-- sdd-owner: implementation -->
- [ ] Implement final allowlist assertion test in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` verifying exactly 32 tools, and test exclusion of browser playback, Stage Mode, and camera control. <!-- sdd-owner: implementation -->
- [ ] Centralize `MAX_POSTER_SIZE_BYTES` constant and add `StagedImageHandle` (`Available`, `Consumed`, `Failed`) to `TurnToolContext` in `src/agent/turn_context.rs`. <!-- sdd-owner: implementation -->
- [ ] Update `src/handlers/agent/multipart_request.rs` to classify stream by initial bytes, staging valid images ($\le 10 \text{ MiB}$) in memory and audio ($\le 100 \text{ MiB}$) in `StagingFileGuard`, rejecting duplicate `file` fields or unsupported payloads with HTTP 400 without invoking Groq. <!-- sdd-owner: implementation -->
- [ ] Register schema for `subir_afiche` in `src/agent/tool_schemas.rs` and implement dispatcher handler in `src/agent/tool_dispatcher.rs` invoking `ReplaceEventPoster::execute`, consuming the image handle, and emitting `ToolAction`. <!-- sdd-owner: implementation -->
- [ ] Update frontend attachment validation in `frontend/js/data/agent-chat-api.js` (`validateAgentAttachment` supporting audio $\le 100 \text{ MiB}$ and image $\le 10 \text{ MiB}$, preserving `validateAudioAttachment` wrapper), update `frontend/js/components/organisms/agent-chat-composer.js` file input to accept `image/*`, and add poster upload progress label in `frontend/js/data/chat-state.js`. <!-- sdd-owner: implementation -->
- [ ] Implement end-to-end multipart chat tests in `tests/agent_chat_api_tests.rs` and `tests/agent-chat-integration.test.mjs`: verify multipart image upload calling `subir_afiche`, poster file persistence in disk storage, turn-scoped cleanup of unconsumed images, and client-side event view refresh. <!-- sdd-owner: implementation -->
- [ ] Verify prompt tokenizer count in `tests/agent_tools_tests.rs` with all 32 tools and full context active, ensuring token count remains strictly below 2,500 tokens. <!-- sdd-owner: implementation -->
- [ ] Consolidate format detection utilities between audio and image pipelines, ensuring zero memory leaks for dropped or unconsumed staged image buffers. <!-- sdd-owner: implementation -->
