# Apply Progress: Shirley Show Core

**Change:** `shirley-show-core`  
**Latest Slice:** Slice B (Show Items, Audio Upload, Range Streaming & Reordering)  
**Attempt Authority Token (Slice B):** `sha256:309e88f7327df5bb443e969e353be90414f13e277d91a8bfb40db5945b9d7bbf`  
**Work Unit:** `slice-b-items-audio-reorder`  
**Request ID:** `slice-b-acquire-001`  
**Status:** Completed (Slice A + Slice B)

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

---

## 3. Strict TDD Cycle Evidence (Slice B)

| Phase / Task Group | RED Command & Evidence | GREEN Implementation & Evidence | TRIANGULATE / REFACTOR Evidence | Status |
|---|---|---|---|---|
| **6. Show Item Domain** | `cargo test --test domain_item_tests` failed (missing `domain::show_item`, `show_item_kind`, `move_direction`, `audio_format`). | Implemented domain types and validation rules in `src/domain/`. Tests passed (7/7). | Added tests for Unicode, multiline lyrics trimming, corrupted DB row handling, and JSON serialization. 8/8 passed. | Passed |
| **7. Show Item Repository** | `cargo test --test show_item_repository_tests` failed (missing `ports::show_item_reader/writer` and `SqliteShowItemRepository`). | Defined ports and implemented repository using `BEGIN IMMEDIATE` with temporary index swap and two-phase recompaction. Tests passed (5/5). | Added boundary tests on single item, edge deletions (first/last), and 5 rapid successive swaps asserting strictly contiguous `0..N-1` indices. 8/8 passed. | Passed |
| **8. Local Audio Storage & Cleanup** | `cargo test --test audio_storage_tests` failed (missing `infra::filesystem` and `SqliteCleanupRepository`). | Implemented `GeneratedAudioKey`, `detect_audio_format`, `LocalAudioStorage`, `SqliteCleanupRepository`, and `reconcile_audio_files`. Tests passed (4/4). | Added tests for corrupted WAV/M4A headers and cleanup queue failure retention with attempt counter and error recording. 5/5 passed. | Passed |
| **9. Item & Audio Use Cases** | `cargo test --test item_use_cases_tests` failed (missing use case modules). | Implemented `AddLyrics`, `AddNote`, `UploadTrack`, `ReorderItem`, `DeleteItem`, and `OpenTrackAudio`. Tests passed (5/5). | Added tests for compensation on empty title and database append failure, non-track rejection in `OpenTrackAudio`, and file cleanup on deletion. 5/5 passed. | Passed |
| **10. Range Parser & Audio Streaming** | `cargo test --test range_parser_tests` and `audio_stream_api_tests` failed (missing range parser and stream handler). | Implemented RFC 9110 range parser in `src/handlers/audio/parse_range.rs` and streaming handler in `src/handlers/audio/stream.rs`. Tests passed. | Verified RFC 9110 compliance: `Accept-Ranges: bytes`, `Content-Range: bytes 0-1023/total`, fallback to 200 on malformed/multi-range, HEAD request empty body, and 404 on missing audio file. 13/13 passed. | Passed |
| **11. Item & Upload HTTP Handlers** | `cargo test --test item_api_tests` failed with 404 (unrouted endpoints). | Implemented Axum handlers in `src/handlers/items/`, DTOs in `src/handlers/dto.rs`, and wired routes in `src/router.rs`. Tests passed (5/5). | Verified route-scoped 101 MiB body limit with exact 100 MiB counter returning 413, unsupported media type returning 415, empty title returning 422, and reorder boundary returning 409. 6/6 passed. | Passed |

---

## 4. Test Execution Summary

Command: `cargo test`

```text
running 5 tests (tests/audio_storage_tests.rs)
test test_detect_audio_format_magic_bytes ... ok
test test_generated_audio_key_validation_and_traversal_resistance ... ok
test test_local_audio_storage_stage_promote_delete ... ok
test test_cleanup_queue_and_reconciliation ... ok
test test_cleanup_failure_retains_record_with_error_and_attempts ... ok
test result: ok. 5 passed; 0 failed

running 6 tests (tests/audio_stream_api_tests.rs)
test test_audio_stream_missing_track_and_missing_file_404 ... ok
test test_audio_stream_range_request_206 ... ok
test test_audio_stream_out_of_bounds_range_416 ... ok
test test_audio_stream_head_request_without_body ... ok
test test_audio_stream_full_request_200 ... ok
test test_audio_stream_malformed_and_multi_range_fallback_to_200 ... ok
test result: ok. 6 passed; 0 failed

running 8 tests (tests/domain_item_tests.rs)
test test_audio_format_properties ... ok
test test_lyrics_payload_validation ... ok
test test_note_payload_validation ... ok
test test_move_direction_parsing ... ok
test test_show_item_from_db_mappings_and_corruption ... ok
test test_show_item_kind_as_str ... ok
test test_track_payload_validation ... ok
test test_show_item_constructors_and_invariants ... ok
test result: ok. 8 passed; 0 failed

running 7 tests (tests/domain_presentation_tests.rs)
test result: ok. 7 passed; 0 failed

running 6 tests (tests/item_api_tests.rs)
test test_add_text_items_validation_and_not_found ... ok
test test_reorder_item_endpoint_and_boundary_conflict ... ok
test test_add_text_items_and_get_presentation_detail ... ok
test test_multipart_track_upload_success_and_validations ... ok
test test_delete_item_endpoint ... ok
test test_multipart_track_upload_oversized_returns_413 ... ok
test result: ok. 6 passed; 0 failed

running 5 tests (tests/item_use_cases_tests.rs)
test test_upload_track_success_and_compensation_on_failure ... ok
test test_open_track_audio_use_case ... ok
test test_add_lyrics_and_note_use_cases ... ok
test test_reorder_item_use_case ... ok
test test_delete_item_use_case_cleans_file ... ok
test result: ok. 5 passed; 0 failed

running 5 tests (tests/presentation_api_tests.rs)
test result: ok. 5 passed; 0 failed

running 7 tests (tests/presentation_repository_tests.rs)
test result: ok. 7 passed; 0 failed

running 5 tests (tests/presentation_use_cases_tests.rs)
test result: ok. 5 passed; 0 failed

running 7 tests (tests/range_parser_tests.rs)
test test_parse_range_clamp_oversized_end ... ok
test test_parse_range_malformed_or_multi_falls_back_to_full ... ok
test test_parse_range_open_ended ... ok
test test_parse_range_none_or_empty ... ok
test test_parse_range_suffix ... ok
test test_parse_range_unsatisfiable ... ok
test test_parse_range_valid_exact ... ok
test result: ok. 7 passed; 0 failed

running 8 tests (tests/show_item_repository_tests.rs)
test test_append_non_existent_presentation_fails ... ok
test test_get_item_by_presentation_and_by_id ... ok
test test_delete_item_recompacts_and_enqueues_audio ... ok
test test_single_item_presentation_swap_boundary_checks ... ok
test test_append_items_and_list_ordered ... ok
test test_swap_atomic_and_boundary_guards ... ok
test test_delete_first_and_last_items_recompaction ... ok
test test_rapid_successive_swaps_maintain_contiguous_indices ... ok
test result: ok. 8 passed; 0 failed

running 3 tests (tests/sqlite_migration_tests.rs)
test result: ok. 3 passed; 0 failed

Total: 72 passed; 0 failed; 0 warnings.
```

---

## 5. Files Changed / Created (Slice B)

### New Files
- `src/domain/audio_format.rs`
- `src/domain/move_direction.rs`
- `src/domain/show_item.rs`
- `src/domain/show_item_kind.rs`
- `src/ports/cleanup_queue.rs`
- `src/ports/show_item_reader.rs`
- `src/ports/show_item_writer.rs`
- `src/infra/sqlite/cleanup_repository.rs`
- `src/infra/sqlite/show_item_repository.rs`
- `src/infra/filesystem/mod.rs`
- `src/infra/filesystem/detect_audio_format.rs`
- `src/infra/filesystem/generated_audio_key.rs`
- `src/infra/filesystem/local_audio_storage.rs`
- `src/infra/filesystem/reconcile_audio_files.rs`
- `src/use_cases/add_lyrics.rs`
- `src/use_cases/add_note.rs`
- `src/use_cases/upload_track.rs`
- `src/use_cases/reorder_item.rs`
- `src/use_cases/delete_item.rs`
- `src/use_cases/open_track_audio.rs`
- `src/handlers/audio/mod.rs`
- `src/handlers/audio/parse_range.rs`
- `src/handlers/audio/stream.rs`
- `src/handlers/items/mod.rs`
- `src/handlers/items/add_text.rs`
- `src/handlers/items/upload_track.rs`
- `src/handlers/items/reorder.rs`
- `src/handlers/items/delete.rs`
- `tests/audio_storage_tests.rs`
- `tests/audio_stream_api_tests.rs`
- `tests/domain_item_tests.rs`
- `tests/item_api_tests.rs`
- `tests/item_use_cases_tests.rs`
- `tests/range_parser_tests.rs`
- `tests/show_item_repository_tests.rs`

### Modified Files (Minimal Integration)
- `Cargo.toml` (enabled `axum` multipart feature, added `tokio-util` with `io` feature)
- `Cargo.lock`
- `src/error.rs` (added `BadRequest` 400, `PayloadTooLarge` 413, `UnsupportedMediaType` 415)
- `src/domain/mod.rs` (exported show item domain types)
- `src/ports/mod.rs` (exported show item and cleanup traits)
- `src/infra/mod.rs` (exported filesystem module)
- `src/infra/sqlite/mod.rs` (exported new repositories)
- `src/use_cases/mod.rs` (exported new use cases)
- `src/handlers/mod.rs` (exported audio and items handlers)
- `src/handlers/dto.rs` (added item DTOs, updated presentation detail response)
- `src/handlers/presentations/get.rs` (mapped show items into presentation detail response)
- `src/state/app_state.rs` (wired item repositories, storage, and use cases)
- `src/router.rs` (registered item and streaming routes)
- `openspec/changes/shirley-show-core/tasks.md` (checked off Slice B tasks 6-11)

---

## 6. Deviations from Design

None. All implementations strictly adhere to `design.md` and `specs/show-item/spec.md`.

---

## 7. Workload & PR Boundary

- **Work Unit:** `slice-b-items-audio-reorder` (PR 2 of 3 in stacked chain to `main`).
- **Attempt Authority Token:** `sha256:309e88f7327df5bb443e969e353be90414f13e277d91a8bfb40db5945b9d7bbf`
- **Request ID:** `slice-b-acquire-001`
- **Max changed lines limit:** 3000 lines.
- **Production Code (`src/`):** ~1,712 lines in new modules + 348 lines changed in existing files.
- **Review Strategy:** Approved `size:exception` for PR 2 stacked onto PR 1.
- **Remaining Slice:** Slice C (Frontend UI, CSS tokens, and exclusive player controller).
