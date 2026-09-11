# Tasks: Shirley Show AI Agent

**Change:** `shirley-show-ai-agent`  
**Execution mode:** Auto  
**Artifact store:** OpenSpec  
**Delivery strategy:** ask-on-risk  
**Review budget:** 400 lines per review unit  
**Strict TDD:** true (runner: `cargo test`)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 1700 – 2400 lines (S1: 950 – 1300 lines, S2: 750 – 1100 lines) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (S1.1 Provider & Ports) → PR 2 (S1.2 Dispatcher & Loop) → PR 3 (S1.3 HTTP & Staging) → PR 4 (S2.1 State & Client) → PR 5 (S2.2 Panel UI & Integration) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High
```

---

## Slice 1: Backend Agent and Bounded Tool Execution

### Work Unit 1.1: Runtime Configuration, Groq Provider Port and Infrastructure Client
- [x] RED: Add unit and mock integration tests in `tests/groq_client_tests.rs` verifying Groq request serialization (model, system prompt, message history, tool definitions, tool_choice auto), Bearer authorization header injection, base URL normalisation, and HTTP error mapping (401 to `provider_auth`, 429 to `provider_rate_limited`, 5xx to `provider_unavailable`, network timeout to `agent_timeout`, malformed JSON choices to `provider_error`) against an ephemeral local Axum mock server. <!-- sdd-owner: implementation -->
- [x] GREEN: Add `dotenvy` and `reqwest` (with `rustls-tls` and `json`) dependencies to `Cargo.toml`; implement `GroqConfig` in `src/config.rs`; update `src/main.rs` to load environment variables via `dotenvy::dotenv().ok()` before `Config::from_env()`; define the domain port `ChatCompletionProvider` in `src/ports/chat_completion_provider.rs` and export it in `src/ports/mod.rs`; implement `src/infra/groq/client.rs`, `src/infra/groq/dto.rs`, `src/infra/groq/error.rs`, and export via `src/infra/groq/mod.rs` and `src/infra/mod.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Add edge-case test coverage in `tests/groq_client_tests.rs` for empty choices array, trailing slashes in base URL, and verification that `GROQ_API_KEY` is omitted or stripped of whitespace without panic. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Verify that error messages and debug outputs in `src/infra/groq/` do not log raw Bearer keys, full conversation prompts, or raw provider payload bytes; verify tests pass with `cargo test --test groq_client_tests`. <!-- sdd-owner: implementation -->

### Work Unit 1.2: Agent Tool Schemas, Context Tracking and Local Dispatcher
- [x] RED: Add tests in `tests/agent_tools_tests.rs` asserting strict JSON Schemas for exactly six tools (`create_presentation`, `list_presentations`, `get_presentation`, `add_lyrics`, `add_note`, `add_track_from_staged`), rejection of unknown or destructive tools with structured errors, presentation ID validation against authorized `TurnToolContext`, deduplication of mutation calls by tool call ID and canonical JSON argument signature, and successful execution of underlying domain use cases. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement agent message domain types in `src/domain/agent_message.rs` and tool types in `src/domain/agent_tool.rs`, exporting them in `src/domain/mod.rs`; implement system prompt constants in `src/agent/system_prompt.rs`; implement tool JSON Schemas in `src/agent/tool_schemas.rs`; implement canonical argument signature calculation in `src/agent/tool_signature.rs`; implement request-scoped ID tracking in `src/agent/turn_context.rs`; implement the local tool dispatcher in `src/agent/tool_dispatcher.rs`, wiring existing use cases (`CreatePresentation`, `ListPresentations`, `GetPresentation`, `AddLyrics`, `AddNote`, `UploadTrack`), and export via `src/agent/mod.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Add tests in `tests/agent_tools_tests.rs` for duplicate mutation calls returning `duplicate_call_ignored`, cached reads for duplicate query tools, invalid presentation ID rejection, empty name/text trim validation, and missing staged file handles. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Verify isolation of the dispatcher: confirm the model cannot access SQLite tables or the filesystem directly and that mutations append items contiguously; verify with `cargo test --test agent_tools_tests`. <!-- sdd-owner: implementation -->

### Work Unit 1.3: Bounded Agent Orchestration Loop and AppState Integration
- [x] RED: Add test cases in `tests/agent_chat_tests.rs` using a deterministic fixed-response provider stub covering: single-turn informational query, multi-iteration tool execution chain (e.g. create show followed by adding note and lyrics), loop termination at 5 iterations with confirmed actions reported, 30-second global deadline cancellation preserving committed operations, deterministic message composition (grounding: assistant message generated strictly from confirmed local tool actions), and graceful unconfigured response (`notice.code = "agent_unconfigured"`) when no API key is provided. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement the bounded orchestration use case `AgentChat` in `src/use_cases/agent_chat.rs` and export it in `src/use_cases/mod.rs`; update `src/state/app_state.rs` to add `AgentRuntime` with configurable limits (`max_iterations = 5`, `deadline = 30s`) and provider injection for deterministic tests while maintaining default production construction. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Add tests in `tests/agent_chat_tests.rs` for tool execution failure returned back to the model within the loop, provider error occurring after one successful tool mutation (verifying committed action is preserved in `actions_taken`), and active presentation ID update only upon successful `create_presentation`. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Ensure cancellation safety using `tokio::time::timeout_at` for the global request deadline and verify that dropped futures leave database records consistent; verify with `cargo test --test agent_chat_tests`. <!-- sdd-owner: implementation -->

### Work Unit 1.4: HTTP Handler `POST /api/agent/chat` and Request-Scoped Staging Lifecycle
- [x] RED: Add endpoint integration tests in `tests/agent_chat_api_tests.rs` covering: valid text-only JSON request (200 OK), empty or whitespace-only message (400 invalid_request), oversized message (> 32,000 chars, 400 invalid_request), history truncation preserving the latest 20 messages / 32,000 chars, unsupported Content-Type (415 Unsupported Media Type), multipart request with valid audio attachment (`.mp3`, `.wav`, `.m4a`), empty text with valid audio attachment defaulting to track addition, attachment exceeding 100 MiB (400 file_too_large), unsupported attachment format (400 unsupported_format), staged audio cleaned up when unused or on failure, staged audio moved to managed audio when consumed, and unconfigured key returning HTTP 200 with `agent_unconfigured` notice. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement JSON request extractor and validation in `src/handlers/agent/json_request.rs`; implement multipart streaming extractor with streaming 100 MiB size counter and `StagingFileGuard` in `src/handlers/agent/multipart_request.rs`; implement `POST /api/agent/chat` handler in `src/handlers/agent/chat.rs`, exporting via `src/handlers/agent/mod.rs` and `src/handlers/mod.rs`; register the route `/api/agent/chat` in `src/router.rs` and update `src/lib.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Add tests in `tests/agent_chat_api_tests.rs` for client connection abort during multipart streaming ensuring immediate deletion of `.part` staging files, duplicate audio fields rejection, and unconfigured environment behavior. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Run full test suite with `cargo test` to ensure zero regressions across existing presentation, item, upload, cleanup, and streaming tests; verify `.env.example` documents `GROQ_API_KEY`, `GROQ_BASE_URL`, and `GROQ_MODEL`. <!-- sdd-owner: implementation -->

---

## Slice 2: Large Conversational UI

### Work Unit 2.1: Headless Chat State, Refresh Signals and Agent API Client
- [ ] RED: Write unit-level invariant tests in `tests/agent_chat_ui_tests.rs` or focused module verification scripts verifying `chatState` transitions (open/close panel, set draft text, set/replace attachment, begin turn, resolve turn with confirmed actions, fail turn with error notice, set active presentation context, and discard late responses via generation ID and `AbortController`). <!-- sdd-owner: implementation -->
- [ ] GREEN: Extract shared byte-formatting logic into `frontend/js/utils/format/bytes.js` and update `frontend/js/components/organisms/track-item.js` to import it; implement `frontend/js/core/state/chat-state.js` with subscription and action methods; implement `frontend/js/core/state/active-presentation-context.js` for route context tracking; implement `frontend/js/core/state/agent-refresh.js` for catalog/presentation reload events; implement `frontend/js/core/state/stage-mode-state.js` for shortcut suppression; implement `frontend/js/data/agent-chat-api.js` supporting JSON and `FormData` dispatch with error classification. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Verify client-side file validation in `agent-chat-api.js` (rejecting files > 100 MiB and invalid extensions immediately before network dispatch), draft and attachment retention on failed requests, and retry offered only when `actions_taken` is empty. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Ensure headless state modules remain 100% decoupled from DOM nodes and follow micro-modular guidelines in `AGENTS.md`. <!-- sdd-owner: implementation -->

### Work Unit 2.2: Pure UI Molecules, Organisms, Behaviors and Scoped Styles
- [ ] RED: Create visual and structural test harnesses for chat molecules and organisms verifying DOM structure, accessibility attributes (`role="dialog"`, `aria-modal="true"`, `aria-live="polite"`), touch targets (≥ 48 × 48 px), safe text escaping preventing HTML/script execution, and keyboard behaviors (`Escape`, `Enter`, `Shift+Enter`, `Ctrl+K`). <!-- sdd-owner: implementation -->
- [ ] GREEN: Add required SVG icons (`sparkles`, `send`, `paperclip`, `bot`) to `frontend/js/components/atoms/icon.js`; implement focus trap in `frontend/js/behaviors/use-focus-trap.js`; implement auto-scroll behavior in `frontend/js/behaviors/use-stick-to-bottom.js`; implement shortcut handler in `frontend/js/behaviors/bind-agent-chat-shortcuts.js`; implement molecules `frontend/js/components/molecules/agent-chat-trigger.js`, `frontend/js/components/molecules/agent-message-bubble.js`, `frontend/js/components/molecules/agent-typing-indicator.js`, `frontend/js/components/molecules/agent-attachment-chip.js`, and `frontend/js/components/molecules/agent-tool-card.js`; implement organisms `frontend/js/components/organisms/agent-chat-composer.js` and `frontend/js/components/organisms/agent-chat-panel.js`; implement controller `frontend/js/controllers/agent-chat-controller.js`; create 1:1 stylesheet files in `frontend/styles/tokens/motion.css`, `frontend/styles/molecules/agent-chat-trigger.css`, `frontend/styles/molecules/agent-message-bubble.css`, `frontend/styles/molecules/agent-typing-indicator.css`, `frontend/styles/molecules/agent-attachment-chip.css`, `frontend/styles/molecules/agent-tool-card.css`, `frontend/styles/organisms/agent-chat-composer.css`, and `frontend/styles/organisms/agent-chat-panel.css`, importing them into `frontend/styles/index.css`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Test edge cases in UI rendering: assistant message with malicious HTML/script strings is rendered strictly as plain text, multiline lyrics retain line breaks, long filenames wrap without overflowing the composer, disabled state during pending prevents duplicate submits, and `@media (prefers-reduced-motion: reduce)` disables keyframes and sliding transitions. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Verify responsive layout constraints: 580 px slide-over panel with backdrop on desktop (> 768 px), `100vw × 100dvh` fullscreen overlay on mobile (≤ 768 px), and consistent antique gold/charcoal design token alignment. <!-- sdd-owner: implementation -->

### Work Unit 2.3: Application Composition, Route Sync and End-to-End Verification
- [ ] RED: Write integration test/check verifying that opening chat from catalog sets catalog mode context, opening chat from presentation detail sets show context, and confirming tool actions trigger selective view reload. <!-- sdd-owner: implementation -->
- [ ] GREEN: Wire `AgentChatController` as a singleton in `frontend/js/main.js`; update `frontend/js/components/organisms/app-topbar.js` and route controllers (`frontend/js/controllers/presentations-controller.js`, `frontend/js/controllers/presentation-detail-controller.js`) to compose `agent-chat-trigger` and publish active presentation context; subscribe `PresentationsController` to catalog refresh events and `PresentationDetailController` to presentation-specific item updates; register global `Ctrl+K` / `Cmd+K` shortcut listener while checking `stage-mode-state` to suppress activation during live performance. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Perform end-to-end manual verification matrix:
  1. Desktop (1280 px): open chat via trigger or `Ctrl+K`, verify 580 px width, backdrop, and focus trap.
  2. Mobile (390 px): open chat, verify 100% fullscreen coverage and safe area insets.
  3. Conversational show creation: ask Shirley to create a show, verify tool receipt card and instant catalog refresh.
  4. Conversational item addition: in presentation view, ask Shirley to add a note and lyrics, verify instant sequence update preserving scroll.
  5. Audio attachment: attach a valid `.mp3` file, verify attachment chip, send, verify `add_track_from_staged` tool execution, audio moved to managed directory, and playback functioning.
  6. Failure and degradation states: verify graceful message when `GROQ_API_KEY` is missing, friendly error on rate limits or invalid key, and retention of draft message and attachment on failure.
  7. Stage Mode isolation: enter Stage Mode, press `Ctrl+K`, verify chat remains completely suppressed.
  8. Session lifecycle: navigate between catalog and presentation detail, verify in-memory chat history is preserved; reload page, verify chat resets cleanly without affecting SQLite data. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Clean up obsolete debug logs, verify zero console errors during navigation or chat teardown, and verify full test suite passes with `cargo test`. <!-- sdd-owner: implementation -->
