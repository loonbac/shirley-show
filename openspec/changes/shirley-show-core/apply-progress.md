# Apply Progress: Shirley Show Core — Slice A

**Change:** `shirley-show-core`  
**Slice:** Slice A (Backend Foundation, Domain, SQLite Storage & Presentations API)  
**Attempt Authority Token:** `sha256:0d12b1cb53d5921da808fd08da8c778c7dde1fbdc4536da9150144135e668c3b`  
**Work Unit:** `slice-a-backend-presentations`  
**Request ID:** `slice-a-acquire-003`  
**Status:** Completed (Slice A)  

---

## 1. Executive Summary

Slice A establishes the core Rust backend foundation for Shirley Show:
- Domain presentation model with Unicode and whitespace trimming invariants.
- Standardized error hierarchy and JSON error responses `{ "error": { "code", "message", "field" } }`.
- SQLite connection pool with embedded migration `0001_shirley_show_core.sql` configuring `PRAGMA foreign_keys=ON`, `PRAGMA busy_timeout=5000`, and `PRAGMA journal_mode=WAL`.
- Presentation repository ports (`PresentationReader`, `PresentationWriter`) and SQLite implementation with cascade cleanup queuing.
- Isolated atomic use cases (1 action per struct/file): `CreatePresentation`, `ListPresentations`, `GetPresentation`, `RenamePresentation`, `DeletePresentation`.
- Axum REST API handlers and router exposing `/api/presentations` (`GET`, `POST`, `GET :id`, `PATCH :id`, `DELETE :id`).
- 100% strict TDD adherence with RED → GREEN → TRIANGULATE → REFACTOR cycles across 5 test suites (27 tests total).

---

## 2. Completed Tasks (Slice A)

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

---

## 3. Strict TDD Cycle Evidence

| Phase / Task Group | RED Command & Evidence | GREEN Implementation & Evidence | TRIANGULATE / REFACTOR Evidence | Status |
|---|---|---|---|---|
| **1. Domain Presentation** | `cargo test --test domain_presentation_tests` failed (missing `domain` and `error` modules). | Implemented `Presentation` in `src/domain/presentation.rs` and `AppError` in `src/error.rs`. Tests passed (5/5). | Added tests for whitespace trimming, Unicode handling, and non-empty title constraints. 7/7 passed. | Passed |
| **2. SQLite Migrations & PRAGMAs** | `cargo test --test sqlite_migration_tests` failed (missing `infra::sqlite`). | Implemented `migrations/0001_shirley_show_core.sql`, `init_pool` with PRAGMAs (WAL, busy_timeout=5000, foreign_keys=ON), and `migrate_pool`. Tests passed (2/2). | Verified migration idempotency and PRAGMAs across multiple pooled connections. 3/3 passed. | Passed |
| **3. Presentation Repository** | `cargo test --test presentation_repository_tests` failed (missing `ports` and `presentation_repository`). | Defined `PresentationReader`/`Writer` traits and implemented `SqlitePresentationRepository`. Tests passed (4/4). | Added tests for duplicate titles distinct UUIDs, rename preservation, and cascade queueing to `pending_audio_deletions`. 7/7 passed. | Passed |
| **4. Presentation Use Cases** | `cargo test --test presentation_use_cases_tests` failed (missing `use_cases`). | Implemented `CreatePresentation`, `ListPresentations`, `GetPresentation`, `RenamePresentation`, `DeletePresentation` using mock ports. Tests passed (5/5). | Verified strict error code mapping (`"title_required"`, `"presentation_not_found"`). 5/5 passed. | Passed |
| **5. Presentations HTTP API** | `cargo test --test presentation_api_tests` failed (missing `router` and `state`). | Implemented DTOs, Axum handlers, router, and `AppState`. Tests passed (4/4). | Added tests for error response format `{ "error": { "code", "message", "field" } }` and status codes 200, 201, 400, 404, 422. 5/5 passed. | Passed |

---

## 4. Test Execution Summary

Command: `cargo test`

```text
running 0 tests (src/lib.rs)
test result: ok. 0 passed; 0 failed

running 7 tests (tests/domain_presentation_tests.rs)
test test_create_presentation_rejects_empty_or_whitespace_title ... ok
test test_create_presentation_unique_ids_for_identical_titles ... ok
test test_create_presentation_trims_whitespace ... ok
test test_internal_whitespace_preserved ... ok
test test_rename_presentation_rejects_empty_or_whitespace ... ok
test test_rename_presentation_success ... ok
test test_unicode_and_special_characters_handling ... ok
test result: ok. 7 passed; 0 failed

running 5 tests (tests/presentation_api_tests.rs)
test test_create_and_get_presentation ... ok
test test_get_presentations_empty ... ok
test test_error_response_format_and_status_codes ... ok
test test_delete_presentation ... ok
test test_patch_presentation_rename ... ok
test result: ok. 5 passed; 0 failed

running 7 tests (tests/presentation_repository_tests.rs)
test test_rename_presentation ... ok
test test_rename_non_existent_returns_not_found ... ok
test test_delete_non_existent_returns_not_found ... ok
test test_create_and_list_presentations ... ok
test test_get_presentation ... ok
test test_duplicate_titles_yield_distinct_records ... ok
test test_delete_presentation_and_cascade_with_audio_queue ... ok
test result: ok. 7 passed; 0 failed

running 5 tests (tests/presentation_use_cases_tests.rs)
test test_create_presentation_use_case ... ok
test test_delete_presentation_use_case ... ok
test test_list_presentations_use_case ... ok
test test_get_presentation_use_case ... ok
test test_rename_presentation_use_case ... ok
test result: ok. 5 passed; 0 failed

running 3 tests (tests/sqlite_migration_tests.rs)
test test_migration_idempotency ... ok
test test_pragmas_persist_across_multiple_connections ... ok
test test_sqlite_pragmas_and_migration_tables ... ok
test result: ok. 3 passed; 0 failed

Total: 27 passed; 0 failed; 0 warnings.
```

---

## 5. Files Changed / Created

- `Cargo.toml` (package definition and dependencies)
- `Cargo.lock` (reproducible dependency tree)
- `.gitignore` (added `target/`)
- `migrations/0001_shirley_show_core.sql`
- `src/lib.rs`
- `src/error.rs`
- `src/domain/mod.rs`
- `src/domain/presentation.rs`
- `src/ports/mod.rs`
- `src/ports/presentation_reader.rs`
- `src/ports/presentation_writer.rs`
- `src/infra/mod.rs`
- `src/infra/sqlite/mod.rs`
- `src/infra/sqlite/connect.rs`
- `src/infra/sqlite/migrate.rs`
- `src/infra/sqlite/presentation_repository.rs`
- `src/use_cases/mod.rs`
- `src/use_cases/create_presentation.rs`
- `src/use_cases/list_presentations.rs`
- `src/use_cases/get_presentation.rs`
- `src/use_cases/rename_presentation.rs`
- `src/use_cases/delete_presentation.rs`
- `src/handlers/mod.rs`
- `src/handlers/dto.rs`
- `src/handlers/presentations/mod.rs`
- `src/handlers/presentations/list.rs`
- `src/handlers/presentations/create.rs`
- `src/handlers/presentations/get.rs`
- `src/handlers/presentations/rename.rs`
- `src/handlers/presentations/delete.rs`
- `src/state/mod.rs`
- `src/state/app_state.rs`
- `src/router.rs`
- `tests/domain_presentation_tests.rs`
- `tests/sqlite_migration_tests.rs`
- `tests/presentation_repository_tests.rs`
- `tests/presentation_use_cases_tests.rs`
- `tests/presentation_api_tests.rs`
- `openspec/changes/shirley-show-core/tasks.md` (marked Slice A tasks completed)

---

## 6. Deviations from Design

None. All interfaces, database table schemas, PRAGMA parameters, REST endpoints, DTOs, and error bodies adhere strictly to `design.md` and `specs/presentation/spec.md`.

---

## 7. Workload & PR Boundary

- **Work Unit:** `slice-a-backend-presentations` (PR 1 of 3 in stacked chain to `main`).
- **Production Code (`src/`):** 818 lines (within the planned ~750 lines forecast).
- **Total lines (including integration tests and migrations):** 1,551 lines.
- **Review Strategy:** Approved `size:exception` for PR 1.
- **Orchestrator Settlement:** The orchestrator owns the final settlement of attempt `slice-a-acquire-003`.
