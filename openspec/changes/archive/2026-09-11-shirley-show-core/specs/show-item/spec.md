# Show Item Specification

## Purpose

Defines the vertical sequence, item types (track, lyrics, note), item creation, ordering, audio streaming, audio playback constraints, and deletion invariants for elements within a presentation in Shirley Show.

## Requirements

### Requirement: Single Vertical Sequence and Empty State

Each presentation MUST maintain exactly one ordered vertical sequence of items with contiguous positions, and MUST display an actionable empty state when no items exist.

#### Scenario: Display ordered items in presentation sequence
- GIVEN a presentation containing track, lyrics, and note items with assigned contiguous positions
- WHEN the user opens the presentation view
- THEN the system MUST render all items in a single vertical list strictly ordered by their position indices from top to bottom

#### Scenario: Empty presentation sequence state
- GIVEN a presentation that contains zero items
- WHEN the user opens the presentation view
- THEN the system MUST display an empty state explaining that the show has no items yet and MUST prominently display actions to add a track, lyrics, or note

### Requirement: Add Item with Type Selector

The system MUST provide an explicit type selector allowing the user to choose between Track, Lyrics, and Note before creating an item, and MUST append any successfully created item to the end of the presentation sequence.

#### Scenario: Select item type to add
- GIVEN an open presentation view
- WHEN the user initiates the add item flow
- THEN the system MUST present distinct options to add a "Pista" (Track), "Letra" (Lyrics), or "Nota" (Note)

#### Scenario: Newly created item appended to end of sequence
- GIVEN a presentation containing two existing items at positions 0 and 1
- WHEN the user creates a new valid item (track, lyrics, or note)
- THEN the system MUST assign position 2 to the new item, persist it, and display it at the bottom of the sequence

### Requirement: Track Item Creation and Audio Upload

The system MUST require an audio file and a visible track name to create a track item, MUST store the audio under a system-generated safe filename on disk, and MUST NOT create partial or orphaned records upon upload failure.

#### Scenario: Successful track upload and item creation
- GIVEN a supported audio file under 100 MiB and a track name "Intro Acústica"
- WHEN the user submits the track upload
- THEN the system MUST save the audio file in the managed local audio directory with a generated safe filename, persist an item with type `track`, title "Intro Acústica", and relative audio reference, and append it to the presentation sequence

#### Scenario: Reject track creation with empty or whitespace-only name
- GIVEN a valid audio file but an empty or whitespace-only track name
- WHEN the user attempts to submit the track
- THEN the system MUST reject the request, MUST NOT save the audio file or create a database item, and MUST display a validation error requiring a track name

#### Scenario: Clean up and roll back on failed or interrupted upload
- GIVEN an audio upload interrupted before completion or aborted by a filesystem write failure
- WHEN the upload failure occurs
- THEN the system MUST delete any partially written temporary audio file from disk, MUST NOT persist an item in the database, and MUST display an actionable error message to the user

### Requirement: Audio Format and Size Limits

The system MUST restrict audio uploads to `.mp3`, `.wav`, and `.m4a` file formats, and MUST enforce an explicit maximum file size limit of 100 MiB per upload.

#### Scenario: Reject audio file exceeding 100 MiB limit
- GIVEN an audio file with a size greater than 100 MiB (104,857,600 bytes)
- WHEN the user attempts to upload the file as a track
- THEN the system MUST reject the upload with an explicit error stating that files cannot exceed 100 MiB, and MUST NOT save the file or create a database record

#### Scenario: Reject unsupported file format or extension
- GIVEN a file with an unsupported format or extension (such as `.flac`, `.ogg`, `.mp4`, or `.pdf`)
- WHEN the user attempts to upload the file as a track
- THEN the system MUST reject the upload with an explicit error indicating that only `.mp3`, `.wav`, and `.m4a` formats are supported

### Requirement: Audio Streaming and Mobile Range Requests

The backend HTTP server MUST support streaming audio with `Accept-Ranges: bytes`, return `206 Partial Content` for valid `Range` requests, and handle invalid or out-of-bounds ranges in accordance with RFC 9110 to ensure mobile browser seeking works reliably.

#### Scenario: Streaming audio with valid byte range request
- GIVEN a stored track audio file with a total length of 10,000,000 bytes
- WHEN a client requests the audio resource with header `Range: bytes=0-1023`
- THEN the server MUST return HTTP status `206 Partial Content` with header `Content-Range: bytes 0-1023/10000000`, header `Content-Length: 1024`, and the first 1024 bytes of the audio file

#### Scenario: Full audio request without Range header
- GIVEN a stored track audio file
- WHEN a client requests the audio resource without a `Range` header
- THEN the server MUST return HTTP status `200 OK` with header `Accept-Ranges: bytes` and `Content-Length` matching the total file size

#### Scenario: Range request outside file bounds
- GIVEN a stored track audio file with a total length of 5,000,000 bytes
- WHEN a client requests `Range: bytes=6000000-7000000`
- THEN the server MUST return HTTP status `416 Range Not Satisfiable` with header `Content-Range: bytes */5000000`

### Requirement: Exclusive Single Audio Playback and Stage Controls

The system MUST guarantee that at most ONE audio track plays at any given time across the entire application, MUST provide accessible touch controls with a minimum target size of 44 × 44 px, and MUST NEVER initiate playback automatically without user interaction.

#### Scenario: Play track when no other audio is active
- GIVEN a track item rendered in the show view and no audio currently playing
- WHEN the user taps the play button (touch target >= 44 × 44 px)
- THEN the audio track begins playing, the button transitions to the pause state, and playback time and total duration update continuously

#### Scenario: Start playback of a track while another track is playing
- GIVEN Track A is currently playing audio in the presentation
- WHEN the user taps the play button on Track B
- THEN the system MUST immediately pause and reset Track A, start playback of Track B, and ensure that only Track B is audible

#### Scenario: Pause an actively playing track
- GIVEN Track A is currently playing audio
- WHEN the user taps the pause button on Track A
- THEN playback pauses at the current elapsed time and the button transitions back to the play state

#### Scenario: Initial view render does not autoplay
- GIVEN a presentation containing one or more track items
- WHEN the user opens the presentation view
- THEN all tracks MUST remain stopped and no audio MUST play until a play button is explicitly activated by the user

### Requirement: Audio Interruption on Navigation or Missing Media

The system MUST immediately stop audio playback when the user navigates away from the presentation view, and MUST handle missing audio files on disk gracefully without crashing the view.

#### Scenario: User navigates away from presentation during audio playback
- GIVEN a track is actively playing audio
- WHEN the user navigates back to the presentation catalog or closes the show view
- THEN the system MUST pause the audio playback and release the audio player resources

#### Scenario: Track audio file missing on disk
- GIVEN a track item in the database whose corresponding audio file is missing or unreadable on disk
- WHEN the user attempts to play the track
- THEN the system MUST NOT crash, MUST display a visible error indicating that the audio file is unavailable, and MUST leave the rest of the presentation fully operational

### Requirement: Lyrics Item Creation and Display

The system MUST allow creating a lyrics item with multiline text, MUST reject empty or whitespace-only content, and MUST render lyrics with large, high-contrast typography optimized for stage readability from a music stand.

#### Scenario: Create valid multiline lyrics item
- GIVEN multiline lyrics text containing verses and chorus
- WHEN the user confirms creation of the lyrics item
- THEN the system MUST persist the lyrics item in the sequence and render it with high-contrast, large-font typography suitable for stage reading

#### Scenario: Reject empty or whitespace-only lyrics
- GIVEN lyrics text consisting solely of whitespace or empty string
- WHEN the user attempts to save the lyrics item
- THEN the system MUST reject creation and display a validation error requiring non-empty lyrics text

### Requirement: Note Item Creation and Display

The system MUST allow creating a note item with short text, MUST reject empty or whitespace-only content, and MUST render notes with clear visual cues that distinguish them from lyrics and audio tracks.

#### Scenario: Create valid stage note item
- GIVEN short cue text "Presentar al guitarrista antes del solo acústico"
- WHEN the user confirms creation of the note item
- THEN the system MUST persist the note item and render it with distinctive visual styling (e.g., cue badge or colored border) clearly distinguished from lyrics and track items

#### Scenario: Reject empty or whitespace-only note
- GIVEN note text consisting solely of whitespace or empty string
- WHEN the user attempts to save the note item
- THEN the system MUST reject creation and display a validation error requiring non-empty note text

### Requirement: Atomic Item Reordering with Boundary Guards

The system MUST provide explicit "Subir" (Up) and "Bajar" (Down) controls to exchange adjacent items atomically within an isolated database transaction, MUST disable or ignore "Subir" for the first item in the sequence, and MUST disable or ignore "Bajar" for the last item in the sequence.

#### Scenario: Move item up in the sequence
- GIVEN Item A at position 0 and Item B at position 1
- WHEN the user activates "Subir" on Item B
- THEN the system MUST atomically swap positions so Item B is at position 0 and Item A is at position 1, preserving contiguous indices

#### Scenario: Move item down in the sequence
- GIVEN Item A at position 0 and Item B at position 1
- WHEN the user activates "Bajar" on Item A
- THEN the system MUST atomically swap positions so Item B is at position 0 and Item A is at position 1

#### Scenario: Guard first item against moving up
- GIVEN Item A at position 0 (the first item in the presentation sequence)
- WHEN the user inspects the controls for Item A
- THEN the "Subir" control MUST be disabled or absent, and any upward reorder request for Item A MUST be rejected without altering item positions

#### Scenario: Guard last item against moving down
- GIVEN Item C at position 2 in a 3-item sequence (the last item)
- WHEN the user inspects the controls for Item C
- THEN the "Bajar" control MUST be disabled or absent, and any downward reorder request for Item C MUST be rejected without altering item positions

#### Scenario: Rapid successive reordering preserves contiguous ordering
- GIVEN multiple reorder requests triggered in rapid succession
- WHEN each request is processed by the server
- THEN each swap MUST execute within an immediate database transaction (`BEGIN IMMEDIATE` / `COMMIT`) so that positions remain strictly contiguous integers `0..N-1` with no duplicates or gaps

### Requirement: Item Deletion and Resource Cleanup

The system MUST require explicit user confirmation before deleting an item, MUST stop audio playback if the deleted item is currently playing, MUST delete the item record, MUST delete any associated audio file from disk, and MUST recompact remaining item positions to maintain contiguous order.

#### Scenario: Cancel item deletion
- GIVEN an existing item in the presentation sequence
- WHEN the user triggers delete but cancels the confirmation prompt
- THEN the system MUST NOT delete the item and MUST retain its position and data intact

#### Scenario: Delete track item with active playback and stored file
- GIVEN a track item that is currently playing audio and has a corresponding audio file stored on disk
- WHEN the user confirms deletion of the track item
- THEN the system MUST immediately stop the audio playback, remove the item record from the database, delete the audio file from disk, and recompact subsequent item positions so the sequence remains contiguous `0..N-1`

#### Scenario: Delete lyrics or note item and recompact positions
- GIVEN a sequence of 3 items at positions 0, 1, and 2, where position 1 is a lyrics or note item
- WHEN the user confirms deletion of the item at position 1
- THEN the system MUST remove the item from the database, decrement the position of the item at position 2 to position 1, and update the view
