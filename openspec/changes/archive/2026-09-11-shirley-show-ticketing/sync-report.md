# Sync Report: `shirley-show-ticketing`

**Change:** `shirley-show-ticketing`
**Mode:** openspec (file-backed)
**Sync type:** archive-time sync fallback (parent/orchestrator explicitly instructed "Archiva según convención (8 dominios nuevos como specs canónicos: eventos, entradas, checkin, interesados, gastos, resumen, entrada-publica, ressy-ticketing + cierre)" for a file-backed archive with no prior `sdd-sync` run; treated as explicit approval of the archive-time sync fallback per the SDD archive contract)
**Status:** SUCCESS
**Date:** 2026-09-11

## Gate evidence before sync

- Verify report `verify-report.md`: verdict `pass_with_warnings`, 0 blockers, 0 critical findings, 31/31 requirements, 67/67 scenarios, `cargo test` 330 passed / 0 failed (exit 0), `cargo build` exit 0, `cargo fmt --check` clean, ticketing-scoped clippy clean. Residual warnings are non-critical (global clippy failures in non-ticketing tests, four CSS-class frontend assertions, documented SQLite/filesystem non-distributed transaction).
- Persisted `tasks.md` re-read immediately before sync fallback: 50/50 implementation tasks checked, 0 unchecked `- [ ]` markers (grep-verified). `apply-progress.md` corroborates all_done, 0 remaining.
- No same-domain active changes; canonical targets did not exist before this sync (verified: `openspec/specs/{domain}` absent for all 8 domains).

## Domains synced (all NEW canonical specs)

| Domain | Canonical target | Requirements | Operation |
|---|---|---|---|
| `eventos` | `openspec/specs/eventos/spec.md` | 5 | ADDED (new canonical) |
| `entradas` | `openspec/specs/entradas/spec.md` | 6 | ADDED (new canonical) |
| `checkin` | `openspec/specs/checkin/spec.md` | 3 | ADDED (new canonical) |
| `interesados` | `openspec/specs/interesados/spec.md` | 3 | ADDED (new canonical) |
| `gastos` | `openspec/specs/gastos/spec.md` | 3 | ADDED (new canonical) |
| `resumen` | `openspec/specs/resumen/spec.md` | 3 | ADDED (new canonical) |
| `entrada-publica` | `openspec/specs/entrada-publica/spec.md` | 4 | ADDED (new canonical) |
| `ressy-ticketing` | `openspec/specs/ressy-ticketing/spec.md` | 4 | ADDED (new canonical) |

Delta source: `openspec/changes/shirley-show-ticketing/specs/{domain}/spec.md` → `openspec/specs/{domain}/spec.md` (full spec copy, verified byte-identical via `diff -q`; no merge operations needed).

## ADDED requirement names

**eventos (5)**
1. Event Creation and Validation
2. Event Modification and Status Toggle (Finalizar / Reabrir)
3. Event Poster Storage, Upload Limit, and Safe Replacement
4. Event Deletion with Cascade and Asset Cleanup
5. Dynamic Capacity and Occupancy Calculation

**entradas (6)**
1. Multi-Row Transactional Ticket Issuance
2. Payment Medium and Status Invariants
3. Payment Confirmation Workflow
4. QR Concealment for Pending Tickets
5. Ticket Search, Filtering, and Listing
6. Ticket Cancellation and Deletion Guardrails

**checkin (3)**
1. Exclusive Verification by Secret QR Code
2. Three Discrete Validation Outcomes
3. Atomic and Concurrency-Safe Entry Registration

**interesados (3)**
1. Lead Registration and Attribute Validation
2. Status Lifecycle and Projected Capacity Influence
3. Conversion of Lead to Ticket Sale (Pase a Venta)

**gastos (3)**
1. Closed Expense Category Catalog
2. Expense Field Validation and Monetary Invariants
3. Expense Modification and Deletion

**resumen (3)**
1. Seven Core Event Metrics Calculation
2. Payment Method and Revenue Breakdown
3. Categorized Expense Breakdown

**entrada-publica (4)**
1. Public Ticket View by Scan Token
2. Minimal Safe Public Data Surface
3. Conditional QR Rendering and Payment Pending State
4. Client-Side WhatsApp Link Construction (wa.me)

**ressy-ticketing (4)**
1. Fixed Eight Tool Allowlist and Dispatcher
2. Grounded Context and Anti-Hallucination Invariants
3. Conversational Clarification Before Destructive or Financial Action
4. Accounting Truthfulness in Conversational Summaries

## MODIFIED / REMOVED

- MODIFIED: 0
- REMOVED: 0
- Destructive merge guard: N/A — no canonical domain deltas touched; the 8 pre-existing canonical specs (`agent-chat`, `agent-chat-ui`, `agent-tools`, `deploy-packaging`, `presentation`, `remote-gpu`, `ressy-remote-ux`, `show-item`) preserved untouched.

## Warnings

- None. No other active change touches any of the 8 synced domains (only active change is `shirley-show-ticketing`; collisions: none).

## Blockers

None.