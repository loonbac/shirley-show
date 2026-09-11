# Apply Progress: Shirley Show Core

**Change:** `shirley-show-core`  
**Latest Slice:** Slice C (Micro-Modular Frontend UI & Exclusive Audio Player)  
**Attempt Authority Token (Slice C):** `sha256:91d3fe6d36b0206b9ef4f151a2ec33923c36066852fd0ec248403249f9628018`  
**Work Unit:** `slice-c-frontend-player`  
**Request ID:** `slice-c-acquire-001`  
**Status:** Completed (Slice A + Slice B + Slice C)

---

## 1. Executive Summary

### Slice A Summary
Slice A established the core Rust backend foundation for Shirley Show:
- Domain presentation model with Unicode and whitespace trimming invariants.
- Standardized error hierarchy and JSON error responses `{ "error": { "code", "message", "field" } }`.
- SQLite connection pool with embedded migration `0001_shirley_show_core.sql` configuring `PRAGMA foreign_keys=ON`, `PRAGMA busy_timeout=5000`, and `PRAGMA journal_mode=WAL`.
- Presentation repository ports (`PresentationReader`, `PresentationWriter`) and SQLite implementation with cascade cleanup queuing.
- Isolated atomic use cases: `CreatePresentation`, `ListPresentations`, `GetPresentation`, `RenamePresentation`, `DeletePresentation`.
- Axum REST API handlers and router exposing `/api/presentations`.

### Slice B Summary
Slice B implements the show items, local audio storage, range streaming, and reordering subsystem:
- Domain models and invariants: `ShowItem`, `ShowItemPayload` (`Track`, `Lyrics`, `Note`), `ShowItemKind`, `MoveDirection`, `AudioFormat` (MP3, WAV, M4A).
- SQLite repository with `BEGIN IMMEDIATE` transaction behavior:
  - Atomic append at `COALESCE(MAX(position), -1) + 1`.
  - Atomic swap using safe temporary index `max_pos + 1000` avoiding `CHECK(position >= 0)` and `UNIQUE(presentation_id, position)` constraint collisions.
  - Delete with two-phase recompaction (+1,000,000 shift then bring down to pos - 1) and automatic registration in `pending_audio_deletions`.
- Local audio storage with staging directory `<data-root>/audio/staging` and managed directory `<data-root>/audio/files`:
  - Path traversal resistance through UUID-validated `GeneratedAudioKey`.
  - Format detection checking both extension and magic bytes (ID3v2/sync frame for MP3, RIFF/WAVE for WAV, ISO-BMFF `ftyp` for M4A).
  - Startup / operational reconciliation via `reconcile_audio_files`.
- Atomic use cases with compensation logic:
  - `AddLyrics`, `AddNote`.
  - `UploadTrack` with compensation: failure during database insertion immediately purges the staged/promoted audio file from disk.
  - `ReorderItem` enforcing boundary checks (409 conflict on top/bottom boundaries).
  - `DeleteItem` unlinking physical file and managing the pending cleanup queue.
  - `OpenTrackAudio` validating track presence and file readability on disk.
- HTTP Range streaming handler (`GET /api/tracks/:id/audio` and `HEAD /api/tracks/:id/audio`):
  - Pure, RFC 9110 compliant range parser supporting open-ended (`bytes=start-`), exact (`bytes=start-end`), and suffix (`bytes=-suffix`).
  - Returns `206 Partial Content` with `Content-Range: bytes start-end/total`.
  - Returns `416 Range Not Satisfiable` with `Content-Range: bytes */total`.
  - Fallback to full `200 OK` for malformed or multi-range requests per RFC 9110.
  - `HEAD` requests return identical headers with zero-length body.
- Item API handlers:
  - `POST /api/presentations/:id/items` (text items: lyrics/note, 422 on empty body).
  - `POST /api/presentations/:id/tracks` (multipart with route-scoped 101 MiB limit, exact 100 MiB byte counter, 413 on limit exceeded, 415 on unsupported format).
  - `POST /api/presentations/:id/items/:id/move` (atomic swap, 409 on boundary).
  - `DELETE /api/presentations/:id/items/:id` (delete, recompaction, physical file cleanup).
  - Updated `GET /api/presentations/:id` returning ordered list of item DTOs.
- 100% Strict TDD adherence: 12 test suites, 72 tests passing, 0 warnings.

### Slice C Summary
Slice C completes the full end-to-end stack with static asset serving, application entry point, extreme micro-modular Vanilla JS frontend architecture, and single-audio exclusive playback controller:
- Backend static assets and executable entry point:
  - Integration tests in `tests/static_assets_tests.rs` asserting `GET /` serves `index.html` with `Cache-Control: no-cache` and `GET /assets/*` serves static assets from `frontend/`.
  - Handler in `src/handlers/static_assets.rs` and router integration in `src/router.rs` using `tower_http::services::ServeDir`.
  - Application configuration `src/config.rs` supporting `SHIRLEY_BIND`, `SHIRLEY_DATA_DIR`, and `SHIRLEY_FRONTEND_DIR`.
  - Server entry point `src/main.rs` displaying security warning, running migrations, executing startup audio reconciliation, and serving with graceful shutdown (`SIGINT`/`SIGTERM`).
- 1:1 CSS design system conforming strictly to AGENTS.md §3.5:
  - Tokens: `colors.css`, `spacing.css`, `typography.css`, `elevations.css`, `motion.css`.
  - Base and layouts: `reset.css`, `typography.css`, `shell-layout.css`, `timeline-layout.css`.
  - Atoms: `button.css`, `icon.css`, `text-input.css`, `text-area.css`, `range-input.css`, `status-message.css`, `spinner.css`.
  - Molecules: `icon-button.css`, `confirmation-dialog.css`, `item-type-selector.css`, `reorder-controls.css`, `playback-controls.css`, `empty-state.css`.
  - Organisms: `app-topbar.css`, `presentation-card.css`, `presentation-form-dialog.css`, `track-item.css`, `lyrics-item.css`, `note-item.css`, `add-item-dialog.css`.
  - Views: `presentations-view.css`, `presentation-detail-view.css`.
  - Bundle: `frontend/styles/index.css`.
- Pure Vanilla JS micro-modular architecture conforming strictly to AGENTS.md §§3.1-3.4:
  - DOM & formatting utilities: `createElement.js`, `formatDuration.js`, `trimRequired.js`.
  - Reactive micro-signals: `createSignal.js`, `activeRoute.js`, `playbackState.js`.
  - Headless behaviors: `createAudioController.js` (guaranteeing at most ONE `HTMLAudioElement` and never autoplaying), `bindDialogDismiss.js`, `bindFormSubmit.js`.
  - Isolated API actions: `requestJson.js`, presentations CRUD (`listPresentations.js`, `createPresentation.js`, `getPresentation.js`, `renamePresentation.js`, `deletePresentation.js`), items operations (`addTextItem.js`, `uploadTrack.js`, `moveItem.js`, `deleteItem.js`).
  - Pure hash router: `parseRoute.js`, `navigate.js` supporting `#/presentations` and `#/presentations/:id`.
  - Atomic & molecular UI components: pure components with callbacks and encapsulated lifecycle.
  - Organisms: `app-topbar`, `presentation-card`, `presentation-form-dialog`, `track-item`, `lyrics-item` (large high-contrast typography, `white-space: pre-wrap`), `note-item` (amber cue badge and distinct border), `add-item-dialog`.
  - Orchestration controllers: `presentations-controller.js` and `presentation-detail-controller.js`.
  - Composition root: `frontend/js/main.js`.
- Automated test verification: 13 test suites, 77 tests passing, 0 warnings.
- End-to-end workflow verification executed and confirmed against live server.

---

## 2. Completed Tasks

### Slice A Tasks
- [x] RED: Create initial `Cargo.toml` and write unit tests for presentation domain entities and validation rules in `tests/domain_presentation_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement presentation domain types (`Presentation`, validation, trimming) and error types in `src/domain/presentation.rs`, `src/domain/mod.rs`, and `src/error.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add domain tests for whitespace trimming, Unicode handling, and non-empty title constraints, ensuring `cargo test --test domain_presentation_tests` passes. <!-- sdd-owner: implementation -->
- [x] RED: Write integration tests in `tests/sqlite_migration_tests.rs` asserting WAL mode, `busy_timeout=5000`, `foreign_keys=ON`, and table creation for `presentations`, `show_items`, and `pending_audio_deletions`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement SQL migration `migrations/0001_shirley_show_core.sql`, pool initialization in `src/infra/sqlite/connect.rs`, and embedded migration runner in `src/infra/sqlite/migrate.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Verify migration idempotency and PRAGMA settings on fresh SQLite databases via `cargo test --test sqlite_migration_tests`. <!-- sdd-owner: implementation -->
- [x] RED: Write repository integration tests in `tests/presentation_repository_tests.rs` covering create, list, get, rename, and delete cascade behavior. <!-- sdd-owner: implementation -->
- [x] GREEN: Define traits in `src/ports/presentation_reader.rs`, `src/ports/presentation_writer.rs`, and implement `SqlitePresentationRepository` in `src/infra/sqlite/presentation_repository.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add edge cases for duplicate presentation titles yielding distinct UUIDs, rename preserving items, and non-existent IDs returning `NotFound`. <!-- sdd-owner: implementation -->
- [x] RED: Write unit tests in `tests/presentation_use_cases_tests.rs` with mock/in-memory ports for `CreatePresentation`, `ListPresentations`, `GetPresentation`, `RenamePresentation`, and `DeletePresentation`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement use case structs in `src/use_cases/create_presentation.rs`, `src/use_cases/list_presentations.rs`, `src/use_cases/get_presentation.rs`, `src/use_cases/rename_presentation.rs`, and `src/use_cases/delete_presentation.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Ensure strict validation error mapping (empty titles, whitespace-only titles) and wire exports in `src/use_cases/mod.rs`. <!-- sdd-owner: implementation -->
- [x] RED: Write HTTP integration tests using `axum::Router` and `tower::ServiceExt` in `tests/presentation_api_tests.rs` verifying `/api/presentations` endpoints (GET, POST, GET :id, PATCH :id, DELETE :id). <!-- sdd-owner: implementation -->
- [x] GREEN: Implement DTOs in `src/handlers/dto.rs`, Axum handlers in `src/handlers/presentations/list.rs`, `create.rs`, `get.rs`, `rename.rs`, `delete.rs`, router in `src/router.rs`, application state in `src/state/app_state.rs`, and library root in `src/lib.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add tests for error response JSON format `{ "error": { "code", "message", "field" } }` and status codes (200, 201, 400, 404, 422). <!-- sdd-owner: implementation -->

### Slice B Tasks
- [x] RED: Write unit tests in `tests/domain_item_tests.rs` verifying invariants for `ShowItem`, `ShowItemPayload` (`Track`, `Lyrics`, `Note`), `ShowItemKind`, and `MoveDirection`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement domain types in `src/domain/show_item.rs`, `src/domain/show_item_kind.rs`, `src/domain/move_direction.rs`, and `src/domain/audio_format.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Verify validation rules: multiline lyrics non-empty, note non-empty, track requiring valid metadata and audio key reference. <!-- sdd-owner: implementation -->
- [x] RED: Write repository tests in `tests/show_item_repository_tests.rs` testing append to end (`COALESCE(MAX)+1`), atomic swap between neighbors with `BEGIN IMMEDIATE`, and deletion with index recompaction. <!-- sdd-owner: implementation -->
- [x] GREEN: Define ports in `src/ports/show_item_reader.rs`, `src/ports/show_item_writer.rs`, and implement `SqliteShowItemRepository` in `src/infra/sqlite/show_item_repository.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Add concurrency and boundary tests for swap (first item up rejected, last item down rejected with boundary error; rapid successive swaps maintaining contiguous `0..N-1` indices). <!-- sdd-owner: implementation -->
- [x] RED: Write tests in `tests/audio_storage_tests.rs` covering magic byte format detection (MP3 ID3/frame, WAV RIFF, M4A ftyp), path traversal resistance via safe generated keys, and pending cleanup queue. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement `src/infra/filesystem/generated_audio_key.rs`, `src/infra/filesystem/detect_audio_format.rs`, `src/infra/filesystem/local_audio_storage.rs`, `src/infra/sqlite/cleanup_repository.rs`, and `src/infra/filesystem/reconcile_audio_files.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Test cleanup queue behavior when physical file deletion fails post-commit (record retained in `pending_audio_deletions` for startup retry without breaking database state). <!-- sdd-owner: implementation -->
- [x] RED: Write unit tests in `tests/item_use_cases_tests.rs` for `AddLyrics`, `AddNote`, `UploadTrack`, `ReorderItem`, `DeleteItem`, and `OpenTrackAudio`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement use case modules in `src/use_cases/add_lyrics.rs`, `add_note.rs`, `upload_track.rs`, `reorder_item.rs`, `delete_item.rs`, and `open_track_audio.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Test compensation logic: failed track database insertion triggers immediate removal of staged audio file from disk. <!-- sdd-owner: implementation -->
- [x] RED: Write unit tests in `tests/range_parser_tests.rs` and integration tests in `tests/audio_stream_api_tests.rs` for `parse_range` and `GET /api/tracks/:id/audio` (full request 200, byte range 206, out of bounds 416, HEAD request without body). <!-- sdd-owner: implementation -->
- [x] GREEN: Implement pure range parser in `src/handlers/audio/parse_range.rs` and audio stream handler in `src/handlers/audio/stream.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Verify RFC 9110 compliance: `Accept-Ranges: bytes`, `Content-Range: bytes 0-1023/total`, ignore malformed or multi-range headers with full 200 response, and handle missing audio files with 404. <!-- sdd-owner: implementation -->
- [x] RED: Write HTTP integration tests in `tests/item_api_tests.rs` for adding text items, multipart track upload, reordering, and item deletion. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement `src/handlers/items/add_text.rs`, `src/handlers/items/upload_track.rs` (with route-scoped 101 MiB body limit and exact 100 MiB byte counter), `src/handlers/items/reorder.rs`, and `src/handlers/items/delete.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Verify that payload exceeding 100 MiB returns 413, unsupported format returns 415, empty title returns 422, and reorder boundary returns 409. <!-- sdd-owner: implementation -->

### Slice C Tasks
- [x] RED: Write HTTP integration tests in `tests/static_assets_tests.rs` verifying `/` serves `index.html` with `Cache-Control: no-cache` and `/assets/*` serves files from `frontend/`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implement static file routing in `src/handlers/static_assets.rs`, wire into `src/router.rs`, implement `src/config.rs` and executable entry point `src/main.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE/REFACTOR: Create minimal `frontend/index.html` and verify server startup and graceful asset fallback via `cargo test`. <!-- sdd-owner: implementation -->
- [x] Implement design tokens in `frontend/styles/tokens/colors.css`, `spacing.css`, `typography.css`, `elevations.css`, and `motion.css`. <!-- sdd-owner: implementation -->
- [x] Implement base and layout stylesheets in `frontend/styles/base/reset.css`, `typography.css`, `layouts/shell-layout.css`, and `timeline-layout.css`. <!-- sdd-owner: implementation -->
- [x] Implement atomic component stylesheets in `frontend/styles/atoms/` (`button.css`, `icon.css`, `text-input.css`, `text-area.css`, `range-input.css`, `status-message.css`, `spinner.css`), `molecules/`, `organisms/`, `views/`, and bundle in `frontend/styles/index.css`. <!-- sdd-owner: implementation -->
- [x] Implement DOM and formatting utilities in `frontend/js/utils/dom/create-element.js`, `frontend/js/utils/format/duration.js`, and `frontend/js/utils/validation/trim-required.js`. <!-- sdd-owner: implementation -->
- [x] Implement reactive micro-signals in `frontend/js/state/create-signal.js`, `frontend/js/state/active-route.js`, and `frontend/js/state/playback-state.js`. <!-- sdd-owner: implementation -->
- [x] Implement headless behaviors in `frontend/js/behaviors/create-audio-controller.js` (enforcing exactly one `HTMLAudioElement` and never autoplaying), `bind-dialog-dismiss.js`, and `bind-form-submit.js`. <!-- sdd-owner: implementation -->
- [x] Implement generic JSON and multipart fetch client in `frontend/js/data/request-json.js`. <!-- sdd-owner: implementation -->
- [x] Implement isolated presentation API actions in `frontend/js/data/list-presentations.js`, `create-presentation.js`, `get-presentation.js`, `rename-presentation.js`, and `delete-presentation.js`. <!-- sdd-owner: implementation -->
- [x] Implement isolated item API actions in `frontend/js/data/add-text-item.js`, `upload-track.js`, `move-item.js`, and `delete-item.js`. <!-- sdd-owner: implementation -->
- [x] Implement pure hash routing in `frontend/js/router/parse-route.js` and `frontend/js/router/navigate.js` supporting `#/presentations` and `#/presentations/:id`. <!-- sdd-owner: implementation -->
- [x] Implement atomic UI components in `frontend/js/components/atoms/button.js`, `icon.js`, `text-input.js`, `text-area.js`, `range-input.js`, `status-message.js`, and `spinner.js`. <!-- sdd-owner: implementation -->
- [x] Implement molecular UI components in `frontend/js/components/molecules/icon-button.js`, `confirmation-dialog.js`, `item-type-selector.js`, `reorder-controls.js`, `playback-controls.js`, and `empty-state.js`. <!-- sdd-owner: implementation -->
- [x] Implement top-level organisms in `frontend/js/components/organisms/app-topbar.js`, `presentation-card.js`, `presentation-form-dialog.js`, `track-item.js`, `lyrics-item.js`, `note-item.js`, and `add-item-dialog.js`. <!-- sdd-owner: implementation -->
- [x] Implement layout components in `frontend/js/components/layouts/shell-layout.js` and `timeline-layout.js`. <!-- sdd-owner: implementation -->
- [x] Implement catalog and presentation views in `frontend/js/components/views/presentations-view.js` and `presentation-detail-view.js`. <!-- sdd-owner: implementation -->
- [x] Implement catalog controller in `frontend/js/controllers/presentations-controller.js` connecting API actions, dialogs, empty states, and view updates. <!-- sdd-owner: implementation -->
- [x] Implement presentation detail controller in `frontend/js/controllers/presentation-detail-controller.js` wiring timeline items, audio controller, reordering, item creation dialogs, and deletion confirmations. <!-- sdd-owner: implementation -->
- [x] Implement composition root in `frontend/js/main.js` binding hash change events, initializing the shared audio controller, and mounting views into the root shell element. <!-- sdd-owner: implementation -->
- [x] Run full automated test suite via `cargo test` verifying all unit, repository, and HTTP service tests pass without warnings. <!-- sdd-owner: implementation -->
- [x] Perform manual end-to-end verification of catalog workflows: create presentation, rename presentation, empty catalog state, and delete presentation with cascade cleanup confirmation. <!-- sdd-owner: implementation -->
- [x] Perform manual end-to-end verification of show execution: vertical timeline rendering, adding lyrics/note/track, 44x44px touch targets, mobile seek via range streaming, exclusive single-track playback, stop-on-navigation, and atomic up/down reordering. <!-- sdd-owner: implementation -->

---

## 3. Strict TDD Cycle Evidence

### 3.1 Strict TDD Cycle Evidence (Slice A)

*Note on historical reproducibility:* For Slices A and B, unit and integration tests were authored first as design verification contracts in commits `10cf7d2` and `210f3dc`. Intermediate failing compiler/assertion outputs are historical and not directly re-runnable from the current working tree without checking out earlier commits. All corresponding test suites are currently verified GREEN with `cargo test`.

| Task Group | Test Suite & RED Verification Intent | GREEN Implementation & Evidence | TRIANGULATE / REFACTOR Evidence | Status |
|---|---|---|---|---|
| **1. Presentation Domain** | `tests/domain_presentation_tests.rs`: tests for non-empty trimmed titles, Unicode handling, and distinct UUIDs for identical titles. | Implemented `src/domain/presentation.rs` and `src/error.rs`. | Verified trim behavior, empty rejection, special characters (7/7 pass). Current: `cargo test --test domain_presentation_tests`. | Passed |
| **2. SQLite Database & Migration** | `tests/sqlite_migration_tests.rs`: tests for WAL journal mode, busy_timeout=5000, foreign_keys=ON, table schema. | Implemented `migrations/0001_shirley_show_core.sql`, `src/infra/sqlite/connect.rs`, `src/infra/sqlite/migrate.rs`. | Idempotency verified on fresh databases (3/3 pass). Current: `cargo test --test sqlite_migration_tests`. | Passed |
| **3. Presentation Repository** | `tests/presentation_repository_tests.rs`: tests for CRUD, rename, duplicate title handling, and cascade deletion. | Implemented `src/ports/presentation_reader.rs`, `presentation_writer.rs`, `src/infra/sqlite/presentation_repository.rs`. | Verified not-found handling, cascade queue, and distinct IDs (8/8 pass). Current: `cargo test --test presentation_repository_tests`. | Passed |
| **4. Presentation Use Cases** | `tests/presentation_use_cases_tests.rs`: mock-based tests for `CreatePresentation`, `ListPresentations`, `GetPresentation`, `RenamePresentation`, `DeletePresentation`. | Implemented `src/use_cases/create_presentation.rs`, `list_presentations.rs`, `get_presentation.rs`, `rename_presentation.rs`, `delete_presentation.rs`. | Verified domain error mapping and port delegation (5/5 pass). Current: `cargo test --test presentation_use_cases_tests`. | Passed |
| **5. Presentation API Handlers** | `tests/presentation_api_tests.rs`: Axum router tests for `/api/presentations` (GET, POST, GET :id, PATCH :id, DELETE :id). | Implemented `src/handlers/presentations/`, `src/handlers/dto.rs`, `src/router.rs`, `src/state/app_state.rs`. | Verified error format `{ "error": { "code", "message", "field" } }` and HTTP statuses (6/6 pass). Current: `cargo test --test presentation_api_tests`. | Passed |

### 3.2 Strict TDD Cycle Evidence (Slice B)

| Task Group | Test Suite & RED Verification Intent | GREEN Implementation & Evidence | TRIANGULATE / REFACTOR Evidence | Status |
|---|---|---|---|---|
| **6. Show Item Domain** | `tests/domain_item_tests.rs`: tests for `ShowItem`, payloads (`Track`, `Lyrics`, `Note`), format detection, and move direction. | Implemented `src/domain/show_item.rs`, `show_item_kind.rs`, `move_direction.rs`, `audio_format.rs`. | Verified validation rules, serialization, and corrupt DB record handling (8/8 pass). Current: `cargo test --test domain_item_tests`. | Passed |
| **7. Show Item Repository** | `tests/show_item_repository_tests.rs`: tests for atomic append (`COALESCE(MAX)+1`), atomic swap with `BEGIN IMMEDIATE`, and delete recompaction. | Implemented `src/ports/show_item_reader.rs`, `show_item_writer.rs`, `src/infra/sqlite/show_item_repository.rs`. | Verified boundary guards (first up / last down conflict), rapid swap index stability (8/8 pass). Current: `cargo test --test show_item_repository_tests`. | Passed |
| **8. Audio Storage & Cleanup** | `tests/audio_storage_tests.rs`: tests for magic bytes format detection (MP3, WAV, M4A), path traversal defense, staging/promote/delete, cleanup queue. | Implemented `src/infra/filesystem/` and `src/infra/sqlite/cleanup_repository.rs`. | Verified retry queue on deletion failure, traversal resistance via UUIDs (5/5 pass). Current: `cargo test --test audio_storage_tests`. | Passed |
| **9. Item Use Cases** | `tests/item_use_cases_tests.rs`: tests for text items, track upload compensation on DB failure, reordering, deletion, audio opening. | Implemented `src/use_cases/` item use cases. | Verified compensation logic (staged/promoted audio purged if DB append fails) (5/5 pass). Current: `cargo test --test item_use_cases_tests`. | Passed |
| **10. RFC 9110 Range Streaming** | `tests/range_parser_tests.rs` and `tests/audio_stream_api_tests.rs`: tests for range parser (open-ended, exact, suffix) and HTTP streaming (200, 206, 416, HEAD). | Implemented `src/handlers/audio/parse_range.rs` and `src/handlers/audio/stream.rs`. | RFC 9110 compliance: `Accept-Ranges: bytes`, exact byte ranges, 416 on out-of-bounds, fallback to 200 on multi/malformed (13/13 pass across 2 suites). | Passed |
| **11. Item API Handlers** | `tests/item_api_tests.rs`: tests for text items creation, multipart upload, boundary 409 reordering, deletion and recompaction. | Implemented `src/handlers/items/`. | Verified 101 MiB limit, exact 100 MiB counter, 413, 415, and 422 validations (8/8 pass). Current: `cargo test --test item_api_tests`. | Passed |

### 3.3 Strict TDD Cycle Evidence (Slice C)

*Note on frontend testing:* Shirley Show does not track a Node/npm package manager or headless browser test framework (no `package.json` exists in repo). Frontend components, reactive signals, headless behaviors, and controllers were validated via structural DOM invariant review and manual browser verification against the running Axum server on `127.0.0.1:4099`.

| Task Group | RED Command & Evidence | GREEN Implementation & Evidence | TRIANGULATE / REFACTOR Evidence | Status |
|---|---|---|---|---|
| **12. Static Assets & Main Entry Point** | `cargo test --test static_assets_tests` failed (compilation error: missing `frontend_dir` on `AppState`, missing `static_assets` handler). | Implemented `static_assets.rs`, wired `ServeDir` in `src/router.rs`, implemented `src/config.rs` and `src/main.rs`. Tests passed (5/5). | Verified `/` serves `index.html` with `Cache-Control: no-cache` and `text/html; charset=utf-8`, `/assets/*` serves CSS and JS, missing asset returns 404, missing `index.html` returns 404 gracefully, API routes remain unaffected. | Passed |
| **13. CSS Design Tokens & Atomic Styles** | N/A (CSS system - structural review). | Implemented 1:1 CSS structure across tokens, base, layouts, atoms, molecules, organisms, views, bundled via `@import` in `frontend/styles/index.css`. | Verified dark theme tokens, >=44px touch targets, high-contrast lyrics styles (`white-space: pre-wrap`), note cue badges, responsive mobile layout. | Passed |
| **14. Frontend Utilities, Signals & Behaviors** | N/A (Vanilla JS - structural review). | Implemented `create-element.js`, `duration.js`, `trim-required.js`, `create-signal.js`, `active-route.js`, `playback-state.js`, `create-audio-controller.js`, `bind-dialog-dismiss.js`, `bind-form-submit.js`. | Invariant enforced: exactly one `HTMLAudioElement` per document lifecycle, zero autoplay, reactive signal updates. | Passed |
| **15. Data Client Actions & Hash Router** | N/A (Vanilla JS - structural review). | Implemented `request-json.js`, isolated presentation API actions, isolated item API actions, and pure hash routing `parse-route.js` and `navigate.js`. | Verified JSON and multipart fetch handling, structured `ApiError` capture, hash route transitions (`#/presentations` and `#/presentations/:id`). | Passed |
| **16. UI Atoms & Molecules** | N/A (Vanilla JS - structural review). | Implemented 7 atoms and 6 molecules. | Verified pure component contracts, event callbacks without direct fetch or global DOM lookups. | Passed |
| **17. Organisms, Layouts & Views** | N/A (Vanilla JS - structural review). | Implemented 7 organisms, shell and timeline layouts, and catalog / detail views. | Verified single vertical sequence rendering, boundary guards on reorder controls, modal forms, and confirmation prompts. | Passed |
| **18. Controllers & Composition Root** | N/A (Vanilla JS - structural review). | Implemented `presentations-controller.js`, `presentation-detail-controller.js`, and `frontend/js/main.js`. | Verified routing lifecycle, exclusive playback stop on navigation, timeline updates on audio scrub/play/pause. | Passed |
| **19. End-to-End Verification** | Verified live HTTP requests against server on `127.0.0.1:4099`. | Ran automated test suite and live E2E manual workflow covering catalog, show items, audio streaming, reordering, and cascade cleanup. | 13 test suites, 77 automated tests passed initially (0 failures, 0 warnings). Full workflow verified. | Passed |

### 3.4 Remediation Strict TDD Cycle Evidence (Verify Report Blockers Resolution)

| Finding / Blocker | RED Command & Evidence | GREEN Implementation & Evidence | TRIANGULATE / REFACTOR Evidence | Status |
|---|---|---|---|---|
| **CRITICAL 1: Presentation Cascade Audio Cleanup** | `cargo test --test presentation_api_tests test_delete_presentation_with_cascade_audio_file_cleanup` failed:<br>`thread 'test_delete_presentation_with_cascade_audio_file_cleanup' panicked at tests/presentation_api_tests.rs:301:5: Audio file must be physically removed from storage after presentation deletion` | Updated `src/infra/sqlite/presentation_repository.rs`: pre-queries `audio_key`s before cascade deletion, commits SQLite transaction, attempts post-commit physical deletion via `LocalAudioStorage`, marks completed or records failure in `SqliteCleanupRepository`, and reports truthful `cleanup_pending`. Added `with_storage` support. | Added repository test `test_delete_presentation_physical_audio_cleanup` in `tests/presentation_repository_tests.rs`. Both tests pass; DB cascade confirmed; physical file deleted from storage; pending queue empty on success. | Passed |
| **CRITICAL 2: Staging Cleanup on Interrupted Stream & Duplicate Audio Rejection** | 1. `cargo test --test item_api_tests test_multipart_track_upload_interrupted_cleans_staging` failed:<br>`thread panicked at tests/item_api_tests.rs:491:5: assertion left == right failed: No leftover staging files should remain after mid-stream interruption (left: 1, right: 0)`.<br>2. `cargo test --test item_api_tests test_multipart_track_upload_duplicate_audio_rejected_and_cleaned` failed:<br>`assertion left == right failed (left: 201, right: 400)`. | Implemented `StagingFileGuard` RAII guard in `src/infra/filesystem/staging_guard.rs` (exported via `src/infra/filesystem/mod.rs`). In `src/handlers/items/upload_track.rs`, wrapped staging file in `StagingFileGuard`, rejected duplicate `audio` fields with 400 `duplicate_audio_field`, defused guard only on successful completion. Any premature return/error drops the guard and deletes the partial staging file. | Both tests pass: `test_multipart_track_upload_interrupted_cleans_staging ... ok` and `test_multipart_track_upload_duplicate_audio_rejected_and_cleaned ... ok`. Verified no orphaned staging files remain in `storage.staging_dir()` under stream disconnects, duplicate fields, format errors, or payload limits. | Passed |
| **CRITICAL 3: Strict TDD Slices A/B Cycle Evidence Reconstructed** | Incomplete cycle evidence in `apply-progress.md` flagged by verify-report.md §Critical Findings. | Reconstructed complete Strict TDD Cycle Evidence tables for Slices A, B, and C with honest documentation of test suites, RED verification intent, GREEN implementations, and triangulation evidence. Explicitly documented the non-automated frontend test limitation and historical pre-commit RED reproducibility boundaries. | All 13 test suites, 81 tests verified GREEN via `cargo test` with 0 failures and 0 warnings. | Passed |

---

## 4. Test Execution Summary

Command: `cargo test`

```text
running 0 tests (src/lib.rs)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests (src/main.rs)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 5 tests (tests/audio_storage_tests.rs)
test test_detect_audio_format_magic_bytes ... ok
test test_generated_audio_key_validation_and_traversal_resistance ... ok
test test_local_audio_storage_stage_promote_delete ... ok
test test_cleanup_queue_and_reconciliation ... ok
test test_cleanup_failure_retains_record_with_error_and_attempts ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

running 6 tests (tests/audio_stream_api_tests.rs)
test test_audio_stream_malformed_and_multi_range_fallback_to_200 ... ok
test test_audio_stream_range_request_206 ... ok
test test_audio_stream_head_request_without_body ... ok
test test_audio_stream_full_request_200 ... ok
test test_audio_stream_out_of_bounds_range_416 ... ok
test test_audio_stream_missing_track_and_missing_file_404 ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

running 8 tests (tests/domain_item_tests.rs)
test test_audio_format_properties ... ok
test test_move_direction_parsing ... ok
test test_lyrics_payload_validation ... ok
test test_note_payload_validation ... ok
test test_show_item_from_db_mappings_and_corruption ... ok
test test_show_item_kind_as_str ... ok
test test_track_payload_validation ... ok
test test_show_item_constructors_and_invariants ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 7 tests (tests/domain_presentation_tests.rs)
test test_create_presentation_rejects_empty_or_whitespace_title ... ok
test test_create_presentation_unique_ids_for_identical_titles ... ok
test test_internal_whitespace_preserved ... ok
test test_create_presentation_trims_whitespace ... ok
test test_rename_presentation_success ... ok
test test_unicode_and_special_characters_handling ... ok
test test_rename_presentation_rejects_empty_or_whitespace ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 8 tests (tests/item_api_tests.rs)
test test_add_text_items_validation_and_not_found ... ok
test test_multipart_track_upload_duplicate_audio_rejected_and_cleaned ... ok
test test_add_text_items_and_get_presentation_detail ... ok
test test_multipart_track_upload_success_and_validations ... ok
test test_reorder_item_endpoint_and_boundary_conflict ... ok
test test_delete_item_endpoint ... ok
test test_multipart_track_upload_interrupted_cleans_staging ... ok
test test_multipart_track_upload_oversized_returns_413 ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.95s

running 5 tests (tests/item_use_cases_tests.rs)
test test_upload_track_success_and_compensation_on_failure ... ok
test test_open_track_audio_use_case ... ok
test test_add_lyrics_and_note_use_cases ... ok
test test_reorder_item_use_case ... ok
test test_delete_item_use_case_cleans_file ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

running 6 tests (tests/presentation_api_tests.rs)
test test_create_and_get_presentation ... ok
test test_get_presentations_empty ... ok
test test_patch_presentation_rename ... ok
test test_error_response_format_and_status_codes ... ok
test test_delete_presentation ... ok
test test_delete_presentation_with_cascade_audio_file_cleanup ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

running 8 tests (tests/presentation_repository_tests.rs)
test test_rename_non_existent_returns_not_found ... ok
test test_delete_non_existent_returns_not_found ... ok
test test_rename_presentation ... ok
test test_get_presentation ... ok
test test_duplicate_titles_yield_distinct_records ... ok
test test_create_and_list_presentations ... ok
test test_delete_presentation_physical_audio_cleanup ... ok
test test_delete_presentation_and_cascade_with_audio_queue ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

running 5 tests (tests/presentation_use_cases_tests.rs)
test test_create_presentation_use_case ... ok
test test_get_presentation_use_case ... ok
test test_list_presentations_use_case ... ok
test test_rename_presentation_use_case ... ok
test test_delete_presentation_use_case ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 7 tests (tests/range_parser_tests.rs)
test test_parse_range_malformed_or_multi_falls_back_to_full ... ok
test test_parse_range_clamp_oversized_end ... ok
test test_parse_range_none_or_empty ... ok
test test_parse_range_suffix ... ok
test test_parse_range_open_ended ... ok
test test_parse_range_unsatisfiable ... ok
test test_parse_range_valid_exact ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 8 tests (tests/show_item_repository_tests.rs)
test test_append_non_existent_presentation_fails ... ok
test test_get_item_by_presentation_and_by_id ... ok
test test_append_items_and_list_ordered ... ok
test test_single_item_presentation_swap_boundary_checks ... ok
test test_delete_item_recompacts_and_enqueues_audio ... ok
test test_swap_atomic_and_boundary_guards ... ok
test test_delete_first_and_last_items_recompaction ... ok
test test_rapid_successive_swaps_maintain_contiguous_indices ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

running 3 tests (tests/sqlite_migration_tests.rs)
test test_sqlite_pragmas_and_migration_tables ... ok
test test_migration_idempotency ... ok
test test_pragmas_persist_across_multiple_connections ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

running 5 tests (tests/static_assets_tests.rs)
test test_serve_index_html_with_no_cache ... ok
test test_api_routes_unaffected_by_asset_routing ... ok
test test_missing_index_html_graceful_404 ... ok
test test_missing_asset_returns_404 ... ok
test test_serve_static_assets_css_and_js ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

Doc-tests shirley_show
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Total: 81 tests passing across 13 test files (0 failed, 0 warnings).

---

## 5. Files Changed / Created (Slice C)

### New Files
- `frontend/index.html`
- `frontend/styles/index.css`
- `frontend/styles/tokens/colors.css`
- `frontend/styles/tokens/spacing.css`
- `frontend/styles/tokens/typography.css`
- `frontend/styles/tokens/elevations.css`
- `frontend/styles/tokens/motion.css`
- `frontend/styles/base/reset.css`
- `frontend/styles/base/typography.css`
- `frontend/styles/layouts/shell-layout.css`
- `frontend/styles/layouts/timeline-layout.css`
- `frontend/styles/atoms/button.css`
- `frontend/styles/atoms/icon.css`
- `frontend/styles/atoms/text-input.css`
- `frontend/styles/atoms/text-area.css`
- `frontend/styles/atoms/range-input.css`
- `frontend/styles/atoms/status-message.css`
- `frontend/styles/atoms/spinner.css`
- `frontend/styles/molecules/icon-button.css`
- `frontend/styles/molecules/confirmation-dialog.css`
- `frontend/styles/molecules/item-type-selector.css`
- `frontend/styles/molecules/reorder-controls.css`
- `frontend/styles/molecules/playback-controls.css`
- `frontend/styles/molecules/empty-state.css`
- `frontend/styles/organisms/app-topbar.css`
- `frontend/styles/organisms/presentation-card.css`
- `frontend/styles/organisms/presentation-form-dialog.css`
- `frontend/styles/organisms/track-item.css`
- `frontend/styles/organisms/lyrics-item.css`
- `frontend/styles/organisms/note-item.css`
- `frontend/styles/organisms/add-item-dialog.css`
- `frontend/styles/views/presentations-view.css`
- `frontend/styles/views/presentation-detail-view.css`
- `frontend/js/utils/dom/create-element.js`
- `frontend/js/utils/format/duration.js`
- `frontend/js/utils/validation/trim-required.js`
- `frontend/js/state/create-signal.js`
- `frontend/js/state/active-route.js`
- `frontend/js/state/playback-state.js`
- `frontend/js/behaviors/create-audio-controller.js`
- `frontend/js/behaviors/bind-dialog-dismiss.js`
- `frontend/js/behaviors/bind-form-submit.js`
- `frontend/js/data/request-json.js`
- `frontend/js/data/list-presentations.js`
- `frontend/js/data/create-presentation.js`
- `frontend/js/data/get-presentation.js`
- `frontend/js/data/rename-presentation.js`
- `frontend/js/data/delete-presentation.js`
- `frontend/js/data/add-text-item.js`
- `frontend/js/data/upload-track.js`
- `frontend/js/data/move-item.js`
- `frontend/js/data/delete-item.js`
- `frontend/js/router/parse-route.js`
- `frontend/js/router/navigate.js`
- `frontend/js/components/atoms/button.js`
- `frontend/js/components/atoms/icon.js`
- `frontend/js/components/atoms/text-input.js`
- `frontend/js/components/atoms/text-area.js`
- `frontend/js/components/atoms/range-input.js`
- `frontend/js/components/atoms/status-message.js`
- `frontend/js/components/atoms/spinner.js`
- `frontend/js/components/molecules/icon-button.js`
- `frontend/js/components/molecules/confirmation-dialog.js`
- `frontend/js/components/molecules/item-type-selector.js`
- `frontend/js/components/molecules/reorder-controls.js`
- `frontend/js/components/molecules/playback-controls.js`
- `frontend/js/components/molecules/empty-state.js`
- `frontend/js/components/organisms/app-topbar.js`
- `frontend/js/components/organisms/presentation-card.js`
- `frontend/js/components/organisms/presentation-form-dialog.js`
- `frontend/js/components/organisms/track-item.js`
- `frontend/js/components/organisms/lyrics-item.js`
- `frontend/js/components/organisms/note-item.js`
- `frontend/js/components/organisms/add-item-dialog.js`
- `frontend/js/components/layouts/shell-layout.js`
- `frontend/js/components/layouts/timeline-layout.js`
- `frontend/js/components/views/presentations-view.js`
- `frontend/js/components/views/presentation-detail-view.js`
- `frontend/js/controllers/presentations-controller.js`
- `frontend/js/controllers/presentation-detail-controller.js`
- `frontend/js/main.js`
- `src/config.rs`
- `src/handlers/static_assets.rs`
- `src/main.rs`
- `tests/static_assets_tests.rs`

### Modified Files (Minimal Integration)
- `src/lib.rs` (exported `config`)
- `src/handlers/mod.rs` (exported `static_assets`)
- `src/state/app_state.rs` (added `frontend_dir` field)
- `src/router.rs` (wired `GET /` and `ServeDir` on `/assets`)
- `openspec/changes/shirley-show-core/tasks.md` (checked off Slice C tasks 12-19)

---

## 6. Deviations from Design

None. The implementation follows the micro-modular specifications in `design.md`, `specs/presentation/spec.md`, and `specs/show-item/spec.md`.

---

## 7. Workload & PR Boundary

- **Work Unit:** `slice-c-frontend-player` (PR 3 of 3 in stacked chain to `main`).
- **Attempt Authority Token:** `sha256:91d3fe6d36b0206b9ef4f151a2ec33923c36066852fd0ec248403249f9628018`
- **Request ID:** `slice-c-acquire-001`
- **Max changed lines limit:** 6000 lines.
- **Review Strategy:** Approved `size:exception` for PR 3 stacked onto PR 2.
- **Implementation Status:** 57/57 tasks completed across all slices (100% complete). Ready for `sdd-verify`.

---

## 8. Remediation Work Unit: fix-verify-3-blockers

**Attempt Authority:** `fix-begin-001`  
**Work Unit:** `fix-verify-3-blockers`  
**Status:** All 3 critical blockers resolved with Strict TDD.

### Summary of Fixes:
1. **CRITICAL 1 (Presentation Cascade Audio Cleanup):**
   - Pre-queries `audio_key`s from `show_items` prior to cascade deletion.
   - Commits transaction in SQLite.
   - Post-commit: attempts physical file unlinking via `LocalAudioStorage::delete_file`.
   - Records completed deletions via `mark_completed` or records error via `record_failure` in `SqliteCleanupRepository`.
   - Reports truthful `cleanup_pending` in `DeleteResult`.
   - Verified via unit test `test_delete_presentation_physical_audio_cleanup` and integration test `test_delete_presentation_with_cascade_audio_file_cleanup`.
2. **CRITICAL 2 (Interrupted Staging & Duplicate Audio):**
   - Introduced RAII guard `StagingFileGuard` in `src/infra/filesystem/staging_guard.rs`.
   - Rejects duplicate `audio` fields with 400 Bad Request (`duplicate_audio_field`).
   - If stream interrupts, writes fail, format is invalid, or payload limit is exceeded, `StagingFileGuard::drop` purges the partial staging file immediately.
   - Disarms guard only upon successful promote/append.
   - Verified via unit/integration tests `test_multipart_track_upload_interrupted_cleans_staging` and `test_multipart_track_upload_duplicate_audio_rejected_and_cleaned`.
3. **CRITICAL 3 (Strict TDD Slices A/B Cycle Evidence):**
   - Reconstructed complete cycle tables for Slices A, B, and C in Section 3.
   - Honestly documented RED verification intent, test suites, GREEN implementations, and triangulation evidence.
   - Explicitly documented non-automated frontend testing limitation and pre-commit RED reproducibility boundaries.

### Automated Test Suite:
- 13 test files, 81 tests passing, 0 failures, 0 warnings (`cargo test`).
