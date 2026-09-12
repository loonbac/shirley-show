# Tasks: Ressy Agent Parity (`shirley-show-agent-parity`)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 1050 - 1300 lines total (S1: ~340, S2: ~480, S3: ~380) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Slice 1: Safe tools) → PR 2 (Slice 2: Destructive tools & confirmation) → PR 3 (Slice 3: Poster upload & image chat) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

---

## Delivery Slices Overview

1. **Slice 1: Safe Editing, Reordering, and Lifecycle Tools (8 tools)**
   - Registers and dispatches 8 non-destructive tools (`renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`).
   - Allowlist updates from 16 to 24 tools.
   - Reuses existing use cases in `src/use_cases/` and `src/use_cases/ticketing/`.
   - Budget estimate: ~340 changed lines (Low/Medium budget risk).

2. **Slice 2: Destructive Tools, Server-Side Confirmation Registry, and Prompt Bounds (7 tools)**
   - Registers and dispatches 7 destructive tools (`eliminar_presentacion`, `eliminar_elemento`, `eliminar_evento`, `finalizar_evento`, `eliminar_entrada`, `eliminar_gasto`, `eliminar_interesado`).
   - Introduces server-side ephemeral confirmation registry (`PendingDestructiveRegistry`), in-memory UUID `conversation_id`, and deterministic two-turn execution boundary.
   - Updates `system_prompt.rs` with centralized confirmation instructions under 2,500 token budget.
   - Allowlist updates from 24 to 31 tools.
   - Budget estimate: ~480 changed lines (High budget risk — triggers `ask-on-risk` gate before apply).

3. **Slice 3: Poster Upload and Chat Image Attachment (1 tool + Multimodal Chat)**
   - Registers and dispatches `subir_afiche` delegating to `ReplaceEventPoster`.
   - Extends chat multipart pipeline with backend magic-byte detection (JPEG, PNG, WebP) and 10 MiB limit alongside 100 MiB audio.
   - Extends frontend composer and API validation for image attachments and reactive event view refresh.
   - Allowlist reaches final count of exactly 32 tools.
   - Budget estimate: ~380 changed lines (Medium budget risk).

---

## Slice 1: Safe Editing, Reordering, and Lifecycle Tools (8 Tools)

### 1.1 Tests (RED)
- [x] Implement unit and schema contract tests for the 8 safe tools in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs`, asserting strict schemas (`additionalProperties: false`), correct parameter types/ranges, and intermediate catalog count of 24 tools. <!-- sdd-owner: implementation -->
- [x] Implement input validation tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` for parameter constraints: non-empty trimmed titles, mutually exclusive reordering parameters (`posicion_destino` vs `direccion`), element edit requirements (at least one of `texto` or `titulo`), valid event tier arrays, and valid closed enums for expense category, interest status, and payment method. <!-- sdd-owner: implementation -->

### 1.2 Implementation (GREEN)
- [x] Register schemas for the 8 safe tools (`renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`) in `src/agent/tool_schemas.rs`, updating `get_tool_definitions()` to expose 24 tools. <!-- sdd-owner: implementation -->
- [x] Define strict argument structs with `#[serde(deny_unknown_fields)]` and implement dispatch logic calling existing use cases (`RenamePresentation`, `ReorderItem`, `UpdateShowItem`, `UpdateEvent`, `ReopenEvent`, `UpdateExpense`, `UpdateInterest`, `ConvertInterestToSale`) in `src/agent/tool_dispatcher.rs`. <!-- sdd-owner: implementation -->
- [x] Record structured `ToolAction` with `status: "success"` or `"error"` including affected `presentation_id` or `event_id` for all 8 tools in `src/agent/tool_dispatcher.rs`. <!-- sdd-owner: implementation -->

### 1.3 Triangulation & Edge Cases (TRIANGULATE)
- [x] Add integration tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` verifying that domain errors from use cases (e.g. non-existent entity, capacity overflow, invalid state transition) return structured tool errors `{ "error": ..., "message": ... }` instead of reporting success. <!-- sdd-owner: implementation -->
- [x] Verify turn-scoped deduplication in `src/agent/tool_dispatcher.rs` and `tests/agent_tools_tests.rs` so identical mutating calls within the same turn execute only once and return a structured duplicate warning. <!-- sdd-owner: implementation -->
- [x] Add reactive refresh tests in `frontend/js/data/agent-refresh.js` and `tests/agent-chat-integration.test.mjs` verifying that successful safe tool executions trigger view updates for presentations, items, events, expenses, and leads. <!-- sdd-owner: implementation -->

### 1.4 Refactoring (REFACTOR)
- [x] Refactor argument parsing, enum mapping, and error translation helpers in `src/agent/tool_dispatcher.rs` to maintain high cohesion and eliminate boilerplate across presentation and ticketing handlers. <!-- sdd-owner: implementation -->

---

## Slice 2: Destructive Tools, Server-Side Confirmation Registry, and Prompt Bounds (7 Tools)

### 2.1 Tests (RED)
- [ ] Implement unit tests in `tests/agent_chat_tests.rs` for `PendingDestructiveRegistry` in `src/agent/destructive_confirmation.rs`: test arming, atomic single-use consumption (`take`), 15-minute TTL eviction, conversation isolation, and rejection of client-forged history without server-side pending state. <!-- sdd-owner: implementation -->
- [ ] Implement two-turn conversational flow tests in `tests/agent_chat_tests.rs`: turn 1 refuses execution, identifies target/consequences, and asks for exact phrase; turn 2 with exact phrase (`sí, elimínalo` / `sí, finalízalo`) executes canonically; ambiguous reply (`dale`, `ok`, `hazlo`) re-arms and repeats question; cancellation (`mejor no`, `cancela`) clears pending state; repeated turn 3 confirmation does not re-execute. <!-- sdd-owner: implementation -->
- [ ] Implement schema and allowlist count tests in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` for the 7 destructive tools, verifying strict schemas and intermediate catalog count of 31 tools. <!-- sdd-owner: implementation -->

### 2.2 Implementation (GREEN)
- [ ] Create `src/agent/destructive_confirmation.rs` defining `PendingDestructiveAction`, `DestructiveTool`, `DestructiveTarget`, `ConfirmationReply`, and thread-safe `PendingDestructiveRegistry`, exposing them in `src/agent/mod.rs`. <!-- sdd-owner: implementation -->
- [ ] Wire `Arc<PendingDestructiveRegistry>` into `AppState` (`src/state/app_state.rs`) and update HTTP handlers in `src/handlers/agent/json_request.rs`, `src/handlers/agent/multipart_request.rs`, and `src/handlers/agent/chat.rs` to accept, generate, and return `conversation_id`, taking any pending action into turn context. <!-- sdd-owner: implementation -->
- [ ] Update `TurnToolContext` in `src/agent/turn_context.rs` to carry `conversation_id`, `pending_destructive_action`, `confirmation_reply`, and `confirmation_requested`, and implement synthetic authorized execution in `src/use_cases/agent_chat.rs`. <!-- sdd-owner: implementation -->
- [ ] Register schemas for the 7 destructive tools in `src/agent/tool_schemas.rs` and implement preflight checks and use-case dispatch in `src/agent/tool_dispatcher.rs` for `DeletePresentation`, `DeleteItem`, `DeleteEvent`, `FinalizeEvent`, `DeleteTicket`, `DeleteExpense`, and `DeleteInterest`. <!-- sdd-owner: implementation -->
- [ ] Rewrite `src/agent/system_prompt.rs` into compact sections with centralized two-turn confirmation rules, inject pending confirmation context in `build_context_system_prompt`, and add automated test ensuring rendered system prompt is strictly below 2,500 tokens. <!-- sdd-owner: implementation -->

### 2.3 Triangulation & Edge Cases (TRIANGULATE)
- [ ] Implement concurrency and race-condition tests in `tests/agent_chat_tests.rs`: verify that two concurrent requests for the same conversation cannot both execute the same destructive authorization, and verify cascade deletion consequences for events and presentations. <!-- sdd-owner: implementation -->
- [ ] Add tests in `tests/agent_chat_tests.rs` for non-existent target resources: verify that confirmed deletion against an already-deleted or invalid target returns structured domain error (`not_found`) without crashing or reporting success. <!-- sdd-owner: implementation -->
- [ ] Implement client-side in-memory `conversationId` propagation in `frontend/js/data/chat-state.js` and test automatic navigation to parent view (`removed: true`) when the active event or presentation is deleted in `tests/agent-chat-state.test.mjs`. <!-- sdd-owner: implementation -->

### 2.4 Refactoring (REFACTOR)
- [ ] Refactor confirmation text matching and target resolution in `src/agent/destructive_confirmation.rs` and `src/agent/tool_dispatcher.rs`, extracting cohesive helpers for consequence messages and label lookups. <!-- sdd-owner: implementation -->

---

## Slice 3: Poster Upload and Chat Image Attachment (1 Tool + Multimodal Chat)

### 3.1 Tests (RED)
- [ ] Implement magic-byte classification unit tests in `tests/agent_chat_api_tests.rs`: verify valid JPEG (`FF D8 FF`), PNG (`89 50 4E 47 ...`), and WebP (`RIFF ... WEBP`), rejection of spoofed extensions/MIME, rejection of files exceeding 10 MiB (10,485,760 bytes), and verify regression-free handling of audio files up to 100 MiB. <!-- sdd-owner: implementation -->
- [ ] Implement unit and integration tests for `subir_afiche` in `tests/ticketing_ressy_tests.rs`: test successful replacement via `ReplaceEventPoster`, error on missing attached image (`no_image_staged`), error on non-existent event (`evento_not_found`), and prevention of duplicate consumption (`image_already_consumed`). <!-- sdd-owner: implementation -->
- [ ] Implement final allowlist assertion test in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` verifying exactly 32 tools, and test exclusion of browser playback, Stage Mode, and camera control. <!-- sdd-owner: implementation -->

### 3.2 Implementation (GREEN)
- [ ] Centralize `MAX_POSTER_SIZE_BYTES` constant and add `StagedImageHandle` (`Available`, `Consumed`, `Failed`) to `TurnToolContext` in `src/agent/turn_context.rs`. <!-- sdd-owner: implementation -->
- [ ] Update `src/handlers/agent/multipart_request.rs` to classify stream by initial bytes, staging valid images ($\le 10 \text{ MiB}$) in memory and audio ($\le 100 \text{ MiB}$) in `StagingFileGuard`, rejecting duplicate `file` fields or unsupported payloads with HTTP 400 without invoking Groq. <!-- sdd-owner: implementation -->
- [ ] Register schema for `subir_afiche` in `src/agent/tool_schemas.rs` and implement dispatcher handler in `src/agent/tool_dispatcher.rs` invoking `ReplaceEventPoster::execute`, consuming the image handle, and emitting `ToolAction`. <!-- sdd-owner: implementation -->
- [ ] Update frontend attachment validation in `frontend/js/data/agent-chat-api.js` (`validateAgentAttachment` supporting audio $\le 100 \text{ MiB}$ and image $\le 10 \text{ MiB}$, preserving `validateAudioAttachment` wrapper), update `frontend/js/components/organisms/agent-chat-composer.js` file input to accept `image/*`, and add poster upload progress label in `frontend/js/data/chat-state.js`. <!-- sdd-owner: implementation -->

### 3.3 Triangulation & Edge Cases (TRIANGULATE)
- [ ] Implement end-to-end multipart chat tests in `tests/agent_chat_api_tests.rs` and `tests/agent-chat-integration.test.mjs`: verify multipart image upload calling `subir_afiche`, poster file persistence in disk storage, turn-scoped cleanup of unconsumed images, and client-side event view refresh. <!-- sdd-owner: implementation -->
- [ ] Verify prompt tokenizer count in `tests/agent_tools_tests.rs` with all 32 tools and full context active, ensuring token count remains strictly below 2,500 tokens. <!-- sdd-owner: implementation -->

### 3.4 Refactoring (REFACTOR)
- [ ] Consolidate format detection utilities between audio and image pipelines, ensuring zero memory leaks for dropped or unconsumed staged image buffers. <!-- sdd-owner: implementation -->
