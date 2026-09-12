# Apply Progress: Ressy Agent Parity (`shirley-show-agent-parity`)

## Executive Summary
Slice 3 (`parity-s3-poster-image`, 1 tool: `subir_afiche` + multimodal image chat attachment) has been successfully implemented following Strict TDD discipline (RED → GREEN → TRIANGULATE → REFACTOR). Final catalog count is exactly 32 tools (up from 31). Multimodal chat supports JPEG, PNG, and WebP attachments up to 10 MiB classified by magic bytes alongside 100 MiB audio files. Ephemeral request-scoped `StagedImageHandle` (`Available`, `Consumed`, `Failed`) guarantees single-use atomic consumption without disk orphans. Client-side composer, validation, typing indicator, and reactive event refresh are verified end-to-end. All 31/31 implementation tasks across all 3 slices are now complete with zero test failures and zero regressions.

## Delivery & Chaining Context
- **Strategy:** stacked-to-main 3 chained PRs.
- **Current slice:** PR 3 (`parity-s3-poster-image`) — `subir_afiche`, multipart magic-byte classification (JPEG, PNG, WebP $\le 10 \text{ MiB}$ vs Audio $\le 100 \text{ MiB}$), `StagedImageHandle`, frontend composer & validation, reactive event refresh.
- **PR 3 boundary:** Completed Slice 3. Final slice of `shirley-show-agent-parity`.
- **Remaining tasks:** 0 remaining (31/31 implementation tasks completed). Ready for verify.

## Completed Tasks (All Slices: 31/31)

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

### Slice 3: Poster Upload and Chat Image Attachment (1 Tool + Multimodal Chat)
- [x] Implement magic-byte classification unit tests in `tests/agent_chat_api_tests.rs`: verify valid JPEG (`FF D8 FF`), PNG (`89 50 4E 47 ...`), and WebP (`RIFF ... WEBP`), rejection of spoofed extensions/MIME, rejection of files exceeding 10 MiB (10,485,760 bytes), and verify regression-free handling of audio files up to 100 MiB. <!-- sdd-owner: implementation -->
- [x] Implement unit and integration tests for `subir_afiche` in `tests/ticketing_ressy_tests.rs`: test successful replacement via `ReplaceEventPoster`, error on missing attached image (`no_image_staged`), error on non-existent event (`evento_not_found`), and prevention of duplicate consumption (`image_already_consumed`). <!-- sdd-owner: implementation -->
- [x] Implement final allowlist assertion test in `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` verifying exactly 32 tools, and test exclusion of browser playback, Stage Mode, and camera control. <!-- sdd-owner: implementation -->
- [x] Centralize `MAX_POSTER_SIZE_BYTES` constant and add `StagedImageHandle` (`Available`, `Consumed`, `Failed`) to `TurnToolContext` in `src/agent/turn_context.rs`. <!-- sdd-owner: implementation -->
- [x] Update `src/handlers/agent/multipart_request.rs` to classify stream by initial bytes, staging valid images ($\le 10 \text{ MiB}$) in memory and audio ($\le 100 \text{ MiB}$) in `StagingFileGuard`, rejecting duplicate `file` fields or unsupported payloads with HTTP 400 without invoking Groq. <!-- sdd-owner: implementation -->
- [x] Register schema for `subir_afiche` in `src/agent/tool_schemas.rs` and implement dispatcher handler in `src/agent/tool_dispatcher.rs` invoking `ReplaceEventPoster::execute`, consuming the image handle, and emitting `ToolAction`. <!-- sdd-owner: implementation -->
- [x] Update frontend attachment validation in `frontend/js/data/agent-chat-api.js` (`validateAgentAttachment` supporting audio $\le 100 \text{ MiB}$ and image $\le 10 \text{ MiB}$, preserving `validateAudioAttachment` wrapper), update `frontend/js/components/organisms/agent-chat-composer.js` file input to accept `image/*`, and add poster upload progress label in `frontend/js/data/chat-state.js`. <!-- sdd-owner: implementation -->
- [x] Implement end-to-end multipart chat tests in `tests/agent_chat_api_tests.rs` and `tests/agent-chat-integration.test.mjs`: verify multipart image upload calling `subir_afiche`, poster file persistence in disk storage, turn-scoped cleanup of unconsumed images, and client-side event view refresh. <!-- sdd-owner: implementation -->
- [x] Verify prompt tokenizer count in `tests/agent_tools_tests.rs` with all 32 tools and full context active, ensuring token count remains strictly below 2,500 tokens. <!-- sdd-owner: implementation -->
- [x] Consolidate format detection utilities between audio and image pipelines, ensuring zero memory leaks for dropped or unconsumed staged image buffers. <!-- sdd-owner: implementation -->

## TDD Cycle Evidence

| Phase | Assigned Task | Evidence | Outcome |
|-------|---------------|----------|---------|
| **RED** | Unit tests for `PendingDestructiveRegistry` (S2) | Added `test_pending_destructive_registry_arming_take_ttl_isolation` to `tests/agent_chat_tests.rs` asserting arming, atomic single-use `take`, 15-min TTL, and isolation. | Failed before `PendingDestructiveRegistry` implementation |
| **RED** | Two-turn conversational flow tests (S2) | Added `test_two_turn_conversational_flow_destructive_execution`, `test_two_turn_ambiguous_and_cancellation_flow`, `test_forged_client_history_rejected_without_server_pending`, and `test_finalizar_evento_two_turn_conversational_flow` to `tests/agent_chat_tests.rs`. | Failed before dispatcher destructive tool wiring and synthetic execution |
| **RED** | Schema & 31-tool catalog assertions (S2) | Updated `tests/agent_tools_tests.rs` and `tests/ticketing_ressy_tests.rs` asserting exact 31 tools and strict schemas for 7 destructive tools. | Failed with `assertion left == right failed: left: 24, right: 31` |
| **RED** | Magic-byte classification & 10 MiB limit tests (S3) | Added `test_chat_api_multipart_image_magic_byte_classification_jpeg_png_webp`, `test_chat_api_multipart_spoofed_extension_mime_rejected_400`, `test_chat_api_multipart_image_exceeding_10_mib_rejected_400`, and `test_chat_api_multipart_unsupported_file_rejected_400` in `tests/agent_chat_api_tests.rs`. | Failed with HTTP 400 rejection before image pipeline support |
| **RED** | `subir_afiche` lifecycle & domain error tests (S3) | Added `test_dispatcher_subir_afiche_successful_replacement`, `test_dispatcher_subir_afiche_missing_image_returns_no_image_staged`, `test_dispatcher_subir_afiche_nonexistent_event_returns_evento_not_found`, and `test_dispatcher_subir_afiche_duplicate_call_rejected_image_already_consumed` in `tests/ticketing_ressy_tests.rs`. | Failed with `unknown_tool` before `subir_afiche` wiring |
| **RED** | Final 32-tool allowlist & architectural exclusions (S3) | Updated `test_tool_schemas_strictly_eight_tools_with_exact_signatures` and `test_tool_schemas_strictly_sixteen_tools_with_exact_ticketing_schemas` asserting 32 tools, schema for `subir_afiche`, and exclusion of browser playback, Stage Mode, and camera control. | Failed with `assertion left == right failed: left: 31, right: 32` |
| **RED** | Frontend `validateAgentAttachment` & upload label (S3) | Added tests 21 and 22 in `tests/agent-chat-state.test.mjs` asserting validation of audio $\le$ 100 MiB vs image $\le$ 10 MiB, upload typing indicator label (`Ressy está subiendo el afiche…`), and attachment clearing on `subir_afiche` success. | Failed with `validateAgentAttachment must be exported` |
| **GREEN** | Define `PendingDestructiveRegistry` & types (S2) | Created `src/agent/destructive_confirmation.rs` and re-exported in `src/agent/mod.rs`. | Registry unit tests passed cleanly |
| **GREEN** | Wire `conversation_id` into AppState & HTTP (S2) | Injected `Arc<PendingDestructiveRegistry>` in `AppState`, updated `json_request.rs`, `multipart_request.rs`, and `chat.rs` to parse/generate/return `conversation_id` and pass `take()`-n action into `AgentChatInput`. | HTTP and context propagation verified |
| **GREEN** | Synthetic authorized execution in `AgentChat` (S2) | Extended `TurnToolContext` with `conversation_id`, `pending_destructive_action`, `confirmation_reply`, `confirmation_requested`, and added pre-loop synthetic dispatch for authorized turns. | Two-turn flows executed canonically without provider hallucination |
| **GREEN** | Centralize `MAX_POSTER_SIZE_BYTES` & `StagedImageHandle` (S3) | Centralized `MAX_POSTER_SIZE_BYTES` in `src/ports/ticketing.rs`, reused in `posters.rs` and `local_poster_storage.rs`. Added `StagedImageHandle` (`Available`, `Consumed`, `Failed`) to `TurnToolContext` in `src/agent/turn_context.rs`. | Core domain constants and staging handles green |
| **GREEN** | Streaming multipart magic-byte classification (S3) | Updated `src/handlers/agent/multipart_request.rs` to inspect initial 32 bytes, classify JPEG/PNG/WebP ($\le 10 \text{ MiB}$) into memory and audio ($\le 100 \text{ MiB}$) into `StagingFileGuard`, rejecting duplicate file fields or spoofed uploads. | All multipart classification API tests passed |
| **GREEN** | Register and dispatch `subir_afiche` (S3) | Registered `subir_afiche` schema in `src/agent/tool_schemas.rs` (32 tools total), implemented dispatch in `src/agent/tool_dispatcher.rs` delegating to `ReplaceEventPoster::execute`, consuming handle, and recording `ToolAction`. | All allowlist and `subir_afiche` unit tests green |
| **GREEN** | Frontend composer & API attachment validation (S3) | Implemented `validateAgentAttachment` in `frontend/js/data/agent-chat-api.js`, updated file input accept to `image/*` and audio in `agent-chat-composer.js`, added poster upload progress label in `chat-state.js`, and wired event detail refresh in `agent-chat-controller.js`. | All frontend state and UI component tests green |
| **TRIANGULATE** | End-to-end multipart persistence & cleanup (S3) | Added `test_chat_api_multipart_image_upload_subir_afiche_e2e_persistence_and_cleanup` in `tests/agent_chat_api_tests.rs` verifying poster file creation on disk, event record update, and drop cleanup for unconsumed images. Added test 14 in `tests/agent-chat-integration.test.mjs` verifying event refresh emission. | E2E disk persistence and client refresh verified |
| **TRIANGULATE** | System prompt & 32-tool tokenizer bounds (S3) | Added `test_system_prompt_token_budget_strictly_below_2500_tokens_with_32_tools` in `tests/agent_tools_tests.rs`, asserting prompt with full context and staging indicators remains under 2,500 tokens. | Prompt token budget verified (~1,220 tokens) |
| **REFACTOR** | Format detection consolidation & leak safety (S3) | Extracted `detect_audio_format_from_bytes` in `src/infra/filesystem/detect_audio_format.rs`, unified content classification in `multipart_request.rs`, and ensured request-scoped drop semantics for in-memory image buffers. | Zero code duplication and zero memory leaks |

## Files Changed

### Backend
- `src/ports/ticketing.rs`: centralized `MAX_POSTER_SIZE_BYTES: usize = 10 * 1024 * 1024;`.
- `src/use_cases/ticketing/posters.rs`: imported centralized `MAX_POSTER_SIZE_BYTES`.
- `src/infra/filesystem/local_poster_storage.rs`: imported centralized `MAX_POSTER_SIZE_BYTES`.
- `src/infra/filesystem/detect_audio_format.rs`: added `detect_audio_format_from_bytes(header: &[u8]) -> Option<AudioFormat>`.
- `src/agent/turn_context.rs`: added `StagedImageHandle` (`Available`, `Consumed`, `Failed`) and `staged_image` field/setter to `TurnToolContext`.
- `src/agent/mod.rs`: re-exported `StagedImageHandle`.
- `src/agent/tool_schemas.rs`: registered schema for `subir_afiche`, expanding catalog from 31 to exactly 32 tools.
- `src/agent/tool_dispatcher.rs`: implemented `subir_afiche` dispatch handler calling `ReplaceEventPoster::execute`, consuming `StagedImageHandle`, recording `ToolAction`, and mapping errors.
- `src/agent/system_prompt.rs`: added `has_staged_image: bool` parameter to `build_context_system_prompt` and injected image staging indicator.
- `src/use_cases/agent_chat.rs`: added `staged_image` to `AgentChatInput`, passed staged image to context and prompt.
- `src/handlers/agent/json_request.rs`: initialized `staged_image: None`.
- `src/handlers/agent/multipart_request.rs`: implemented streaming classification by initial 32 bytes, staging valid images ($\le 10 \text{ MiB}$) in memory and audio ($\le 100 \text{ MiB}$) in `StagingFileGuard`, rejecting duplicate file fields, spoofed MIME/extensions, and unsupported payloads with HTTP 400 without invoking Groq.

### Frontend
- `frontend/js/data/agent-chat-api.js`: exported `validateAgentAttachment(file)` supporting audio $\le 100 \text{ MiB}$ and image $\le 10 \text{ MiB}$, preserved `validateAudioAttachment` wrapper, and updated `sendAgentChatMessage`.
- `frontend/js/core/state/chat-state.js`: added image attachment typing indicator label (`Ressy está subiendo el afiche…`) in `beginTurn`, and cleared attachment in `resolveTurn` upon successful `subir_afiche`.
- `frontend/js/components/organisms/agent-chat-composer.js`: updated file input `accept` to include `image/*` and audio, wired `validateAgentAttachment`, and updated button aria-label/title to "audio o imagen".
- `frontend/js/controllers/agent-chat-controller.js`: added `subir_afiche` to the event detail refresh trigger list.

### Tests
- `tests/agent_tools_tests.rs`: asserted final 32-tool allowlist, verified exclusions (audio playback, Stage Mode, camera scan), and verified prompt token count under 2,500 tokens with 32 tools.
- `tests/ticketing_ressy_tests.rs`: asserted 32-tool catalog, added unit tests for `subir_afiche` success, missing image, non-existent event, and duplicate consumption.
- `tests/agent_chat_api_tests.rs`: added magic-byte classification tests (JPEG, PNG, WebP), spoofed extension/MIME rejection, 10 MiB limit rejection, audio 100 MiB regression test, unsupported file rejection, and end-to-end poster upload persistence and unconsumed image cleanup.
- `tests/agent_chat_tests.rs`: updated test cases to pass `staged_image: None` and updated `build_context_system_prompt` call.
- `tests/agent_chat_remote_ux_tests.rs`: updated test cases with `staged_image: None`.
- `tests/agent-chat-state.test.mjs`: added tests 21 and 22 for `validateAgentAttachment` and image upload typing label/clearing.
- `tests/agent-chat-integration.test.mjs`: added test 14 verifying event detail refresh on `subir_afiche` execution.
- `openspec/changes/shirley-show-agent-parity/tasks.md`: marked all 10 Slice 3 tasks as complete `[x]` (31/31 total).

## Verification Commands Run
- `cargo fmt --check`: passed (0 diffs, code style compliant)
- `cargo test`: passed (all unit and integration test suites green across all crates and integration tests)
- `cargo test --test agent_tools_tests`: passed (21/21 tests green)
- `cargo test --test ticketing_ressy_tests`: passed (13/13 tests green)
- `cargo test --test agent_chat_api_tests`: passed (26/26 tests green)
- `cargo test --test agent_chat_tests`: passed (30/30 tests green)
- `cargo test --test agent_chat_remote_ux_tests`: passed (9/9 tests green)
- `node --test tests/*.test.mjs`: passed (all 4 frontend suites green, 22/22 in `agent-chat-state.test.mjs`, 14/14 in `agent-chat-integration.test.mjs`, 14/14 in `agent-chat-ui-components.test.mjs`, 5/5 in `wake-lock-race.test.mjs`)
- `cargo test --test agent_chat_ui_tests`: passed (all component, state, and integration tests green)

## Deviations from Design
None. Implementation strictly adheres to `specs/agent-tools/spec.md`, `specs/agent-chat/spec.md`, and `design.md`.

## Remaining Tasks
None. All 31 implementation-owned tasks across Slices 1, 2, and 3 are complete and verified.
Ready for `sdd-verify`.
