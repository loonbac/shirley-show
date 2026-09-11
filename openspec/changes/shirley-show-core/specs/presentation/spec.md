# Presentation Specification

## Purpose

Defines the management, lifecycle, catalog, and persistence invariants for presentations in Shirley Show. A presentation is the top-level container that groups ordered show items for a rehearsal, acoustic set, or live musical performance.

## Requirements

### Requirement: List Presentations

The system MUST list all existing presentations in the catalog and provide an accessible entry point to create, open, rename, and delete presentations.

#### Scenario: List all existing presentations
- GIVEN presentations exist in storage
- WHEN the user accesses the presentation list view
- THEN the system MUST return and display all presentations with their titles and stable identifiers in the catalog interface

#### Scenario: Empty presentations catalog
- GIVEN no presentations exist in storage
- WHEN the user accesses the presentation list view
- THEN the system MUST display an empty state explaining that no presentations have been created yet and MUST display a prominent action to create the first presentation

### Requirement: Create Presentation

The system MUST create a presentation with a user-provided non-empty title, trim leading and trailing whitespace, assign a persistent stable identifier, and initialize the presentation with an empty item sequence.

#### Scenario: Successful presentation creation with valid name
- GIVEN the user provides a presentation name "Viernes 24 Acústico"
- WHEN the user confirms presentation creation
- THEN the system MUST persist a new presentation with the trimmed title "Viernes 24 Acústico" and a generated stable identifier, initialize its item sequence as empty, and make it available in the catalog

#### Scenario: Reject presentation creation with empty or whitespace-only name
- GIVEN the user submits a presentation name consisting solely of whitespace or an empty string
- WHEN the user attempts to create the presentation
- THEN the system MUST reject the request, MUST NOT persist any new presentation record, and MUST present a validation error requiring a non-empty name

#### Scenario: Multiple presentations with identical visible names
- GIVEN an existing presentation named "Ensayo General"
- WHEN the user creates another presentation with the same name "Ensayo General"
- THEN the system MUST assign a distinct, unique stable identifier to the new presentation so both presentations exist and route independently without collision

### Requirement: Open Presentation and Handle Route Staleness

The system MUST open an existing presentation by its stable identifier, display its ordered show items, and handle requests for missing or deleted presentation identifiers gracefully without application failure.

#### Scenario: Open an existing presentation
- GIVEN a presentation with identifier "pres-123" exists in storage
- WHEN the user opens the presentation from the catalog or navigates to its URL
- THEN the system MUST load presentation "pres-123", update the view to the presentation detail, and display its show items

#### Scenario: Access non-existent or deleted presentation identifier
- GIVEN a presentation identifier "pres-unknown" that does not exist in storage (e.g., stale link or deleted from another session)
- WHEN the user navigates to that presentation route
- THEN the system MUST NOT fail with an unhandled error, MUST inform the user that the requested presentation was not found, and MUST safely return the user to the presentation list

### Requirement: Rename Presentation

The system MUST allow updating the visible title of an existing presentation without changing its identifier, items, or item positions, and MUST reject empty or whitespace-only names.

#### Scenario: Successfully rename an existing presentation
- GIVEN an existing presentation with identifier "pres-123", title "Setlist Borrador", and three existing items
- WHEN the user renames the presentation to "Setlist Final"
- THEN the system MUST update the visible title to "Setlist Final", MUST keep identifier "pres-123", and MUST leave all three items and their positions unchanged

#### Scenario: Reject empty or whitespace-only title when renaming
- GIVEN an existing presentation with title "Show Acústico"
- WHEN the user attempts to rename the presentation with an empty string or whitespace only
- THEN the system MUST reject the update, retain the original title "Show Acústico", and present a validation error

### Requirement: Delete Presentation with Confirmation and Cascade Cleanup

The system MUST require explicit user confirmation before deleting a presentation, and upon confirmation MUST atomically delete the presentation record, cascade delete all associated show items, remove all referenced audio files from disk, and stop any active audio playback.

#### Scenario: Cancel presentation deletion
- GIVEN an existing presentation in the catalog
- WHEN the user initiates the delete action but cancels the confirmation prompt
- THEN the system MUST NOT delete the presentation, its items, or its audio files, and MUST keep all data intact

#### Scenario: Confirm presentation deletion with cascade audio file cleanup
- GIVEN an existing presentation containing multiple items, including track items with stored audio files on disk
- WHEN the user confirms deletion of the presentation
- THEN the system MUST delete the presentation record and all its item records from the database, MUST delete all associated audio files from disk storage, and MUST return the user to the presentation list

#### Scenario: Delete currently active presentation during audio playback
- GIVEN the user is viewing presentation "pres-123" while an audio track from that presentation is actively playing
- WHEN the user confirms deletion of presentation "pres-123"
- THEN the system MUST immediately stop the audio playback, release audio resources, execute the cascade deletion of database records and audio files, and navigate to the presentation list
