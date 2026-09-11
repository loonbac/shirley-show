# Delta for Presentation

## MODIFIED Requirements

### Requirement: List Presentations

1. The backend API MUST return an enriched presentation summary for each presentation in `GET /api/presentations` including stable identifier, title, timestamps, counts per item kind (`tracks_count`, `lyrics_count`, `notes_count`), `total_items`, and up to three ordered `preview_items`.
2. The system MUST render the catalog as a warm home library featuring a personal welcome header with aggregate library counts, a prominent presentation creation action, and rich presentation cards.
3. Each presentation card MUST display repertoire identity, human-readable recency, colored badges for each item kind, a setlist preview of up to three initial items, a dominant primary "Abrir show" action with touch target of at least 48 px, and contained secondary rename and delete actions.
4. When no presentations exist, the system MUST render a warm empty state with a prominent action to create the first presentation.
(Previously: Listed basic presentations returning only identifiers and titles without item counts, preview snippets, or warm library card styling.)

#### Scenario: List existing presentations with enriched counts and setlist preview
- GIVEN presentations exist in storage with track, lyrics, and note items
- WHEN the client requests `GET /api/presentations` and renders the catalog
- THEN the system MUST return HTTP status 200 with presentation summaries including `tracks_count`, `lyrics_count`, `notes_count`, `total_items`, and up to three ordered `preview_items`, and the home view MUST render rich presentation cards displaying these counts, a preview snippet, and a prominent "Abrir show" button

#### Scenario: Presentation with zero items rendered in catalog
- GIVEN an existing presentation that contains zero items
- WHEN the user accesses the catalog view
- THEN the system MUST display the presentation card with all type counts at 0, an indicator that the repertoire has no items yet, and a prominent "Abrir show" button

#### Scenario: Empty presentations catalog
- GIVEN no presentations exist in storage
- WHEN the user accesses the presentation list view
- THEN the system MUST display a warm explanatory empty state explaining that no presentations exist yet, MUST omit aggregate library summary counters, and MUST prominently display an action to create the first presentation

#### Scenario: Open presentation from dominant card action
- GIVEN a presentation card rendered in the home library
- WHEN the user taps or clicks the dominant "Abrir show" button (touch target >= 48 px)
- THEN the system MUST navigate directly to that presentation's detail view
