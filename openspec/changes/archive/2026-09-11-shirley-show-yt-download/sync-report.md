# Sync Report: `shirley-show-yt-download`

**Change:** `shirley-show-yt-download`
**Mode:** openspec (file-backed)
**Sync type:** archive-time sync fallback (parent/orchestrator explicitly instructed "Archiva según convención (specs canónicos + cierre)" for a file-backed archive with no prior `sdd-sync` run; treated as explicit approval of the archive-time sync fallback per the SDD archive contract)
**Status:** SUCCESS
**Date:** 2026-09-11

## Gate evidence before sync

- Verify report `verify-report.md`: verdict `pass`, 0 blockers, 0 critical findings, 10/10 requirements, 51/51 scenarios, `cargo test` exit 0, `cargo build` exit 0, `cargo fmt --all -- --check` clean.
- Persisted `tasks.md` re-read immediately before sync: 24/24 implementation tasks checked, 0 unchecked `- [ ]` markers (grep-verified).
- Canonical targets `openspec/specs/agent-tools/spec.md` and `openspec/specs/agent-chat/spec.md` already existed (created 2026-09-11 by `shirley-show-ai-agent`), so merge operations applied.
- Same active domain warning: `shirley-show-vocal-sep` also declares deltas against `agent-tools` and `agent-chat` (see Warnings).

## Domains synced

| Domain | Canonical target | Before | After | Operation |
|---|---|---|---|---|
| `agent-tools` | `openspec/specs/agent-tools/spec.md` | 7 reqs / 229 lines | 8 reqs / 373 lines | 5 MODIFIED + 1 ADDED |
| `agent-chat` | `openspec/specs/agent-chat/spec.md` | 8 reqs / 232 lines | 10 reqs / 351 lines | 2 MODIFIED + 2 ADDED |

Delta source: `openspec/changes/shirley-show-yt-download/specs/{domain}/spec.md`.

## ADDED requirement names

**agent-tools (1)**
1. YouTube Audio Download Tool and Execution Guardrails

**agent-chat (2)**
1. Ephemeral Staged Audio Registry with TTL Lifecycle
2. Conversational Download Progress and Client State Continuity

## MODIFIED requirement names (replaced by full delta requirement block)

**agent-tools (5)**
1. Fixed Tool Allowlist and Schema Definition
2. Staged Track Creation Tool (add_track_from_staged)
3. Presentation Identifier Integrity and Grounded Context
4. Bounded Model Loop and Request Deadline
5. Duplicate Tool Call Prevention within Turn

**agent-chat (2)**
1. Agent Chat API Endpoint and Content Negotiation
2. Response Contract and Action Reporting

## REMOVED requirement names

None. No `## REMOVED Requirements` section in either delta.

## Preserved requirements (untouched, byte-identical modulo separators)

- `agent-tools`: Presentation Lifecycle Tools; Item Creation Tools (add_lyrics, add_note)
- `agent-chat`: Graceful Degradation Without API Key; Provider Error Handling and Resilience; Request-Scoped Audio Staging and Lifecycle Cleanup; Audio Attachment Validation and Limits; Ephemeral History and Context Size Bounds; Input Validation and Text Guardrails

## Destructive merge guard

`openspec/config.yaml` `rules.archive` requires warning before merging destructive deltas.

- REMOVED requirements: **0** — nothing deleted from any canonical spec.
- MODIFIED blocks: 7 requirement blocks replaced wholesale (full-block replacement, not partial scenario edits). Every MODIFIED block is a strict superset of the canonical block, except one declared and intentional scenario supersession:

  | Canonical scenario | Delta replacement | Verdict |
  |---|---|---|
  | `Global request deadline cancels long-running loop after 30 seconds` | `Global request deadline cancels long-running loop after 120 seconds` | Intentional requirement change (global deadline 30s → 120s, `SHIRLEY_AGENT_TIMEOUT_SECS`). Same semantics at the new bound; not a silent scenario drop. |

- No other scenario was dropped. Verified programmatically: for each MODIFIED requirement, the delta block contains every canonical scenario (plus new ones).
- Scenario-count deltas (canonical → merged): `agent-tools` 32 → 40; `agent-chat` 25 → 36.
- Approval: parent prompt explicitly approved canonical spec sync ("specs canónicos"). No REMOVED requirement and no non-superset replacement, so no additional destructive approval was required.

## Warnings

1. **Same-domain active change collision:** `shirley-show-vocal-sep` (active, not yet applied) declares deltas against the same two domains and the same requirement names:
   - `agent-tools`: adds `Vocal Separation Tool and GPU Execution Guardrails`; modifies `Fixed Tool Allowlist and Schema Definition`, `Staged Track Creation Tool (add_track_from_staged)`, `Presentation Identifier Integrity and Grounded Context`, `Bounded Model Loop and Request Deadline`, `Duplicate Tool Call Prevention within Turn`.
   - `agent-chat`: adds `Ephemeral Staged Audio Registry with TTL Lifecycle`, `Conversational Vocal Separation Feedback and Client State Continuity`; modifies `Agent Chat API Endpoint and Content Negotiation`, `Response Contract and Action Reporting`.
   
   This merge did not corrupt `shirley-show-vocal-sep`, but that change's delta was authored against the *pre-merge* canonical text. When `vocal-sep` is synced, it will re-replace the same blocks and must be re-read against this new canonical state (its `ADDED: Ephemeral Staged Audio Registry with TTL Lifecycle` will now collide with the requirement already added by this change).

2. **Canonical `Purpose` staleness (documentation debt, non-blocking):** `openspec/specs/agent-tools/spec.md` line 5 still reads "…iteration limits, 30-second global deadline…". The merged requirement `Bounded Model Loop and Request Deadline` now specifies 120 seconds. Deliberately left unedited because `## Purpose` is outside the declared delta operation set and the verify report explicitly defers documentation copy reconciliation to a future sync without blocking archive.

3. **Naming drift (`Shirley` vs `Ressy`) carried from verification:** some delta literals still say `Shirley` where the shipped UI/tests use the `Ressy` brand. Non-blocking; noted by `verify-report.md` as future copy reconciliation.

4. **Deviation from prior convention:** the archived `shirley-show-ai-agent` sync created canonical specs as full copies. This sync performed a real merge because the canonical specs already existed.

## Blockers

None.
