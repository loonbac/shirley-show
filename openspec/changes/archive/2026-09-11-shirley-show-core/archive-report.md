# Archive Report: shirley-show-core

**Change:** `shirley-show-core`
**Archive status:** PASS — archived
**Date:** 2026-09-11
**Artifact store:** `openspec` (file-backed)
**Verified revision:** `e3ba0754f3e844f8588fab5eea28685381d1906d`
**Closure commit (pre-archive HEAD):** `a1d45560cbd77a0bce940b113cb51eec32ef3378`
**Archived path:** `openspec/changes/archive/2026-09-11-shirley-show-core/`

## Verdict

The change is archived with a clean verification trail. Verification reported PASS with 16/16 requirements, 42/42 scenarios, 81/81 `cargo test`, successful `cargo build`, 57/57 implementation tasks checked, zero blockers, and zero critical findings. No destructive canonical merge was required because both domain specs are new.

## Artifacts Read

| Artifact | Path | Result |
|---|---|---|
| Proposal | `openspec/changes/shirley-show-core/proposal.md` | present |
| Delta specs | `specs/presentation/spec.md`, `specs/show-item/spec.md` | present (full domain specs) |
| Design | `openspec/changes/shirley-show-core/design.md` | present |
| Tasks | `openspec/changes/shirley-show-core/tasks.md` | 57 checked, 0 unchecked |
| Apply progress | `openspec/changes/shirley-show-core/apply-progress.md` | present (Slices A/B/C + remediation) |
| Verify report | `openspec/changes/shirley-show-core/verify-report.md` | present, `verdict: pass` |
| Sync report | — | absent; archive-time sync fallback used (see below) |
| Config | `openspec/config.yaml` | present (`strict_tdd: true`, archive rule: warn before destructive deltas) |
| Explore | `openspec/changes/shirley-show-core/explore.md` | present (moved with the change) |

## Final Task Completion Gate

- Persisted tasks artifact re-read immediately before sync fallback and move.
- Implementation task markers: 57 `- [x]`, 0 `- [ ]`.
- Unchecked implementation task lines: **none**.
- Deferred parent lifecycle actions: **0**.
- Stale-checkbox reconciliation: not required and not performed; no checkbox was modified by archive.

## Archive-Time Sync Fallback

- `sync-report.md` was absent, so `sdd-sync` had not produced a persisted sync report.
- The parent prompt explicitly directed archiving "según la convención del proyecto (specs canónicos + cierre)", which authorized the archive-time file-backed sync fallback. The native status snapshot (`sync: blocked`, `archive: blocked`) reflected the pre-sync state and carried an empty `blockedReasons` array and no critical findings.
- `sync-report.md` was intentionally not fabricated; the fallback is recorded only here under `sdd-archive` ownership.

### Domains Synced

| Domain | Canonical path | Mode | SHA-256 |
|---|---|---|---|
| `presentation` | `openspec/specs/presentation/spec.md` | new canonical (full copy) | `f50d2b9efce5cbeb72e0e1ed7283bb61ac4a56436c9b358d91d682a2c4835fac` |
| `show-item` | `openspec/specs/show-item/spec.md` | new canonical (full copy) | `c34baf7ce74aee9472ea185f7ee3b761f62f332ed5ecfd628e88aff239f016ae` |

Byte-identical copies verified with `diff` against the delta specs.

### Requirement Names Synced

**`presentation` — 5 requirements, all new/ADDED:**
1. List Presentations
2. Create Presentation
3. Open Presentation and Handle Route Staleness
4. Rename Presentation
5. Delete Presentation with Confirmation and Cascade Cleanup

**`show-item` — 11 requirements, all new/ADDED:**
1. Single Vertical Sequence and Empty State
2. Add Item with Type Selector
3. Track Item Creation and Audio Upload
4. Audio Format and Size Limits
5. Audio Streaming and Mobile Range Requests
6. Exclusive Single Audio Playback and Stage Controls
7. Audio Interruption on Navigation or Missing Media
8. Lyrics Item Creation and Display
9. Note Item Creation and Display
10. Atomic Item Reordering with Boundary Guards
11. Item Deletion and Resource Cleanup

- MODIFIED requirements: **none**.
- REMOVED requirements: **none**.
- Totals: 16 requirements, 42 scenarios, matching the verify report (16/16, 42/42).

### Active Same-Domain Change Warnings

- None. `openspec/changes/` contained only `shirley-show-core` at archive time; `relations.sameDomainActiveChanges` was empty. No competing delta touches `presentation` or `show-item`.

## Destructive Merge Guard

- Not triggered. Both canonical specs were created empty-directory-first (no pre-existing `openspec/specs/**/spec.md`), so no requirement was replaced or deleted and no canonical scenario was dropped.
- No destructive approval was required; `rules.archive` ("warn before merging destructive deltas") was satisfied trivially.

## Structured Status and Action Context

- `artifactStore: openspec`; `planningHome.root` and `workspaceRoot` = `/home/loonbac/Proyectos/shirley-show`.
- `actionContext.mode: repo-local` with `allowedEditRoots: ["/home/loonbac/Proyectos/shirley-show"]`. All canonical writes and the move stayed inside that root.
- Active change selection was unambiguous (`shirley-show-core` only).
- Status findings: the native snapshot reported `verify: ready`, `sync: blocked`, `archive: blocked`, `nextRecommended: sdd-verify`, with empty `blockedReasons` and no critical findings. The parent-supplied final-state facts (verify PASS committed at `a1d4556`) and the on-disk `verify-report.md` superseded that intermediate snapshot; the only real gate left was the missing sync report, resolved by the authorized sync fallback above.
- No CRITICAL verification issue exists; nothing was overridden.
- Repository state before the move: clean working tree at `a1d4556`, no `openspec/changes/archive/` directory yet.

## Move to Archive

```text
openspec/changes/shirley-show-core/
  -> openspec/changes/archive/2026-09-11-shirley-show-core/
```

- `openspec/changes/archive/` created during the move.
- No active artifact was deleted or modified silently; the whole change folder moved intact, including `explore.md`, `apply-progress.md`, and `verify-report.md`.

## Non-Critical Items Carried Forward

- Clippy informational findings (`too_many_arguments` in `src/domain/show_item.rs`, `collapsible_if` in `src/infra/filesystem/reconcile_audio_files.rs`) remain; both are pre-existing, non-blocking, and outside remediation paths.
- Historical implementation commits exceed the 400-line review budget; recorded as review-process debt, not a correctness blocker.
- Design debt: presentation cleanup storage inferred from the database parent when `SqlitePresentationRepository` is not explicitly constructed with storage; explicit dependency injection would be clearer in a future cleanup.
- No frontend automated runner exists; frontend acceptance remains structural/manual as declared in design and apply progress.
- Review/deferred items were accepted for archive without an explicit `size:exception` renewal; the original PR3 `size:exception` is recorded in `apply-progress.md`.

## Memory Observation IDs

- Not applicable: the active artifact store is `openspec` (file-backed), so no Engram artifact observations were used or created for this change's proposal/spec/design/tasks/verify/archive artifacts.

## Delivery

- No remote is configured; no push or pull request was created. Closure is local to `main` in `/home/loonbac/Proyectos/shirley-show`.
