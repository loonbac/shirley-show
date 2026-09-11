# Archive Report — shirley-show-vocal-sep

## Status

**ARCHIVE PASS** — archived 2026-09-11 to `openspec/changes/archive/2026-09-11-shirley-show-vocal-sep/`.

## Verification Gate

- `verify-report.md`: `verdict: pass` (envelope `gentle-ai.verify-result/v1`), 10/10 requirements, 55/55 scenarios PASS, no unresolved `FAIL`/`BLOCKED`/`CRITICAL` findings. Prior critical blockers (PolarFormer preflight alignment with the real `karaoke-separator` wrapper) remediated; live GPU E2E intentionally not run (test stays `ignored`), per original constraints.
- Final Task Completion Gate (re-read immediately before sync/move): **0 unchecked `- [ ]` implementation task markers** in `tasks.md` (16/16 `[x]`). Confirmed by apply-progress and verify-report. No stale-checkbox reconciliation needed.
- `sync-report.md`: absent by design in this run; canonical spec sync was performed archive-time under explicit parent directive ("Archiva según convención (specs canónicos + cierre)") — recorded here as the archive-time sync fallback.

## Artifacts Read

- `openspec/changes/shirley-show-vocal-sep/proposal.md`
- `openspec/changes/shirley-show-vocal-sep/specs/agent-chat/spec.md` (delta)
- `openspec/changes/shirley-show-vocal-sep/specs/agent-tools/spec.md` (delta)
- `openspec/changes/shirley-show-vocal-sep/design.md`
- `openspec/changes/shirley-show-vocal-sep/tasks.md`
- `openspec/changes/shirley-show-vocal-sep/apply-progress.md`
- `openspec/changes/shirley-show-vocal-sep/verify-report.md`
- `openspec/specs/agent-chat/spec.md` (canonical, pre-sync)
- `openspec/specs/agent-tools/spec.md` (canonical, pre-sync)
- `openspec/config.yaml` (checked; no `rules.archive` override present)

## Domains Synced (delta → canonical)

`openspec/changes/shirley-show-vocal-sep/specs/{domain}/spec.md` → `openspec/specs/{domain}/spec.md` for `agent-chat` and `agent-tools`.

### ADDED requirements

- `agent-chat`: `Conversational Vocal Separation Feedback and Client State Continuity`
- `agent-tools`: `Vocal Separation Tool and GPU Execution Guardrails`

### MODIFIED requirements (full-block replacement by exact heading)

- `agent-chat`: `Agent Chat API Endpoint and Content Negotiation`, `Response Contract and Action Reporting`
- `agent-tools`: `Fixed Tool Allowlist and Schema Definition`, `Staged Track Creation Tool (add_track_from_staged)`, `Presentation Identifier Integrity and Grounded Context`, `Bounded Model Loop and Request Deadline`, `Duplicate Tool Call Prevention within Turn`

### REMOVED requirements

- None.

### ADDED-heading collision (documented reconciliation)

- `agent-chat` / `Ephemeral Staged Audio Registry with TTL Lifecycle`: this ADDED heading already existed in canonical (introduced by the previously archived `shirley-show-yt-download` change). Appending blindly would duplicate the heading, so it was resolved as a **replacement** with the vocal-sep delta's superset content ("downloaded and separated" sources, `separate_vocals` + `download_youtube_track` entries). To guarantee zero scenario coverage loss, the two canonical download-specific scenarios not restated by the delta were preserved into the merged block: `Register downloaded audio in registry with 15-minute TTL` and `Consume staged audio once successfully and block second consumption`. Merged block now carries 7 scenarios.

### Destructive Merge Guard

No REMOVED requirements and no destructive deletion of canonical content. Replaced blocks are full, complete, non-partial replacements authored by the verified delta (the delta is the authoritative post-change contract; "(Previously: …)" annotations preserved per existing canonical convention). Replaced-line summary: `agent-chat` −98/+160 net, `agent-tools` 219 insertions; no requirement was dropped. This archive-time sync proceeded under the parent prompt's explicit directive to archive with canonical specs; no destructive approval was needed beyond it.

### Active same-domain change warnings

None (`relationships.sameDomainActiveChanges` is empty; only this change touches `agent-chat` / `agent-tools`).

## Structured Status & ActionContext Findings

- Status engine (`gentle-ai.sdd-status`): artifacts all `done` except `syncReport: missing`; `applyState: all_done`; `dependencies`: apply `all_done`, verify `ready`, sync `blocked`, archive `blocked` — both blocks were due solely to the missing sync-report, which this archive-time sync resolves.
- `actionContext.mode: repo-local`; allowed edit roots: `/home/loonbac/Proyectos/shirley-show`. All writes stayed inside the workspace root. Server on `:3100`, `.run/`, and live E2E untouched (no runtime probes run during archive).
- `nextRecommended` was `sdd-verify` (stale relative to the persisted PASS verify report); superseded by this archive.

## Non-Blocking Documentation Note (verification WARNING, carried forward)

Verify report records a non-blocking WARNING: specs/design still mention the historical `NAME_MODEL_STEM` output template (delta `agent-tools` line 33) while the observed operational contract resolves `NAME_(STEM)_MODEL` with FLAC inputs; the design also describes `POLARFORMER_*` overrides the wrapper ignores. Per audit-trail rules the delta artifact was not silently edited; the template string is carried verbatim into canonical `openspec/specs/agent-tools/spec.md`. Recommendation: a small follow-up doc-change should correct the template token in canonical to `NAME_(STEM)_MODEL` under a fresh change, keeping the verified delta frozen as-is.

## Commands / Integrity

- Canonical merge performed by `/tmp/sdd_sync.py` (requirement-heading-exact matching, programmatic; report above drawn from its output). Post-merge validation: requirement counts 11 (`agent-chat`) / 9 (`agent-tools`), zero duplicate headings, no `MODIFIED`/`REMOVED` miss, no unbalanced code fences.
- Archive move: `openspec/changes/shirley-show-vocal-sep/` → `openspec/changes/archive/2026-09-11-shirley-show-vocal-sep/` (ISO date `2026-09-11`, matching the existing `2026-09-11-*` archive convention).
- Commit: local `main` only, no remote push, per parent instruction.

## Archived Path

`openspec/changes/archive/2026-09-11-shirley-show-vocal-sep/`