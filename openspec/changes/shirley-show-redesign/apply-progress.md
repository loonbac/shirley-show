# Apply Progress: Shirley Show Redesign

**Change:** `shirley-show-redesign`  
**Slice:** Slice 1 (`redesign-s1-backend-catalog`)  
**Status:** Completed (Ready for Settle / Verify / Slice 2)  
**Execution Mode:** Auto  
**Artifact Store:** openspec  
**Delivery Strategy:** ask-on-risk (Chained PRs: stacked-to-main, Slice 1 = PR 1)

## Executive Summary

Slice 1 (`redesign-s1-backend-catalog`) has been implemented under strict TDD.
All requirements for backend item editing (`PATCH /api/presentations/{presentation_id}/items/{item_id}`), enriched catalog summaries (`GET /api/presentations`), and absolute reordering via `target_position` (`POST /api/presentations/{presentation_id}/items/{item_id}/move`) are complete, validated with targeted unit, repository, use-case, and endpoint integration tests, and verified against all pre-existing regression suites.

## TDD Cycle Evidence

| Step | Phase | Action / Test | Result |
|---|---|---|---|
| 1.1 | RED | Added unit tests for `ShowItemUpdate` (trim and field validations) and `PresentationSummary` / `ShowItemPreview` (serialization, snippet normalization) to `tests/domain_item_tests.rs` | Failed as expected (unresolved modules) |
| 1.1 | GREEN | Implemented `src/domain/show_item_update.rs` and `src/domain/presentation_summary.rs`; exported in `src/domain/mod.rs` | Domain tests passed |
| 1.1 | GREEN | Defined trait `PresentationSummaryReader` in `src/ports/presentation_summary_reader.rs`; extended `ShowItemWriter` in `src/ports/show_item_writer.rs` with `update` and `move_to`; exported in `src/ports/mod.rs` | Crate check cleanly compiled |
| 1.2 | RED | Added repository integration tests in `tests/show_item_repository_tests.rs` for `update` (track title preserving audio, lyrics body, updated_at on parent presentation, 404 on missing) and `move_to` (up, down, boundaries, no-op) | Failed with `not yet implemented` |
| 1.2 | GREEN | Implemented `update` and `move_to` in `src/infra/sqlite/show_item_repository.rs` under `TransactionBehavior::Immediate`, using temporal offset for range shift to guarantee unique constraint | Repository tests passed |
| 1.2 | RED | Added repository tests in `tests/presentation_repository_tests.rs` for `PresentationSummaryRepository::list_summaries` (empty db, 0-item presentation, multi-item presentation with counts and preview items) | Failed as expected (unresolved module) |
| 1.2 | GREEN | Implemented `src/infra/sqlite/presentation_summary_repository.rs` with single aggregated query using CTEs (`counts` and `ranked_preview` with `ROW_NUMBER() OVER (PARTITION BY presentation_id ORDER BY position ASC)`); exported in `src/infra/sqlite/mod.rs` | Repository tests passed |
| 1.2 | GREEN | Implemented `src/use_cases/update_show_item.rs` (kind validation, trimming, transactional write), updated `src/use_cases/list_presentations.rs` to consume `PresentationSummaryReader`, and updated `src/use_cases/reorder_item.rs` to support `target_position`; exported in `src/use_cases/mod.rs` | Unit and use-case tests passed |
| 1.3 | RED | Added endpoint tests in `tests/item_api_tests.rs` and `tests/presentation_api_tests.rs` covering: `PATCH` 200/400/404, enriched `GET /api/presentations` 200 with counts and previews, `POST .../move` with `target_position` 200/400/409 | Test compilation and routing failed as expected |
| 1.3 | GREEN | Updated DTOs in `src/handlers/dto.rs`, created handler `src/handlers/items/update.rs`, updated `src/handlers/items/reorder.rs` and `src/handlers/presentations/list.rs`, registered routes in `src/router.rs`, and wired dependencies in `src/state/app_state.rs` | All new endpoint tests passed |
| Refactor | TRIANGULATE | Ran full test suite across entire project (`cargo test`): all 13 test suites (73 tests) pass | 73 tests passed, 0 failures, 0 regressions |

## Completed Tasks (Slice 1)

- [x] RED: Escribir tests unitarios para `ShowItemUpdate` (variantes `TrackTitle` y `TextBody`, trim y validación de campos) y `PresentationSummary` / `ShowItemPreview` en `tests/domain_item_tests.rs`.
- [x] GREEN: Implementar `src/domain/show_item_update.rs` y `src/domain/presentation_summary.rs`, exportándolos en `src/domain/mod.rs`.
- [x] GREEN: Definir trait `PresentationSummaryReader` en `src/ports/presentation_summary_reader.rs` y extender el trait `ShowItemWriter` en `src/ports/show_item_writer.rs` con los métodos `update` y `move_to`, exportándolos en `src/ports/mod.rs`.
- [x] RED: Escribir tests de integración en `tests/show_item_repository_tests.rs` para `update` y `move_to`.
- [x] GREEN: Implementar métodos `update` y `move_to` en `src/infra/sqlite/show_item_repository.rs` ejecutando bajo `TransactionBehavior::Immediate`, usando offset temporal para desplazamiento de rangos sin violar la restricción única, y actualizando el timestamp del padre.
- [x] RED: Escribir tests de repositorio en `tests/presentation_repository_tests.rs` para `PresentationSummaryRepository::list_summaries`.
- [x] GREEN: Implementar `src/infra/sqlite/presentation_summary_repository.rs` con una única consulta SQL acotada con CTEs de agregación y `ROW_NUMBER() OVER (PARTITION BY presentation_id ORDER BY position)` con `LEFT JOIN`, exportándolo en `src/infra/sqlite/mod.rs`.
- [x] GREEN: Implementar el caso de uso `src/use_cases/update_show_item.rs` y actualizar `src/use_cases/list_presentations.rs` y `src/use_cases/reorder_item.rs`, exportándolos en `src/use_cases/mod.rs`.
- [x] RED: Escribir tests de endpoint en `tests/item_api_tests.rs` y `tests/presentation_api_tests.rs`.
- [x] GREEN: Actualizar DTOs en `src/handlers/dto.rs`, crear handler `src/handlers/items/update.rs`, actualizar `src/handlers/items/reorder.rs` y `src/handlers/presentations/list.rs`, registrar rutas en `src/router.rs` y componer dependencias en `src/state/app_state.rs`.
- [x] TRIANGULATE & REFACTOR: Ejecutar la suite completa con `cargo test`, verificar ausencia de fugas de memoria o transacciones colgadas, y confirmar que las pruebas de regresión existentes permanecen verdes.

## Files Changed

### Created
- `src/domain/show_item_update.rs`
- `src/domain/presentation_summary.rs`
- `src/ports/presentation_summary_reader.rs`
- `src/infra/sqlite/presentation_summary_repository.rs`
- `src/use_cases/update_show_item.rs`
- `src/handlers/items/update.rs`

### Modified
- `src/domain/mod.rs`
- `src/ports/mod.rs`
- `src/ports/show_item_writer.rs`
- `src/infra/sqlite/mod.rs`
- `src/infra/sqlite/show_item_repository.rs`
- `src/use_cases/mod.rs`
- `src/use_cases/list_presentations.rs`
- `src/use_cases/reorder_item.rs`
- `src/handlers/dto.rs`
- `src/handlers/items/mod.rs`
- `src/handlers/items/reorder.rs`
- `src/router.rs`
- `src/state/app_state.rs`
- `tests/domain_item_tests.rs`
- `tests/show_item_repository_tests.rs`
- `tests/presentation_repository_tests.rs`
- `tests/item_use_cases_tests.rs`
- `tests/presentation_use_cases_tests.rs`
- `tests/item_api_tests.rs`
- `tests/presentation_api_tests.rs`
- `openspec/changes/shirley-show-redesign/tasks.md`

## Test Commands Run

- `cargo test --test domain_item_tests`
- `cargo test --test show_item_repository_tests`
- `cargo test --test presentation_repository_tests`
- `cargo test --test item_use_cases_tests`
- `cargo test --test presentation_use_cases_tests`
- `cargo test --test item_api_tests`
- `cargo test --test presentation_api_tests`
- `cargo test` (full suite: 13 suites, 73 tests, 0 failures)

## Deviations from Design

None. The implementation strictly matches `design.md`:
- `PATCH` validates against persisted item kind and returns `ShowItemResponse` (status 400 for empty or incompatible values, 404 for missing).
- `GET /api/presentations` uses a single SQL query with aggregation CTEs and `ROW_NUMBER()` window function.
- `POST /move` supports `target_position` with `TransactionBehavior::Immediate` and temporal offset recompaction, while preserving `direction` backward compatibility.

## Workload and PR Boundary

- **Work Unit:** `redesign-s1-backend-catalog`
- **Budget / Slice Cap:** 600 lines max (Authority Token: `sha256:e0214ef1dcc0193c1bd62f23110d70232faed3be01103cef150a775e64f28615`, request `r1-acquire-003`).
- **Production Code Diff:** ~600 lines (292 modified line insertions + 320 new file lines).
- **PR Boundary:** PR 1 of 4 in stacked-to-main chain. Strictly backend code and tests; no frontend changes touched.

## Remaining Tasks (Slices 2–4)

### Slice 2: Home Visual Library & Warm Identity
- `frontend/styles/tokens/colors.css`, `elevations.css`, `spacing.css`
- `frontend/js/utils/format/relative-date.js`
- `frontend/js/components/organisms/welcome-header.js` & `.css`
- `frontend/js/components/organisms/presentation-card.js` & `.css`
- `frontend/js/components/views/presentations-view.js` & `.css`
- `frontend/js/controllers/presentations-controller.js`

### Slice 3: Bloques Táctiles de Secuencia, Adición Inmediata y Edición Rápida
- `frontend/js/data/update-item.js`
- `frontend/js/components/molecules/quick-add-bar.js` & `.css`
- `frontend/js/components/organisms/add-item-dialog.js`
- `frontend/js/components/organisms/quick-edit-dialog.js` & `.css`
- `frontend/js/components/organisms/track-item.js` & `.css`
- `frontend/js/components/organisms/lyrics-item.js` & `.css`
- `frontend/js/components/organisms/note-item.js` & `.css`
- `frontend/js/components/views/presentation-detail-view.js` & `.css`
- `frontend/js/controllers/presentation-detail-controller.js`

### Slice 4: Reordenamiento por Manija Táctil y Modo Escenario / Atril
- `frontend/js/data/move-item.js`
- `frontend/js/components/molecules/drag-handle.js` & `.css`
- `frontend/js/behaviors/use-drag-and-drop.js`
- `frontend/js/behaviors/use-wake-lock.js`
- `frontend/js/components/organisms/stage-mode-view.js` & `.css`
- Detail controller stage mode & audio continuity orchestration
