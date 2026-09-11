# Archive Report: `shirley-show-yt-download`

**Change:** `shirley-show-yt-download`
**Archived at:** `openspec/changes/archive/2026-09-11-shirley-show-yt-download`
**Date:** 2026-09-11
**Mode:** openspec (file-backed)
**Archive status:** **PASS**

## Artifacts read

| Artifact | Path | Status |
|---|---|---|
| Proposal | `openspec/changes/shirley-show-yt-download/proposal.md` | ✅ read (28,781 bytes) |
| Explore | `openspec/changes/shirley-show-yt-download/explore.md` | ✅ read (2,096 bytes) |
| Spec deltas | `specs/agent-tools/spec.md`, `specs/agent-chat/spec.md` | ✅ read |
| Design | `openspec/changes/shirley-show-yt-download/design.md` | ✅ read (31,338 bytes) |
| Tasks | `openspec/changes/shirley-show-yt-download/tasks.md` | ✅ read (24/24 checked) |
| Apply progress | `openspec/changes/shirley-show-yt-download/apply-progress.md` | ✅ read (33,274 bytes) |
| Verify report | `openspec/changes/shirley-show-yt-download/verify-report.md` | ✅ read — **PASS** |
| Sync report | `openspec/changes/shirley-show-yt-download/sync-report.md` | ✅ SUCCESS (created this run via approved archive-time sync fallback) |
| Config | `openspec/config.yaml` | ✅ `schema: spec-driven`, `strict_tdd: true`, `rules.archive: "Warn before merging destructive deltas"` (applied, no destructive delta) |

No legacy flat `openspec/changes/shirley-show-yt-download/spec.md` is used as the only spec artifact; the change is domain-folder based and indexable.

## Verification gate

- `verify-report.md` verdict: **pass** (`schema: gentle-ai.verify-result/v1`, `evidence_revision sha256:8cbba5822e4cb0fd24de433e09663a5fab065f7f5ac9c7e06ed98458cdefa7f2`).
- Blockers: **0**; critical findings: **0**.
- Requirements: **10/10**; scenarios: **51/51**.
- `cargo test` exit 0; `cargo build` exit 0; `cargo check --tests` exit 0; `cargo fmt --all -- --check` exit 0 (no diff); focused suites 35 tests green; `node tests/agent-chat-integration.test.mjs` 8 groups green.
- No unresolved `FAIL` / `BLOCKED` / `CRITICAL` markers or verification blockers. All five previously reported critical blockers were remediated in `5df2495` and re-verified.
- TDD compliance: 6/6 checks satisfied.

## Task completion gate (final re-read immediately before sync fallback and move)

- Persisted `tasks.md` re-read: **24/24** implementation tasks checked (`- [x]`), **0 unchecked** `- [ ]` markers (grep-verified, `grep -n '^\s*- \[ \]'` returned no matches).
- `apply-progress.md` corroborates: slice tables for Slices 1–4 plus remediation, all tasks marked complete, no remaining tasks declared.
- **No stale-checkbox reconciliation performed** — not requested by the parent prompt and not needed, because no unchecked implementation task existed.
- No implementation task was repaired, added, or removed by this phase. Archive did not own task completion; `sdd-apply` had already persisted it.

## Sync (archive-time fallback, parent-approved)

- Prior `sdd-sync` had not been run (`sync-report.md` was missing). Parent prompt explicitly instructed archive "según convención (specs canónicos + cierre)" for a file-backed change with clean verification; per the archive contract this is treated as explicit approval of the archive-time sync fallback. Recorded in `sync-report.md`.
- Canonical specs already existed (created by `shirley-show-ai-agent`), so a real merge was applied by operation section rather than a full copy.

### Domains synced

| Domain | Canonical target | Before | After | Operations |
|---|---|---|---|---|
| `agent-tools` | `openspec/specs/agent-tools/spec.md` | 7 reqs / 229 lines / 32 scenarios | 8 reqs / 373 lines / 40 scenarios | 1 ADDED, 5 MODIFIED |
| `agent-chat` | `openspec/specs/agent-chat/spec.md` | 8 reqs / 232 lines / 25 scenarios | 10 reqs / 351 lines / 36 scenarios | 2 ADDED, 2 MODIFIED |

### ADDED requirement names (3)

- `agent-tools`: **YouTube Audio Download Tool and Execution Guardrails**
- `agent-chat`: **Ephemeral Staged Audio Registry with TTL Lifecycle**
- `agent-chat`: **Conversational Download Progress and Client State Continuity**

### MODIFIED requirement names (7)

- `agent-tools`: Fixed Tool Allowlist and Schema Definition
- `agent-tools`: Staged Track Creation Tool (add_track_from_staged)
- `agent-tools`: Presentation Identifier Integrity and Grounded Context
- `agent-tools`: Bounded Model Loop and Request Deadline
- `agent-tools`: Duplicate Tool Call Prevention within Turn
- `agent-chat`: Agent Chat API Endpoint and Content Negotiation
- `agent-chat`: Response Contract and Action Reporting

### REMOVED requirement names (0)

None.

### Preservation proof

For every MODIFIED requirement the merged canonical block is a strict superset of the previous canonical block: all canonical scenarios retained plus new ones. Programmatic superset check result — `agent-tools` gained 4 scenarios, `agent-chat` gained 3, with exactly one intentional supersession:

| Canonical scenario | Delta replacement | Verdict |
|---|---|---|
| `Global request deadline cancels long-running loop after 30 seconds` | `Global request deadline cancels long-running loop after 120 seconds` | Intentional declared requirement change (global deadline 30 s → 120 s via `SHIRLEY_AGENT_TIMEOUT_SECS`); same semantics at the new bound, not a silent scenario drop. |

Requirements not named by either delta were left byte-identical modulo section separators: `Presentation Lifecycle Tools`, `Item Creation Tools (add_lyrics, add_note)`, `Graceful Degradation Without API Key`, `Provider Error Handling and Resilience`, `Request-Scoped Audio Staging and Lifecycle Cleanup`, `Audio Attachment Validation and Limits`, `Ephemeral History and Context Size Bounds`, `Input Validation and Text Guardrails`.

### Active same-domain change warnings

- **`shirley-show-vocal-sep`** is active and declares deltas against the same two domains and largely the same requirement names (it also adds `Ephemeral Staged Audio Registry with TTL Lifecycle`, which this change has now added to canonical). Its delta was authored against the pre-merge canonical text, so its future sync must re-read the new canonical state; the `ADDED: Ephemeral Staged Audio Registry with TTL Lifecycle` entry will now collide. No canonical corruption was caused by this merge, but the collision is real and must be resolved when `vocal-sep` is synced.

## Destructive merge approvals / blockers

- `openspec/config.yaml` `rules.archive` ("Warn before merging destructive deltas") was applied.
- **REMOVED requirements: 0** — nothing deleted from any canonical spec.
- 7 MODIFIED requirement blocks were replaced wholesale (full-requirement replacement, never partial scenario edits). No MODIFIED delta was partial; every block was complete with its full scenario set.
- One intentional scenario supersession (30 s → 120 s deadline) was listed and summarized before the write; it is not a destructive deletion because the replacement scenario preserves the same normative semantics at the new bound, and the bound change is itself the deliverable of this change.
- No additional destructive-sync approval was required beyond the parent's explicit "specs canónicos" instruction. No blocker.

## Delivery state

- Commits on local `main` only; **no git remote configured** (`git remote -v` empty). No PR created; delivery intentionally remains local.
- Change commits: `22225a4` (feat yt-s1), `c3bb057` (feat yt-s2), `4e31276` (feat yt-s3), `a69f7fb` (feat yt-s4), `5df2495` (fix: 5 verify blockers + real MP3 320K proof), `2aa5158` (docs: verify PASS).
- Review workload: `tasks.md` forecast 980–1180 changed lines with **High** 400-line budget risk. `apply-progress.md` records resolution as 4 chained `stacked-to-main` PRs with explicit maintainer acceptance of `size:exception` for S1–S4, plus a bounded remediation slice `fix-yt-verify-blockers` (max budget 500 lines). `verify-report.md` confirms no functional scope creep during re-verification.

## Structured status and actionContext findings

- Change: `shirley-show-yt-download`; `artifactStore: openspec`; authoritative store: openspec filesystem. `isNonAuthoritative: false`.
- Native status at archive time reported `nextRecommended: "Change selection is ambiguous: shirley-show-vocal-sep, shirley-show-yt-download"` with all dependencies `blocked`. The parent prompt and the user request unambiguously selected `shirley-show-yt-download`, which resolved the ambiguity; the store is authoritative (filesystem-backed with an `openspec/` directory), so the blocked dependencies were verified directly against the artifacts rather than trusted as real blockers.
- `actionContext.mode`: `repo-local`; `workspaceRoot` and `allowedEditRoots`: `/home/loonbac/Proyectos/shirley-show`. All writes — sync targets, `sync-report.md`, `archive-report.md`, and the archive move — are inside the authoritative workspace root; no `allowedEditRoots` violation.
- `relationships`: no `dependsOn`/`supersedes`/`conflictsWith`; `sameDomainActiveChanges` includes `shirley-show-vocal-sep` (warning recorded above).
- Attempt authority in `verify-report.md` (token `sha256:f7fe0ef1…`, request `yv-acquire-002`): **no settle executed**. Archive did not execute settlement.

## Non-critical findings carried from verification (no archive impact)

1. `StagedAudioRegistry` auxiliary getters do not opportunistically purge; a tombstone consumed after expiry during a reserve may remain in memory. `reserve`, the TTL task, and `purge_expired` still block expired consumption and delete temp files.
2. Documentation copy drift: delta literals still mix `Shirley`/bare phrasing while shipped UI and tests use the `Ressy` brand.
3. Assertion quality: 0 CRITICAL, 19 WARNING (18 UI assertions coupled to presentation details, 1 weak non-persistence check).
4. Live `yt-dlp → MP3` E2E (`ffprobe` reporting `codec_name=mp3`, `bit_rate=320000`) was orchestrator-provided external evidence, not reproduced by the verifier because the server remained off and `.run/` was not touched.

## Additional documentation debt recorded by this phase

- `openspec/specs/agent-tools/spec.md` `## Purpose` (line 5) still reads "…iteration limits, 30-second global deadline…" while the merged `Bounded Model Loop and Request Deadline` requirement now specifies 120 seconds. Deliberately left unedited: `## Purpose` is outside the declared delta operation set, and `verify-report.md` explicitly defers documentation copy reconciliation to a future sync without blocking archive.

## Memory traceability (Engram, project `shirley-show`)

| Artifact | Observation ID | `topic_key` |
|---|---|---|
| Explore | 319 | `sdd/shirley-show-yt-download/explore` |
| Spec (deltas) | 320 | `sdd/shirley-show-yt-download/spec` |
| Design | 322 | `sdd/shirley-show-yt-download/design` |
| Tasks | 323 | `sdd/shirley-show-yt-download/tasks` |
| Apply progress | 325 (rev 4) | `sdd/shirley-show-yt-download/apply-progress` |
| Verify — initial blockers | 340 | `verification/shirley-show-yt-download/blockers` |
| Verify — re-verification PASS | 341 | `verification/shirley-show-yt-download/5df2495` |
| Proposal | — (filesystem only) | none persisted to Engram |
| Archive report | saved this run | `sdd/shirley-show-yt-download/archive-report` |

## Archived path

`/home/loonbac/Proyectos/shirley-show/openspec/changes/archive/2026-09-11-shirley-show-yt-download/`

## Blockers

None.
