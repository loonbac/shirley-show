# Ticket Image Sharing

- **Objective:** Make ticket issuance immediately present the generated ticket image and provide native file sharing on phones, without a redundant “Ver imagen de la entrada” step or instructional wall of text.
- **Problem:** The sale-success view requires an extra click to reach an image dialog, and that dialog explains a clipboard workaround instead of offering the mobile system share sheet.
- **Why:** The expected task after issuing a ticket is to view and send its image directly, including to WhatsApp through the operating system share sheet.
- **Scope:** Ticket issuance success UI, ticket image presentation/sharing, focused frontend tests/styles as needed.
- **Constraints:** Preserve support for multiple issued tickets and unpaid ticket behavior; show native sharing only where file sharing is supported; keep a sensible non-mobile fallback. Do not change unrelated ticket flows. No commit unless explicitly requested.
- **TDD:** Off (no configured TDD selection is present); use existing focused functional tests.
- **Route:** Delegated direct implementation. Trigger: multi-file UI behavior and tests; exploration mapped the existing flow.

## Tasks

- [x] **TIS-1 — Replace the redundant image step with immediate ticket image presentation and native share actions.** Include each issued ticket, preserve payment restrictions, remove obsolete instructional copy, and add focused tests.
  - Acceptance: successful issuance immediately shows the ticket image(s); no “Ver imagen de la entrada” button or clipboard instructions remain; share invokes native file sharing from a user tap where supported; cancellation is handled quietly; unsupported environments retain a usable fallback.
  - Checks: `node frontend/test/events-ui.test.js`; `node frontend/test/public-ticket.test.js`; mechanical Impeccable detector on changed UI targets.
  - Progress: complete.
  - Verification evidence: final independent verification passed `node frontend/test/events-ui.test.js`, `node frontend/test/public-ticket.test.js`, and `git diff --check`; image cards provide per-ticket share/download, and the detector reported only two pre-existing warnings in unchanged CSS lines.
  - Route trigger: delegated direct multi-file write; focused test execution completed by writer.
- [x] **TIS-2 — Prevent background scrolling while a modal is open.** Fix the shared modal behavior rather than patching only the ticket view; handle cleanup and nested/concurrent dialogs safely, with focused regression coverage.
  - Acceptance: opening a modal prevents page scrolling behind it; dismissing/destroying it restores the prior scroll behavior; nested dialogs do not unlock scrolling prematurely.
  - Checks: focused frontend tests (including modal lifecycle/cleanup) and UI detector for changed targets.
  - Progress: complete.
  - Verification evidence: final independent verification passed `node frontend/test/dialog-dismiss.test.js`; fixed-body lock restores prior styles and scroll position, with nested/unordered cleanup coverage. `git diff --check` passed; detector warnings are pre-existing in unchanged CSS.
  - Route trigger: delegated mapping (shared modal behavior spans multiple implementations/styles/tests) then delegated direct multi-file write.

- [x] **TIS-3 — Align paid-status handling across ticket image surfaces.** Ensure missing/unknown payment status cannot permit image generation/sharing in one surface while blocking it in another; add a focused regression test.
  - Acceptance: image generation/sharing eligibility is consistent, and only explicitly paid tickets are shareable.
  - Checks: `node frontend/test/events-ui.test.js`.
  - Progress: complete.
  - Verification evidence: final independent `node frontend/test/events-ui.test.js` passed; regression coverage includes missing, null, unknown, pending, and paid statuses. Generator and both UI paths require explicit `Pagado`.
  - Route trigger: delegated direct bounded correction after independent review found an inconsistent fallback.

## Next Step

Implementation commit: `ce360a3` (`feat(events): show and share ticket images immediately`), merged to `main` by fast-forward. Native review approved and acknowledged the reviewed candidate; no push was performed.