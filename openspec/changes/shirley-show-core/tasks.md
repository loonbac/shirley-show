# Tasks: Shirley Show Core

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 2100–2600 lines |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Slice A: Backend Foundation & Presentations Catalog) → PR 2 (Slice B: Items, Audio Streaming, Upload & Reordering) → PR 3 (Slice C: Micro-Modular Frontend UI & Exclusive Player) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

---

## Slice A: Backend Foundation, Domain, SQLite Storage & Presentations API

### 1. Project Setup and Core Domain Models
- [x] RED: Create initial `Cargo.toml` and write unit tests for presentation domain entities and validation rules in `tests/domain_presentation_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement presentation domain types (`Presentation`, validation, trimming) and error types in `src/domain/presentation.rs`, `src/domain/mod.rs`, and `src/error.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add domain tests for whitespace trimming, Unicode handling, and non-empty title constraints, ensuring `cargo test --test domain_presentation_tests` passes. <!-- sdd-owner: implementation -->

### 2. SQLite Database Schema, Pool Configuration and Migrations
- [x] RED: Write integration tests in `tests/sqlite_migration_tests.rs` asserting WAL mode, `busy_timeout=5000`, `foreign_keys=ON`, and table creation for `presentations`, `show_items`, and `pending_audio_deletions`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement SQL migration `migrations/0001_shirley_show_core.sql`, pool initialization in `src/infra/sqlite/connect.rs`, and embedded migration runner in `src/infra/sqlite/migrate.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Verify migration idempotency and PRAGMA settings on fresh SQLite databases via `cargo test --test sqlite_migration_tests`. <!-- sdd-owner: implementation -->

### 3. Presentation Repository Ports and SQLite Implementation
- [x] RED: Write repository integration tests in `tests/presentation_repository_tests.rs` covering create, list, get, rename, and delete cascade behavior. <!-- sdd-owner: implementation -->
- [x] GREEN: Define traits in `src/ports/presentation_reader.rs`, `src/ports/presentation_writer.rs`, and implement `SqlitePresentationRepository` in `src/infra/sqlite/presentation_repository.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add edge cases for duplicate presentation titles yielding distinct UUIDs, rename preserving items, and non-existent IDs returning `NotFound`. <!-- sdd-owner: implementation -->

### 4. Presentation Use Cases
- [x] RED: Write unit tests in `tests/presentation_use_cases_tests.rs` with mock/in-memory ports for `CreatePresentation`, `ListPresentations`, `GetPresentation`, `RenamePresentation`, and `DeletePresentation`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement use case structs in `src/use_cases/create_presentation.rs`, `src/use_cases/list_presentations.rs`, `src/use_cases/get_presentation.rs`, `src/use_cases/rename_presentation.rs`, and `src/use_cases/delete_presentation.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Ensure strict validation error mapping (empty titles, whitespace-only titles) and wire exports in `src/use_cases/mod.rs`. <!-- sdd-owner: implementation -->

### 5. Presentations HTTP Handlers and API Routing
- [x] RED: Write HTTP integration tests using `axum::Router` and `tower::ServiceExt` in `tests/presentation_api_tests.rs` verifying `/api/presentations` endpoints (GET, POST, GET :id, PATCH :id, DELETE :id). <!-- sdd-owner: implementation -->
- [x] GREEN: Implement DTOs in `src/handlers/dto.rs`, Axum handlers in `src/handlers/presentations/list.rs`, `create.rs`, `get.rs`, `rename.rs`, `delete.rs`, router in `src/router.rs`, application state in `src/state/app_state.rs`, and library root in `src/lib.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add tests for error response JSON format `{ "error": { "code", "message", "field" } }` and status codes (200, 201, 400, 404, 422). <!-- sdd-owner: implementation -->

---

## Slice B: Show Items, Audio Upload, Range Streaming & Reordering

### 6. Show Item Domain Entities and Validation
- [ ] RED: Write unit tests in `tests/domain_item_tests.rs` verifying invariants for `ShowItem`, `ShowItemPayload` (`Track`, `Lyrics`, `Note`), `ShowItemKind`, and `MoveDirection`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implement domain types in `src/domain/show_item.rs`, `src/domain/show_item_kind.rs`, `src/domain/move_direction.rs`, and `src/domain/audio_format.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Verify validation rules: multiline lyrics non-empty, note non-empty, track requiring valid metadata and audio key reference. <!-- sdd-owner: implementation -->

### 7. Show Item Repository with Atomic Append, Swap and Compact
- [ ] RED: Write repository tests in `tests/show_item_repository_tests.rs` testing append to end (`COALESCE(MAX)+1`), atomic swap between neighbors with `BEGIN IMMEDIATE`, and deletion with index recompaction. <!-- sdd-owner: implementation -->
- [ ] GREEN: Define ports in `src/ports/show_item_reader.rs`, `src/ports/show_item_writer.rs`, and implement `SqliteShowItemRepository` in `src/infra/sqlite/show_item_repository.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Add concurrency and boundary tests for swap (first item up rejected, last item down rejected with boundary error; rapid successive swaps maintaining contiguous `0..N-1` indices). <!-- sdd-owner: implementation -->

### 8. Local Audio Storage, Format Detection and Cleanup Reconciliation
- [ ] RED: Write tests in `tests/audio_storage_tests.rs` covering magic byte format detection (MP3 ID3/frame, WAV RIFF, M4A ftyp), path traversal resistance via safe generated keys, and pending cleanup queue. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implement `src/infra/filesystem/generated_audio_key.rs`, `src/infra/filesystem/detect_audio_format.rs`, `src/infra/filesystem/local_audio_storage.rs`, `src/infra/sqlite/cleanup_repository.rs`, and `src/infra/filesystem/reconcile_audio_files.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Test cleanup queue behavior when physical file deletion fails post-commit (record retained in `pending_audio_deletions` for startup retry without breaking database state). <!-- sdd-owner: implementation -->

### 9. Show Item and Audio Use Cases
- [ ] RED: Write unit tests in `tests/item_use_cases_tests.rs` for `AddLyrics`, `AddNote`, `UploadTrack`, `ReorderItem`, `DeleteItem`, and `OpenTrackAudio`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implement use case modules in `src/use_cases/add_lyrics.rs`, `add_note.rs`, `upload_track.rs`, `reorder_item.rs`, `delete_item.rs`, and `open_track_audio.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Test compensation logic: failed track database insertion triggers immediate removal of staged audio file from disk. <!-- sdd-owner: implementation -->

### 10. HTTP Range Parsing and Streaming Handler
- [ ] RED: Write unit tests in `tests/range_parser_tests.rs` and integration tests in `tests/audio_stream_api_tests.rs` for `parse_range` and `GET /api/tracks/:id/audio` (full request 200, byte range 206, out of bounds 416, HEAD request without body). <!-- sdd-owner: implementation -->
- [ ] GREEN: Implement pure range parser in `src/handlers/audio/parse_range.rs` and audio stream handler in `src/handlers/audio/stream.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Verify RFC 9110 compliance: `Accept-Ranges: bytes`, `Content-Range: bytes 0-1023/total`, ignore malformed or multi-range headers with full 200 response, and handle missing audio files with 404. <!-- sdd-owner: implementation -->

### 11. Item Management and Multipart Upload HTTP Handlers
- [ ] RED: Write HTTP integration tests in `tests/item_api_tests.rs` for adding text items, multipart track upload, reordering, and item deletion. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implement `src/handlers/items/add_text.rs`, `src/handlers/items/upload_track.rs` (with route-scoped 101 MiB body limit and exact 100 MiB byte counter), `src/handlers/items/reorder.rs`, and `src/handlers/items/delete.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Verify that payload exceeding 100 MiB returns 413, unsupported format returns 415, empty title returns 422, and reorder boundary returns 409. <!-- sdd-owner: implementation -->

---

## Slice C: Micro-Modular Frontend UI & Exclusive Audio Player

### 12. Static Assets Serving and Application Entry Point
- [ ] RED: Write HTTP integration tests in `tests/static_assets_tests.rs` verifying `/` serves `index.html` with `Cache-Control: no-cache` and `/assets/*` serves files from `frontend/`. <!-- sdd-owner: implementation -->
- [ ] GREEN: Implement static file routing in `src/handlers/static_assets.rs`, wire into `src/router.rs`, implement `src/config.rs` and executable entry point `src/main.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE/REFACTOR: Create minimal `frontend/index.html` and verify server startup and graceful asset fallback via `cargo test`. <!-- sdd-owner: implementation -->

### 13. CSS Design Tokens, Reset and Atomic Styles
- [ ] Implement design tokens in `frontend/styles/tokens/colors.css`, `spacing.css`, `typography.css`, `elevations.css`, and `motion.css`. <!-- sdd-owner: implementation -->
- [ ] Implement base and layout stylesheets in `frontend/styles/base/reset.css`, `typography.css`, `layouts/shell-layout.css`, and `timeline-layout.css`. <!-- sdd-owner: implementation -->
- [ ] Implement atomic component stylesheets in `frontend/styles/atoms/` (`button.css`, `icon.css`, `text-input.css`, `text-area.css`, `range-input.css`, `status-message.css`, `spinner.css`), `molecules/`, `organisms/`, `views/`, and bundle in `frontend/styles/index.css`. <!-- sdd-owner: implementation -->

### 14. Frontend Core Utilities, Signals and Headless Behaviors
- [ ] Implement DOM and formatting utilities in `frontend/js/utils/dom/create-element.js`, `frontend/js/utils/format/duration.js`, and `frontend/js/utils/validation/trim-required.js`. <!-- sdd-owner: implementation -->
- [ ] Implement reactive micro-signals in `frontend/js/state/create-signal.js`, `frontend/js/state/active-route.js`, and `frontend/js/state/playback-state.js`. <!-- sdd-owner: implementation -->
- [ ] Implement headless behaviors in `frontend/js/behaviors/create-audio-controller.js` (enforcing exactly one `HTMLAudioElement` and never autoplaying), `bind-dialog-dismiss.js`, and `bind-form-submit.js`. <!-- sdd-owner: implementation -->

### 15. Frontend Data Client Actions and Hash Router
- [ ] Implement generic JSON and multipart fetch client in `frontend/js/data/request-json.js`. <!-- sdd-owner: implementation -->
- [ ] Implement isolated presentation API actions in `frontend/js/data/list-presentations.js`, `create-presentation.js`, `get-presentation.js`, `rename-presentation.js`, and `delete-presentation.js`. <!-- sdd-owner: implementation -->
- [ ] Implement isolated item API actions in `frontend/js/data/add-text-item.js`, `upload-track.js`, `move-item.js`, and `delete-item.js`. <!-- sdd-owner: implementation -->
- [ ] Implement pure hash routing in `frontend/js/router/parse-route.js` and `frontend/js/router/navigate.js` supporting `#/presentations` and `#/presentations/:id`. <!-- sdd-owner: implementation -->

### 16. UI Atoms and Molecules
- [ ] Implement atomic UI components in `frontend/js/components/atoms/button.js`, `icon.js`, `text-input.js`, `text-area.js`, `range-input.js`, `status-message.js`, and `spinner.js`. <!-- sdd-owner: implementation -->
- [ ] Implement molecular UI components in `frontend/js/components/molecules/icon-button.js`, `confirmation-dialog.js`, `item-type-selector.js`, `reorder-controls.js`, `playback-controls.js`, and `empty-state.js`. <!-- sdd-owner: implementation -->

### 17. UI Organisms, Layouts and Views
- [ ] Implement top-level organisms in `frontend/js/components/organisms/app-topbar.js`, `presentation-card.js`, `presentation-form-dialog.js`, `track-item.js`, `lyrics-item.js`, `note-item.js`, and `add-item-dialog.js`. <!-- sdd-owner: implementation -->
- [ ] Implement layout components in `frontend/js/components/layouts/shell-layout.js` and `timeline-layout.js`. <!-- sdd-owner: implementation -->
- [ ] Implement catalog and presentation views in `frontend/js/components/views/presentations-view.js` and `presentation-detail-view.js`. <!-- sdd-owner: implementation -->

### 18. Frontend Orchestration Controllers and Composition Root
- [ ] Implement catalog controller in `frontend/js/controllers/presentations-controller.js` connecting API actions, dialogs, empty states, and view updates. <!-- sdd-owner: implementation -->
- [ ] Implement presentation detail controller in `frontend/js/controllers/presentation-detail-controller.js` wiring timeline items, audio controller, reordering, item creation dialogs, and deletion confirmations. <!-- sdd-owner: implementation -->
- [ ] Implement composition root in `frontend/js/main.js` binding hash change events, initializing the shared audio controller, and mounting views into the root shell element. <!-- sdd-owner: implementation -->

### 19. End-to-End Verification and Acceptance
- [ ] Run full automated test suite via `cargo test` verifying all unit, repository, and HTTP service tests pass without warnings. <!-- sdd-owner: implementation -->
- [ ] Perform manual end-to-end verification of catalog workflows: create presentation, rename presentation, empty catalog state, and delete presentation with cascade cleanup confirmation. <!-- sdd-owner: implementation -->
- [ ] Perform manual end-to-end verification of show execution: vertical timeline rendering, adding lyrics/note/track, 44x44px touch targets, mobile seek via range streaming, exclusive single-track playback, stop-on-navigation, and atomic up/down reordering. <!-- sdd-owner: implementation -->

---

## Review Workload and Slicing Recommendation

- **Estimated changed lines:** ~2100–2600 lines across backend (Rust + SQL) and frontend (Vanilla JS + CSS).
- **400-line budget risk:** High.
- **Chained PRs recommended:** Yes.
- **Delivery strategy:** `ask-on-risk` (pause before apply to approve chain strategy).
- **Proposed chain slices:**
  1. **PR 1 (Slice A):** Backend Foundation, Domain, SQLite Storage & Presentations API (~750 lines).
  2. **PR 2 (Slice B):** Show Items, Audio Upload, Range Streaming & Reordering (~900 lines).
  3. **PR 3 (Slice C):** Micro-Modular Frontend UI & Exclusive Audio Player (~950 lines).
- **Decision needed before apply:** Yes (human confirmation required under `ask-on-risk` before implementing chained PRs).
