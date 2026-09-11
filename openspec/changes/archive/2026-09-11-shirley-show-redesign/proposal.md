# Proposal: Shirley Show Redesign

**Change:** `shirley-show-redesign`  
**Status:** Proposed  
**Product owner:** Shirley  
**Execution mode:** Auto  
**Delivery strategy:** Ask on risk  
**Review budget:** 400 changed lines per review unit

## Intent

Redesign Shirley Show as a warm, elegant, tactile personal stage tool rather than a generic CRUD interface. The product should help Shirley prepare a repertoire from the home library and operate a live show confidently from a phone mounted on a music stand, with fast content changes and minimal risk of accidental actions.

The change will preserve the existing local-first Rust, Axum, SQLite, filesystem audio, and vanilla JavaScript architecture while adding the smallest backend capabilities required by the redesigned workflows.

## Product problem

The current interface exposes the underlying records and actions but does not communicate the identity, readiness, or sequence of a show. The home is visually sparse, presentation cards lack useful summaries, adding and editing content is cumbersome, reordering is optimized for buttons rather than touch, and there is no distraction-free live-performance mode.

This makes a functional product feel weak and administrative at the moments when Shirley needs it to feel reassuring, personal, and stage-ready. It also creates avoidable operational cost: correcting a typo may require deleting and recreating an item, understanding a repertoire requires opening it, and using the app live leaves editing controls exposed.

## Target user and situations

The primary and only in-scope user is Shirley, using a personal local application without authentication or multi-user collaboration.

The experience must support two main situations:

1. **Preparing a show:** on phone or desktop, Shirley reviews her presentation library, opens a repertoire, adds tracks, lyrics, and notes, makes quick corrections, and rearranges the running order.
2. **Performing live:** primarily on a phone mounted on a music stand, Shirley reads large lyrics and cues, starts or pauses one backing track at a time, and keeps the screen awake without seeing destructive editing controls.

The performance situation has the higher urgency and the lower tolerance for ambiguity, visual clutter, interrupted audio, or accidental mutation.

## Product outcome

After this change:

- The home should feel like a personal music library, with a warm welcome and presentation cards that show repertoire identity, content counts, and a short setlist preview.
- A presentation should feel like one physical running-order notebook composed of clearly differentiated track, lyrics, and note blocks.
- Adding an item should begin with one obvious tap on `+ Pista`, `+ Letra`, or `+ Nota`.
- Correcting an existing item should be possible through a focused quick-edit dialog without deletion and recreation.
- Reordering should feel direct on touch devices through a dedicated drag handle while retaining explicit accessible movement controls.
- Stage Mode should provide a fullscreen, high-readability, low-risk performance surface with large playback controls, adjustable text scale, no editing actions, and best-effort wake lock.
- The visual language should feel warm, intimate, elegant, and physical—not corporate, grey, or dashboard-like.

## Current-state gap

### Home library

`GET /api/presentations` currently returns only presentation identity and timestamps. The frontend therefore cannot show per-type counts or a preview without extra requests. Existing presentation cards resemble simple list rows, and rename/delete actions compete visually with the primary task of opening a show.

### Presentation preparation

The sequence is already canonical and ordered, but item creation requires a modal and type-tab flow. Existing item controls are small, reordering is limited to adjacent up/down actions, and no endpoint supports editing an item's title or body.

### Live performance

The application has exclusive audio playback and mobile range streaming, but it lacks a dedicated stage surface. Editing and destructive controls remain part of the same interface, text scale is not stage-oriented, fullscreen is absent, and the device may sleep during a performance.

## Product rules and invariants

The redesign MUST preserve the canonical behavior in `openspec/specs/presentation/spec.md` and `openspec/specs/show-item/spec.md`.

1. A presentation keeps a stable identifier across rename operations and owns exactly one ordered item sequence.
2. Item positions remain contiguous integers from `0` through `N-1` after creation, movement, and deletion.
3. Empty or whitespace-only presentation titles, track titles, lyrics, and notes remain invalid.
4. Track creation still requires a supported audio file and visible non-empty title; accepted formats and the 100 MiB limit do not change.
5. At most one track may play at a time, and playback never starts automatically.
6. Navigating away from a presentation stops and releases playback. Entering or leaving Stage Mode within the same presentation must not interrupt active playback.
7. Missing media must produce a visible recoverable error without disabling the rest of the show.
8. Presentation and item deletion remain explicit-confirmation actions and retain their database, file cleanup, playback-stop, and position-recompaction guarantees.
9. Dragging is available only from a dedicated handle. The rest of each card preserves native vertical scrolling.
10. Explicit `Subir` and `Bajar` controls remain available as accessible and keyboard-operable fallback behavior, including first/last boundary guards.
11. Stage Mode exposes no edit, delete, or drag actions.
12. All primary touch controls target at least 48 px where practical and never fall below the canonical 44 × 44 px playback requirement.
13. Fullscreen and Screen Wake Lock are progressive enhancements. Browser denial or lack of support must not prevent Stage Mode from operating as a fixed-viewport experience.
14. Existing audio playback, streaming, local persistence, and route-staleness behavior must remain intact.

## Scope

### Slice 1 — Backend editing and enriched catalog

- Add `PATCH /api/presentations/{presentation_id}/items/{item_id}`.
- Permit a track title update or a lyrics/note body update according to item kind.
- Trim and reject empty values; reject fields inconsistent with the item kind.
- Update the parent presentation's `updated_at` after a successful item edit.
- Enrich `GET /api/presentations` with `tracks_count`, `lyrics_count`, `notes_count`, `total_items`, and up to three ordered preview items.
- Preserve stable presentation identifiers and existing catalog lifecycle behavior.
- Optionally extend the move request with `target_position` for one-transaction absolute movement while preserving the existing `direction` contract.
- If `target_position` is implemented, clamp or reject out-of-range positions consistently, execute under an immediate transaction, and recompact the affected range without duplicates or gaps.

### Slice 2 — Warm visual home library

- Introduce a `welcome-header` organism with a personal greeting, useful aggregate summary, and prominent creation action.
- Redesign `presentation-card` as a rich library card with title, human-readable recency, per-type badges, setlist preview, and a clear `Abrir show` action.
- Move rename and delete into visually secondary actions while preserving discoverability, confirmation, and keyboard access.
- Retain the canonical empty state and prominent first-presentation action.
- Refine reusable tokens and component-scoped styles around warm ivory, aged gold, soft elevation, display typography, and content-type accents.

### Slice 3 — Tactile sequence, immediate add, and quick edit

- Redesign track, lyrics, and note organisms as large tactile blocks with distinct violet, deep-cyan, and warm-amber accents.
- Add an explicit three-action `quick-add-bar` for track, lyrics, and note flows; each successful item still appends to the sequence.
- Add a focused `quick-edit-dialog` or mobile drawer that preloads the editable value, validates it, exposes clear save/cancel behavior, and reports server errors without discarding the user's draft.
- Add the frontend item-update data client and integrate it with presentation detail refresh/state.
- Preserve deletion confirmation and adjacent movement controls.

### Slice 4 — Handle-based reordering and Stage Mode

- Add a headless pointer-event drag behavior attached only to the drag handle.
- Show temporary visual insertion feedback during drag and persist a reorder only after release.
- Keep the current sequence if the pointer is cancelled, the request fails, or no effective position change occurred.
- Add Stage Mode as a full-viewport overlay with fullscreen enhancement, high-contrast large lyrics, distinctive notes, large one-tap track controls, text scaling, and an obvious exit action.
- Reuse the singleton audio controller and playback signal so changing modes does not recreate or restart audio.
- Add a headless wake-lock behavior that acquires while Stage Mode is active, releases on exit, and best-effort reacquires after visibility restoration.

## Affected areas

### Backend

- Presentation list query and response DTOs.
- Show-item write port/trait, update use case, handler, route registration, and SQLite implementation.
- Optional absolute-position reorder request and transaction logic.
- Integration coverage for item updates, enriched summaries, validation, not-found behavior, and ordering invariants.

### Frontend

- Catalog and presentation-detail views/controllers.
- Presentation card and show-item organisms.
- New welcome header, quick-add bar, quick-edit dialog, and Stage Mode components.
- New drag-and-drop and wake-lock headless behaviors.
- Item update data client and catalog response mapping.
- Design tokens plus component-scoped CSS.

### User workflows and operations

- Preparation becomes faster through visible repertoire summaries, direct type choice, editing, and direct manipulation.
- Live operation gains a safer dedicated mode while continuing to use the existing local server and filesystem media.
- No migration of existing SQLite records or audio files is expected.

## Edge cases and failure behavior

- **No presentations:** show a warm explanatory empty state and a prominent creation action instead of an empty summary.
- **Empty presentation:** show all three add choices prominently; Stage Mode may open but must clearly explain that the show has no items.
- **Partial or stale summary data:** render safe zero counts and omit unavailable preview rows rather than blocking the catalog.
- **Stale/deleted route:** inform Shirley and return safely to the library as required by the canonical presentation spec.
- **Concurrent or stale item edit:** a not-found response keeps the draft visible, explains that the item is unavailable, and offers dismissal or refresh; it must not create a replacement item implicitly.
- **Invalid edit fields:** reject type-inconsistent payloads and preserve the original persisted item.
- **Long lyrics:** preserve multiline formatting and scrolling; text scaling must not hide the Stage Mode exit control.
- **Long titles or notes:** wrap without shrinking touch targets or overlapping actions.
- **Failed catalog enrichment:** the API returns an error rather than silently fabricated counts; the UI presents a recoverable loading/error state.
- **Drag versus scroll:** only the handle suppresses touch scrolling. Pointer cancellation or release outside the list restores the original visual order.
- **Reorder request failure:** restore the server-confirmed ordering and announce the failure; never leave optimistic duplicate or missing positions.
- **First/last item:** inaccessible drag destinations and adjacent movement boundaries cause no mutation.
- **Fullscreen denied or unsupported:** Stage Mode remains usable in a fixed full-viewport overlay and communicates no alarming failure.
- **Wake Lock denied, released, or unsupported:** performance controls continue working; the app may show a discreet status but must not repeatedly interrupt Shirley.
- **Tab/background transitions:** release or tolerate automatic wake-lock release and reacquire only while Stage Mode remains active and visible.
- **Audio already playing:** entering/exiting Stage Mode preserves the current track and elapsed position; selecting another track still enforces exclusive playback.
- **Missing audio:** retain the rest of Stage Mode and expose the canonical unavailable-media error.
- **Upload failure:** retain existing cleanup of partial files and do not create an item.

## Constraints

- Keep Rust + Axum, SQLite, disk-based audio, and vanilla JavaScript; do not introduce a frontend framework or remote service.
- Follow the repository's extreme micro-modularity standard: one focused module per function/behavior, pure UI components supplied through data and callbacks, headless interaction behaviors, and component-scoped CSS.
- Do not access network or persistence directly from atoms or molecules.
- Follow strict TDD where the detected runner applies, with `cargo test` as the canonical backend test runner.
- Do not modify `.run/` manually; tests must use isolated temporary directories.
- Preserve the existing schema where practical. Catalog enrichment and item editing should use existing item metadata; any schema migration requires separate justification.
- Keep each review unit at or below the 400 changed-line budget. The four slices are independent review units in dependency order. Under `ask-on-risk`, pause for a delivery decision before implementation if a slice is forecast to exceed that budget; do not infer a size exception or chain strategy.
- Accessibility, native scroll, keyboard fallback, and reduced accidental action risk take precedence over visual novelty.

## Tradeoffs and decisions

1. **Rich summaries in one catalog request over per-card fetching:** this increases backend query/DTO complexity but avoids request fan-out, empty cards, and visible loading churn.
2. **Dedicated drag handle over whole-card dragging:** this slightly reduces the grab area but protects one-handed native scrolling and makes intent explicit.
3. **Absolute movement as optional backend enhancement:** it improves long-distance drag efficiency, but adjacent atomic controls remain canonical and sufficient. Slice 4 may map a drop to bounded adjacent requests if `target_position` is deferred, provided ordering stays correct and UX remains responsive.
4. **Overlay plus progressive fullscreen over fullscreen dependency:** browser permissions and platform support are unreliable, so the core stage experience cannot depend on successful `requestFullscreen()`.
5. **Best-effort Wake Lock over guaranteed screen control:** the browser owns permission and lifecycle; product correctness is defined by graceful degradation rather than guaranteed acquisition.
6. **No edit controls in Stage Mode:** live safety is prioritized over edit convenience. Corrections require returning to preparation mode.
7. **Warm physical styling over dense information:** spacious touch-first composition may show fewer items at once, but readability and confidence on a music stand are the primary outcome.

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Catalog aggregation becomes slow as data grows | Home feels delayed | Use one bounded query strategy, index-aware ordering, and cap previews at three items. |
| Item-kind update rules become ambiguous | Data fields drift or become invalid | Validate allowed fields by persisted kind and reject incompatible payloads atomically. |
| Touch drag captures page scroll | Presentation becomes unusable on mobile | Bind pointer capture and `touch-action: none` only to the dedicated handle. |
| Failed optimistic reorder desynchronizes UI | Running order becomes untrustworthy | Commit only on release, serialize mutations, and restore server-confirmed order on failure. |
| Stage Mode remount interrupts audio | Backing track stops during performance | Reuse the singleton audio controller and update only reactive controls. |
| Fullscreen or wake lock is denied | Reduced stage convenience | Provide fixed-viewport fallback and treat both APIs as optional capabilities. |
| Large text obscures controls | Shirley cannot exit or navigate safely | Keep exit and scale controls in a stable safe-area-aware layer and test long content. |
| Secondary destructive actions remain too easy | Accidental data loss | De-emphasize them visually while retaining explicit confirmation and cleanup invariants. |
| Four slices exceed review budget | Review quality and delivery policy degrade | Forecast each slice before implementation and invoke the ask-on-risk gate before exceeding 400 changed lines. |

## Rollback plan

Rollback is slice-based so later experience layers can be removed without corrupting existing presentations or audio.

1. **Slice 4 rollback:** remove Stage Mode, wake-lock, and drag behavior integrations; retain canonical up/down controls and the preparation view. If absolute movement was added, the backend continues accepting existing `direction` requests, so the new field can be ignored or removed in a compatible follow-up.
2. **Slice 3 rollback:** restore existing show-item rendering and add dialog while leaving the additive PATCH endpoint unused. Existing items remain valid because no storage shape changes.
3. **Slice 2 rollback:** restore the prior catalog components and ignore enriched response fields. Backend response compatibility should retain the pre-existing presentation fields.
4. **Slice 1 rollback:** revert route, use case, and summary DTO/query changes together. Because this proposal expects no database migration, rollback does not require data conversion. Any item edits completed before rollback remain ordinary values in existing columns.
5. **Operational recovery:** stop the local server before binary rollback, preserve the SQLite database and managed audio directory, restart the prior known-good version, and verify catalog opening, sequence order, and audio playback. Never manually edit `.run/` as part of rollback.

Rollback must not delete presentations, items, or audio. If implementation discovers that a schema migration is unavoidable, that migration and its tested backward/down migration become a new approval requirement before proceeding.

## Success criteria

The proposal is successful when all of the following are true:

1. The catalog presents every existing presentation as a warm, information-rich card with accurate type counts, total count, up to three ordered preview entries, and a dominant open action.
2. The zero-presentation state remains clear and offers an immediate first-creation action.
3. Shirley can start adding a track, lyrics, or note through three distinct one-tap choices, and successful additions still append to the canonical sequence.
4. Shirley can edit a track title or lyrics/note body without deleting the item; invalid or type-inconsistent updates preserve existing data and show actionable feedback.
5. The presentation sequence remains contiguous and correctly ordered after button movement, successful drag movement, rapid requests, cancellation, and failures.
6. Native vertical scrolling works from every non-handle part of an item card, and accessible up/down controls remain operable.
7. Stage Mode displays large readable lyrics, distinctive notes, and large track controls while exposing no mutation controls.
8. Entering and leaving Stage Mode does not restart, stop, or duplicate currently playing audio; navigation away still stops playback.
9. At most one audio track plays, no track autoplays, and missing media fails gracefully.
10. Stage Mode remains usable when fullscreen or Wake Lock is unavailable or denied, and Wake Lock is released when the mode ends.
11. Existing presentation rename/delete, item delete/cleanup, upload validation, audio range streaming, and stale-route behaviors do not regress.
12. The implementation follows micro-modular boundaries and each review unit stays within 400 changed lines unless the user explicitly chooses a delivery strategy at the ask-on-risk gate.

## Non-goals

- Authentication, accounts, roles, permissions, or multi-user collaboration.
- Artificial intelligence, generated lyrics, transcription, chord detection, or repertoire recommendations.
- Cloud deployment, hosting, CI/CD production setup, or remote storage.
- Audio editing, trimming, mixing, effects, equalization, or transcoding.
- PDF, print, document, or office-format export.
- Changes to supported audio formats or the 100 MiB upload limit.
- A generalized design-system rewrite beyond components and tokens needed by these four slices.
- Collaborative live synchronization, remote control, or cross-device state sharing.

## Delivery sequence

Implement in dependency order:

1. Backend item editing and enriched catalog contract.
2. Home visual library consuming the enriched contract.
3. Tactile sequence, immediate addition, and quick edit.
4. Handle-based reorder and Stage Mode.

Each slice must be independently reviewable and leave the application usable. Optional `target_position` support belongs to Slice 1 only if it fits the review budget and can preserve the canonical adjacent movement contract; otherwise it is deferred without blocking the remaining product outcome.
