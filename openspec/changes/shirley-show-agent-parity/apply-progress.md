# Apply Progress: Ressy Agent Parity (`shirley-show-agent-parity`)

## Executive Summary
Slice 2 (`parity-s2-destructive`, 7 destructive tools + server-side confirmation registry) has been successfully implemented following Strict TDD discipline (RED → GREEN → TRIANGULATE → REFACTOR). Intermediate catalog count is 31 tools (up from 24). Ephemeral in-memory UUID `conversation_id`, atomic single-use `take()`, 15-minute TTL eviction, client-forged history rejection, and synthetic authorized execution are verified end-to-end. Rendered system prompt remains strictly below 2,500 tokens. All unit, integration, route, and UI tests pass with zero regressions.

## Delivery & Chaining Context
- **Strategy:** stacked-to-main 3 chained PRs.
- **Current slice:** PR 2 (`parity-s2-destructive`) — 7 destructive tools (`eliminar_presentacion`, `eliminar_elemento`, `eliminar_evento`, `finalizar_evento`, `eliminar_entrada`, `eliminar_gasto`, `eliminar_interesado`), server-side `PendingDestructiveRegistry`, prompt compaction under 2,500 tokens.
- **PR 2 boundary:** Completed Slice 2. Ready for settle / verify for Slice 2.
- **Remaining slice:**
  - Slice 3: Poster upload and multimodal image chat (1 tool: `subir_afiche`, magic-byte classification, 32 tools total).

## Completed Tasks (Slice 1 & Slice 2)

### Slice 1: Safe Editing, Reordering, and Lifecycle Tools (8 Tools)
- [x] Implement unit and schema contract tests for the 8 safe tools in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs`, asserting strict schemas (`additionalProperties: false`), correct parameter types/ranges, and intermediate catalog count of 24 tools. <!-- sdd-owner: implementation -->
- [x] Implement input validation tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` for parameter constraints: non-empty trimmed titles, mutually exclusive reordering parameters (`posicion_destino` vs `direccion`), element edit requirements (at least one of `texto` or `titulo`), valid event tier arrays, and valid closed enums for expense category, interest status, and payment method. <!-- sdd-owner: implementation -->
- [x] Register schemas for the 8 safe tools (`renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`) in `src/agent/tool_schemas.rs`, updating `get_tool_definitions()` to expose 24 tools. <!-- sdd-owner: implementation -->
- [x] Define strict argument structs with `#[serde(deny_unknown_fields)]` and implement dispatch logic calling existing use cases (`RenamePresentation`, `ReorderItem`, `UpdateShowItem`, `UpdateEvent`, `ReopenEvent`, `UpdateExpense`, `UpdateInterest`, `ConvertInterestToSale`) in `src/agent/tool_dispatcher.rs`. <!-- sdd-owner: implementation -->
- [x] Record structured `ToolAction` with `status: "success"` or `"error"` including affected `presentation_id` or `event_id` for all 8 tools in `src/agent/tool_dispatcher.rs`. <!-- sdd-owner: implementation -->
- [x] Add integration tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` verifying that domain errors from use cases (e.g. non-existent entity, capacity overflow, invalid state transition) return structured tool errors `{ "error": ..., "message": ... }` instead of reporting success. <!-- sdd-owner: implementation -->
- [x] Verify turn-scoped deduplication in `src/agent/tool_dispatcher.rs` and `tests/agent_tools_tests.rs` so identical mutating calls within the same turn execute only once and return a structured duplicate warning. <!-- sdd-owner: implementation -->
- [x] Add reactive refresh tests in `frontend/js/data/agent-refresh.js` and `tests/agent-chat-integration.test.mjs` verifying that successful safe tool executions trigger view updates for presentations, items, events, expenses, and leads. <!-- sdd-owner: implementation -->
- [x] Refactor argument parsing, enum mapping, and error translation helpers in `src/agent/tool_dispatcher.rs` to maintain high cohesion and eliminate boilerplate across presentation and ticketing handlers. <!-- sdd-owner: implementation -->

### Slice 2: Destructive Tools, Server-Side Confirmation Registry, and Prompt Bounds (7 Tools)
- [x] Implement unit tests in `tests/agent_chat_tests.rs` for `PendingDestructiveRegistry` in `src/agent/destructive_confirmation.rs`: test arming, atomic single-use consumption (`take`), 15-minute TTL eviction, conversation isolation, and rejection of client-forged history without server-side pending state. <!-- sdd-owner: implementation -->
- [x] Implement two-turn conversational flow tests in `tests/agent_chat_tests.rs`: turn 1 refuses execution, identifies target/consequences, and asks for exact phrase; turn 2 with exact phrase (`sí, elimínalo` / `sí, finalízalo`) executes canonically; ambiguous reply (`dale`, `ok`, `hazlo`) re-arms and repeats question; cancellation (`mejor no`, `cancela`) clears pending state; repeated turn 3 confirmation does not re-execute. <!-- sdd-owner: implementation -->
- [x] Implement schema and allowlist count tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` for the 7 destructive tools, verifying strict schemas and intermediate catalog count of 31 tools. <!-- sdd-owner: implementation -->
- [x] Create `src/agent/destructive_confirmation.rs` defining `PendingDestructiveAction`, `DestructiveTool`, `DestructiveTarget`, `ConfirmationReply`, and thread-safe `PendingDestructiveRegistry`, exposing them in `src/agent/mod.rs`. <!-- sdd-owner: implementation -->
- [x] Wire `Arc<PendingDestructiveRegistry>` into `AppState` (`src/state/app_state.rs`) and update HTTP handlers in `src/handlers/agent/json_request.rs`, `src/handlers/agent/multipart_request.rs`, and `src/handlers/agent/chat.rs` to accept, generate, and return `conversation_id`, taking any pending action into turn context. <!-- sdd-owner: implementation -->
- [x] Update `TurnToolContext` in `src/agent/turn_context.rs` to carry `conversation_id`, `pending_destructive_action`, `confirmation_reply`, and `confirmation_requested`, and implement synthetic authorized execution in `src/use_cases/agent_chat.rs`. <!-- sdd-owner: implementation -->
- [x] Register schemas for the 7 destructive tools in `src/agent/tool_schemas.rs` and implement preflight checks and use-case dispatch in `src/agent/tool_dispatcher.rs` for `DeletePresentation`, `DeleteItem`, `DeleteEvent`, `FinalizeEvent`, `DeleteTicket`, `DeleteExpense`, and `DeleteInterest`. <!-- sdd-owner: implementation -->
- [x] Rewrite `src/agent/system_prompt.rs` into compact sections with centralized two-turn confirmation rules, inject pending confirmation context in `build_context_system_prompt`, and add automated test ensuring rendered system prompt is strictly below 2,500 tokens. <!-- sdd-owner: implementation -->
- [x] Implement concurrency and race-condition tests in `tests/agent_chat_tests.rs`: verify that two concurrent requests for the same conversation cannot both execute the same destructive authorization, and verify cascade deletion consequences for events and presentations. <!-- sdd-owner: implementation -->
- [x] Add tests in `tests/agent_chat_tests.rs` for non-existent target resources: verify that confirmed deletion against an already-deleted or invalid target returns structured domain error (`not_found`) without crashing or reporting success. <!-- sdd-owner: implementation -->
- [x] Implement client-side in-memory `conversationId` propagation in `frontend/js/data/chat-state.js` and test automatic navigation to parent view (`removed: true`) when the active event or presentation is deleted in `tests/agent-chat-state.test.mjs`. <!-- sdd-owner: implementation -->
- [x] Refactor confirmation text matching and target resolution in `src/agent/destructive_confirmation.rs` and `src/agent/tool_dispatcher.rs`, extracting cohesive helpers for consequence messages and label lookups. <!-- sdd-owner: implementation -->

## TDD Cycle Evidence

| Phase | Assigned Task | Evidence | Outcome |
|-------|---------------|----------|---------|
| **RED** | Unit tests for `PendingDestructiveRegistry` | Added `test_pending_destructive_registry_arming_take_ttl_isolation` to `tests/agent_chat_tests.rs` asserting arming, atomic single-use `take`, 15-min TTL, and isolation. | Failed before `PendingDestructiveRegistry` implementation |
| **RED** | Two-turn conversational flow tests | Added `test_two_turn_conversational_flow_destructive_execution`, `test_two_turn_ambiguous_and_cancellation_flow`, `test_forged_client_history_rejected_without_server_pending`, and `test_finalizar_evento_two_turn_conversational_flow` to `tests/agent_chat_tests.rs`. | Failed before dispatcher destructive tool wiring and synthetic execution |
| **RED** | Schema & 31-tool catalog assertions | Updated `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` asserting exact 31 tools and strict schemas for 7 destructive tools. | Failed with `assertion left == right failed: left: 24, right: 31` |
| **GREEN** | Define `PendingDestructiveRegistry` & types | Created `src/agent/destructive_confirmation.rs` and re-exported in `src/agent/mod.rs`. | Registry unit tests passed cleanly |
| **GREEN** | Wire `conversation_id` into AppState & HTTP | Injected `Arc<PendingDestructiveRegistry>` in `AppState`, updated `json_request.rs`, `multipart_request.rs`, and `chat.rs` to parse/generate/return `conversation_id` and pass `take()`-n action into `AgentChatInput`. | HTTP and context propagation verified |
| **GREEN** | Synthetic authorized execution in `AgentChat` | Extended `TurnToolContext` with `conversation_id`, `pending_destructive_action`, `confirmation_reply`, `confirmation_requested`, and added pre-loop synthetic dispatch for authorized turns. | Two-turn flows executed canonically without provider hallucination |
| **GREEN** | Dispatch logic for 7 destructive tools | Added `EliminarPresentacionArgs`, `EliminarElementoArgs`, `EventoIdArgs`, `EliminarEntradaArgs`, `EliminarGastoArgs`, `EliminarInteresadoArgs`, preflight existence checks, and use-case execution in `src/agent/tool_dispatcher.rs`. | Two-turn and allowlist tests green |
| **GREEN** | Compact system prompt under 2,500 tokens | Rewrote `src/agent/system_prompt.rs` into 8 compact cohesive sections, injected pending confirmation context, and verified < 2,500 tokens via automated test. | Token budget audit verified (~1,200 estimated tokens) |
| **TRIANGULATE** | Concurrency & race condition safety | Added `test_concurrent_destructive_confirmation_race_single_winner` verifying atomic single-winner `take()`, and `test_cascade_deletion_consequences_for_events_and_presentations` verifying cascades for SQLite records and poster files. | Concurrency stress verified with zero race conditions |
| **TRIANGULATE** | Non-existent target resource handling | Added `test_destructive_execution_against_nonexistent_resource_returns_not_found` verifying structured domain errors (`not_found`) without crashes or false success reports. | Domain errors verified |
| **TRIANGULATE** | Frontend `conversationId` & view navigation | Added tests 19 and 20 in `tests/agent-chat-state.test.mjs`, verified in-memory propagation and clearing of `activeContext` (`removed: true`) upon deletion. | Node unit and integration suites green |
| **REFACTOR** | Cohesive helper extraction | Extracted `dispatch_destructive` in `tool_dispatcher.rs` and centralized normalization in `ConfirmationReply::classify`. | Zero code duplication across the 7 destructive handlers |

## Files Changed
- `src/agent/destructive_confirmation.rs`: created security module with `PendingDestructiveRegistry`, `DestructiveTool`, `DestructiveTarget`, `ConfirmationReply`, and `PendingDestructiveAction`.
- `src/agent/mod.rs`: re-exported destructive confirmation types.
- `src/agent/turn_context.rs`: added `conversation_id`, `pending_destructive_action`, `confirmation_reply`, `confirmation_requested`, and `destructive_registry`.
- `src/agent/tool_schemas.rs`: registered 7 destructive tools (`eliminar_presentacion`, `eliminar_elemento`, `eliminar_evento`, `finalizar_evento`, `eliminar_entrada`, `eliminar_gasto`, `eliminar_interesado`), bringing catalog count to 31.
- `src/agent/tool_dispatcher.rs`: implemented `dispatch_destructive`, preflight checks, use-case delegation, and active ID cleanup.
- `src/agent/system_prompt.rs`: rewrote prompt into 8 compact sections with centralized two-turn confirmation rules and context injection.
- `src/state/app_state.rs`: injected `destructive_registry` into `AppState` and `AgentToolDispatcher`.
- `src/handlers/agent/json_request.rs`: accepted and validated `conversation_id`.
- `src/handlers/agent/multipart_request.rs`: accepted and validated `conversation_id`.
- `src/handlers/agent/chat.rs`: coordinated `conversation_id` generation, `take()` from registry, and response propagation.
- `src/use_cases/agent_chat.rs`: implemented synthetic authorized execution, cancellation handling, ambiguous re-arming, and output formatting.
- `frontend/js/core/state/chat-state.js`: added in-memory `conversationId` tracking and `activeContext` clearing upon deletion.
- `frontend/js/data/chat-state.js`: re-exported chat state.
- `frontend/js/data/agent-chat-api.js`: propagated `conversationId` in JSON and multipart requests.
- `frontend/js/controllers/agent-chat-controller.js`: wired `conversationId` and reactive refresh events for all 7 destructive tools (`removed: true`).
- `tests/agent_tools_tests.rs`: asserted 31-tool catalog count and strict schemas.
- `tests/ticketing_ressy_tests.rs`: asserted 31-tool catalog count and strict schemas.
- `tests/agent_chat_tests.rs`: added tests for registry lifecycle, two-turn flows, ambiguous replies, cancellations, forged history rejection, race conditions, and prompt token budget.
- `tests/agent-chat-state.test.mjs`: added tests 19 and 20 for client-side `conversationId` and context reset on deletion.
- `openspec/changes/shirley-show-agent-parity/tasks.md`: marked all 12 Slice 2 tasks as complete `[x]`.

## Verification Commands Run
- `cargo fmt --check`: passed (0 diffs, code style compliant)
- `cargo test`: passed (all unit and integration test suites green, 30/30 in `agent_chat_tests`, 20/20 in `agent_tools_tests`, 9/9 in `ticketing_ressy_tests`)
- `node --test tests/*.test.mjs`: passed (all 4 frontend suites green, 20/20 in `agent-chat-state.test.mjs`, 13/13 in `agent-chat-integration.test.mjs`)
- `cargo test --test agent_chat_ui_tests`: passed (all component, state, and integration tests green)

## Deviations from Design
None. Implementation strictly adheres to `specs/agent-tools/spec.md`, `specs/agent-chat/spec.md`, and `design.md`.

## Remaining Tasks (Slice 3)
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
