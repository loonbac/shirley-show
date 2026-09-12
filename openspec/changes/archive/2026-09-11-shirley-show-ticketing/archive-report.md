# Archive Report: `shirley-show-ticketing`

**Change:** `shirley-show-ticketing`
**Archived at:** `openspec/changes/archive/2026-09-11-shirley-show-ticketing`
**Date:** 2026-09-11
**Mode:** openspec (file-backed)
**Archive status:** PASS

## Archive preconditions

| Artifact | Read | Status |
|---|---|---|
| `proposal.md` | ✅ | done |
| `specs/checkin/spec.md`, `specs/entrada-publica/spec.md`, `specs/entradas/spec.md`, `specs/eventos/spec.md`, `specs/gastos/spec.md`, `specs/interesados/spec.md`, `specs/ressy-ticketing/spec.md`, `specs/resumen/spec.md` | ✅ | done (31 requirements / 67 scenarios total) |
| `design.md` | ✅ | done |
| `tasks.md` | ✅ | done (50/50 checked) |
| `apply-progress.md` | ✅ | done (all_done, 0 remaining; 59 strict-TDD cycles) |
| `verify-report.md` | ✅ | PASS WITH WARNINGS |
| `sync-report.md` | ✅ | SUCCESS (created this run via approved archive-time sync fallback) |
| `openspec/config.yaml` | ✅ | schema: spec-driven; strict_tdd: true; archive rule: "Warn before merging destructive deltas" (N/A) |

## Verification gate

- `verify-report.md` verdict: **pass_with_warnings** (`schema: gentle-ai.verify-result/v1`, evidence_revision `sha256:141446fb…`, HEAD `fda1335`, work-unit `verify-ticketing`).
- Blockers: 0; critical findings: 0.
- Requirements: 31/31; scenarios: 67/67 (per-domain: checkin 3/3·8/8, entrada-publica 4/4·7/7, entradas 6/6·14/14, eventos 5/5·14/14, gastos 3/3·7/7, interesados 3/3·7/7, ressy-ticketing 4/4·6/6, resumen 3/3·4/4).
- `cargo test` (full, `SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-fda1335-full`): 330 passed, 0 failed, 1 ignored (exit 0); `cargo build` exit 0; `cargo fmt --check` clean; ticketing-scoped clippy (`--lib --bin shirley-show --test ticketing_*`) clean with `-D warnings`; concurrency race test 100/100 repetitions green; `git diff --check fda1335^ fda1335` clean.
- Nonexistent FAIL/BLOCKED/CRITICAL verification markers: none found.
- Non-critical warnings carried (no archive impact): global `cargo clippy --all-targets --all-features` fails on 28 diagnoses in non-ticketing tests (`youtube_url`, `presentation_repository`, `agent_chat`, `agent_tools`); 4 WARNING-level CSS-class frontend assertions with accessible companion assertions; documented SQLite/filesystem non-distributed transaction (mitigated by FS-first order, atomic renames, compensation and `reconcile_orphans`).

## Task completion gate (Final Gate)

- Persisted `tasks.md` re-read immediately before sync fallback and move: **50/50** implementation tasks checked; **0 unchecked** `- [ ]` markers (grep-verified, both in status JSON `taskProgress.unchecked: []` and direct grep).
- No stale-checkbox reconciliation performed (not requested; not needed).
- `apply-progress.md` corroborates all 50 tasks + 7 first-remediation + 2 final-remediation work units complete.
- Dependencies: `apply: all_done`, `verify: ready` (verify report clean). No deferred parent actions (0/0).

## Sync (archive-time fallback, parent-approved)

- Prior `sdd-sync` had not been run (`sync-report.md` missing). Parent prompt explicitly instructed archive per convention — "Archiva según convención (8 dominios nuevos como specs canónicos: eventos, entradas, checkin, interesados, gastos, resumen, entrada-publica, ressy-ticketing + cierre)" — for a file-backed change with clean verification (PASS 31/31, 67/67, 0 blockers); per the SDD archive contract this is treated as explicit approval of the archive-time sync fallback. Recorded in `sync-report.md` inside the archived change.
- 8 NEW canonical specs created (full-copy, non-destructive, byte-identical to deltas):
  - `openspec/specs/eventos/spec.md` (5 requirements)
  - `openspec/specs/entradas/spec.md` (6 requirements)
  - `openspec/specs/checkin/spec.md` (3 requirements)
  - `openspec/specs/interesados/spec.md` (3 requirements)
  - `openspec/specs/gastos/spec.md` (3 requirements)
  - `openspec/specs/resumen/spec.md` (3 requirements)
  - `openspec/specs/entrada-publica/spec.md` (4 requirements)
  - `openspec/specs/ressy-ticketing/spec.md` (4 requirements)
- ADDED requirements: 31 (names listed in `sync-report.md`).
- MODIFIED: 0; REMOVED: 0. Destructive merge guard: N/A (no canonical domain deltas; existing 8 canonical specs untouched).
- Same-domain active change warnings: none (only active change is this one; collisions: none).

## Delivery state

- Commits on local `main` only, no remote configured (verified `git remote -v` empty): `bd54faa` (S1) → `03578a4` (S2) → `9af8f40` (S3) → `d4cd83f` (S4) → `b367923` (S5) → `03fa345` (S6) → `f82a2ba` (7-blocker fix) → `4899ad8` (5-blocker fix) → `fda1335` (Rem2-B1/B2) → `68a5e16` (docs verify PASS final). No PR created; delivery intentionally remains local per parent instruction.
- Review boundary respected: per-slice deltas stayed within the 400-line budget; final remediation commit `fda1335` = 211 changed code/test lines (within budget; 423 incl. artifacts, within the explicit 500-line authority). No `size:exception` used.
- `git status --porcelain` before archive: only untracked `production-data/` — not read, not modified. `.run/` untouched; no process started on `:3100`.

## Structured status and actionContext

- Change: `shirley-show-ticketing`; artifactStore `openspec`; authoritative store: openspec.
- Status native at archive time: `apply: all_done`, `verify: ready`, `sync: blocked` (missing report → resolved via approved fallback), `archive: blocked` (precondition resolved this run).
- `actionContext.mode`: `repo-local`; `workspaceRoot`/`allowedEditRoots`: `/home/loonbac/Proyectos/shirley-show`; all writes (sync targets, reports, move target) inside the workspace root.
- Dependencies: none; conflicts: none; `nextRecommended` before this run: `sdd-verify` (superseded by clean verify + explicit archive instruction).

## Destructive merge approvals / blockers

- None required (no REMOVED/MODIFIED canonical deltas). No blockers.

## Archived path

`/home/loonbac/Proyectos/shirley-show/openspec/changes/archive/2026-09-11-shirley-show-ticketing/`

## Blockers

None.