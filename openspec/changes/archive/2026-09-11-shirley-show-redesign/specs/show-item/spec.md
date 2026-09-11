# Delta for Show Item

## MODIFIED Requirements

### Requirement: Add Item with Type Selector

1. The system MUST provide an immediate quick-add bar with three distinct one-tap actions for Track ("+ Pista"), Lyrics ("+ Letra"), and Note ("+ Nota") accessible on the presentation detail view without requiring multi-step modal tab navigation.
2. Activating "+ Pista" MUST open the audio track upload flow, activating "+ Letra" MUST open the lyrics entry flow, and activating "+ Nota" MUST open the stage note entry flow.
3. All primary quick-add buttons MUST maintain a touch target size of at least 48 px.
4. Any successfully created item of any type MUST be appended to the end of the presentation sequence with contiguous position index N.
(Previously: Required opening an add-item dialog and navigating type tabs before entering content.)

#### Scenario: Select item type to add via quick-add bar
- GIVEN an open presentation view
- WHEN the user views the sequence controls
- THEN the system MUST display three distinct immediate touch buttons: "+ Pista", "+ Letra", and "+ Nota" with minimum touch targets of at least 48 px

#### Scenario: Newly created item appended to end of sequence
- GIVEN a presentation containing two existing items at positions 0 and 1
- WHEN the user creates a new valid item (track, lyrics, or note) via the quick-add bar
- THEN the system MUST assign position 2 to the new item, persist it in storage, and append it to the bottom of the presentation sequence

### Requirement: Atomic Item Reordering with Boundary Guards

1. The system MUST provide a dedicated touch and pointer drag handle on each item block with CSS "touch-action: none" to support direct drag-and-drop reordering.
2. Non-handle areas of each item block MUST preserve native vertical page scrolling without gesture conflict.
3. The backend reorder endpoint MUST optionally accept a "target_position" parameter to move an item to any target position within the sequence, executing the shift and recompaction inside a single immediate database transaction ("BEGIN IMMEDIATE; ... COMMIT;").
4. The system MUST retain accessible "Subir" (Up) and "Bajar" (Down) button controls as keyboard and assistive-technology fallbacks to exchange adjacent items atomically.
5. The system MUST disable or ignore upward reorder for the first item in the sequence, and MUST disable or ignore downward reorder for the last item in the sequence.
6. If a drag interaction is cancelled via "pointercancel", released outside a valid drop target, or if the reorder request fails on the server, the system MUST restore the server-confirmed sequence order and MUST NOT leave duplicate or missing position indices.
(Previously: Supported reordering only via adjacent Up and Down swap buttons without touch drag-and-drop or target_position support.)

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
- THEN each swap MUST execute within an immediate database transaction ("BEGIN IMMEDIATE; ... COMMIT;") so that positions remain strictly contiguous integers 0..N-1 with no duplicates or gaps

#### Scenario: Reorder item via dedicated drag handle and target position
- GIVEN a presentation containing items at positions 0, 1, and 2
- WHEN the user drags the dedicated handle of the item at position 2 and drops it at position 0
- THEN the system MUST submit a move request with target position 0, atomically shift intervening items to positions 1 and 2, and render the updated sequence

#### Scenario: Cancelled pointer drag restores original sequence
- GIVEN a user initiating a drag gesture from an item's handle
- WHEN the gesture is interrupted by a "pointercancel" event or released outside the presentation sequence list
- THEN the system MUST cancel the reorder operation, restore the original visual order immediately, and MUST NOT send a mutation request to the server

#### Scenario: Touch interaction outside drag handle preserves native scrolling
- GIVEN an item block in the presentation detail view
- WHEN the user performs a vertical swipe gesture on any area of the item card outside the dedicated drag handle
- THEN the browser MUST execute native vertical page scrolling without initiating drag reordering or suppressing momentum

#### Scenario: Server reorder failure restores confirmed sequence
- GIVEN an item dragged to a new position
- WHEN the server reorder request fails due to a network or database error
- THEN the system MUST revert the UI to the last confirmed server order, display an actionable error notification, and keep positions contiguous

## ADDED Requirements

### Requirement: Item Content Quick Editing

1. The backend HTTP server MUST provide "PATCH /api/presentations/{presentation_id}/items/{item_id}" to update the title of an existing track item or the text body of an existing lyrics or note item without requiring audio re-upload or item recreation.
2. The update request MUST trim whitespace and reject empty or whitespace-only values for title or body with HTTP status 400 Bad Request.
3. The update request MUST reject payloads containing fields incompatible with the item's kind (such as updating "body" on a track item or "title" on a lyrics or note item) with HTTP status 400 Bad Request.
4. Updating a non-existent item or an item within a non-existent presentation MUST return HTTP status 404 Not Found.
5. A successful item update MUST update the parent presentation's "updated_at" timestamp.
6. The frontend MUST provide a focused quick-edit interface that preloads the existing value, validates input before submission, and preserves the user's uncommitted draft with an actionable error message if the server update fails.

#### Scenario: Successfully update track title without re-uploading audio
- GIVEN an existing track item with title "Take 1" and stored audio file on disk
- WHEN the user updates the title to "Intro Acústica Final" via the quick-edit interface
- THEN the backend MUST update the title to "Intro Acústica Final", leave the audio file and sequence position intact, update the parent presentation's "updated_at" timestamp, and the UI MUST display the updated title

#### Scenario: Successfully update lyrics or note body
- GIVEN an existing lyrics or note item
- WHEN the user updates the text content via the quick-edit interface
- THEN the backend MUST update the text body in storage, update the parent presentation's "updated_at" timestamp, and the UI MUST display the updated text

#### Scenario: Reject empty or whitespace-only item edit (PATCH vacío)
- GIVEN an existing item in a presentation
- WHEN a client sends a PATCH request with an empty string or whitespace-only value for title or body
- THEN the server MUST return HTTP status 400 Bad Request, MUST NOT alter the stored item, and the quick-edit interface MUST display a validation error while preserving the user's draft

#### Scenario: Reject update for non-existent item or presentation (PATCH inexistente)
- GIVEN an item identifier or presentation identifier that does not exist in storage
- WHEN a client sends a PATCH request to update that item
- THEN the server MUST return HTTP status 404 Not Found, and the UI MUST inform the user that the item could not be found without creating a replacement item

#### Scenario: Reject field incompatible with item kind
- GIVEN an existing track item
- WHEN a client sends a PATCH request containing a "body" field
- THEN the server MUST return HTTP status 400 Bad Request and leave the stored track item unchanged

#### Scenario: Failed item update preserves draft and displays actionable error
- GIVEN an open quick-edit dialog with edited content
- WHEN the save request fails due to a server or connection error
- THEN the dialog MUST remain open, keep the user's edited draft intact, and display an actionable error message

### Requirement: Tactile Sequence Blocks and Quick Edit Interface

1. The presentation detail view MUST render track, lyrics, and note items as tactile blocks with touch targets of at least 48 px.
2. Each block MUST feature distinctive visual color accents according to its type: violet for tracks, deep-cyan for lyrics, and warm-amber for notes.
3. Each item block MUST provide a quick-edit action that opens a lightweight quick-edit dialog pre-filled with the item's current text.
4. The quick-edit interface MUST autofocus the input field, support confirmation via keyboard shortcut (Enter for titles, Cmd/Ctrl+Enter for multiline text), and support cancellation without altering the item.

#### Scenario: Render tactile item blocks with minimum 48px touch targets and type accents
- GIVEN a presentation containing track, lyrics, and note items
- WHEN the user views the presentation sequence
- THEN each item MUST be rendered with a minimum touch target size of 48 px and the corresponding type accent (violet for tracks, deep-cyan for lyrics, warm-amber for notes)

#### Scenario: Open quick-edit dialog prefilled with current content
- GIVEN an item in the presentation sequence
- WHEN the user activates the quick-edit action on that item
- THEN the system MUST open a focused quick-edit dialog prefilled with the item's existing title or body text and set focus to the input element

#### Scenario: Save quick edit via keyboard shortcut
- GIVEN the quick-edit dialog open with modified text
- WHEN the user presses Enter (for track title) or Cmd/Ctrl+Enter (for lyrics or note body)
- THEN the system MUST submit the update request and close the dialog upon success

#### Scenario: Cancel quick-edit dialog preserves existing content
- GIVEN the quick-edit dialog open with unsaved changes
- WHEN the user activates the cancel button or presses Escape
- THEN the dialog MUST close immediately without submitting any mutation and the original item content MUST remain unchanged

### Requirement: Stage Mode Distraction-Free Performance Surface

1. The presentation detail view MUST provide a dedicated action to enter Stage Mode for live performance.
2. Stage Mode MUST render as a full-viewport, distraction-free overlay containing only performance-critical elements: track playback controls, stage-scaled lyrics, and high-visibility stage notes.
3. Stage Mode MUST completely omit all editing, creation, deletion, and reordering controls to eliminate the risk of accidental mutations during live performance.
4. Lyrics in Stage Mode MUST render in large display typography and MUST provide accessible "A-" and "A+" font scaling controls to increase or decrease font size from a music stand without obscuring navigation or controls.
5. Notes in Stage Mode MUST render with high-visibility warm-amber background and borders for immediate stage cue recognition.
6. Track items in Stage Mode MUST provide large, high-contrast one-tap play/pause buttons with touch targets of at least 48 × 48 px.
7. Stage Mode MUST provide an always-visible, easily reachable exit action to return to the presentation preparation view.

#### Scenario: Enter Stage Mode with distraction-free layout
- GIVEN an open presentation with show items
- WHEN the user activates Stage Mode
- THEN the system MUST display the full-viewport stage overlay with enlarged lyrics, high-visibility notes, and large track controls

#### Scenario: Conceal all mutation and reordering controls in Stage Mode
- GIVEN the user is viewing a presentation in Stage Mode
- WHEN inspecting the interface
- THEN all edit buttons, delete actions, drag handles, "Subir" and "Bajar" buttons, and the quick-add bar MUST be absent from the display

#### Scenario: Adjust lyrics typography scale using A- and A+ controls
- GIVEN lyrics displayed in Stage Mode
- WHEN the user activates the "A+" control
- THEN the lyrics font size MUST increase progressively while keeping exit and navigation controls accessible and visible

#### Scenario: Display stage notes with high-visibility amber styling
- GIVEN a note item in the sequence during Stage Mode
- WHEN rendered on the stage surface
- THEN the note MUST display with high-contrast warm-amber background and styling clearly distinct from lyrics and tracks

#### Scenario: Large one-tap track playback in Stage Mode
- GIVEN a track item rendered in Stage Mode
- WHEN the user taps the large play button (touch target >= 48 × 48 px)
- THEN playback MUST begin immediately and the button MUST transition to the pause state

#### Scenario: Open Stage Mode for presentation with zero items
- GIVEN a presentation that contains zero items
- WHEN the user activates Stage Mode
- THEN the stage overlay MUST display an explanatory message indicating that the show has no items yet, provide an exit action, and MUST NOT fail with an error

#### Scenario: Exit Stage Mode returns to preparation view
- GIVEN an active Stage Mode view
- WHEN the user activates the exit action
- THEN the system MUST close the stage overlay and return the user to the presentation preparation view with all editing controls restored

### Requirement: Stage Mode Audio Continuity Across View Transitions

1. Entering or exiting Stage Mode within the same presentation MUST NOT stop, reset, restart, or duplicate currently active audio playback.
2. Stage Mode MUST share the application's singleton audio controller and reactive playback state with the preparation view.
3. Starting playback of a track within Stage Mode while another track is playing MUST enforce exclusive single playback in accordance with canonical requirements.
4. Navigating away from the presentation (such as returning to the home library) MUST stop audio playback and release audio resources.

#### Scenario: Enter Stage Mode during active playback without interruption
- GIVEN an audio track is actively playing in the presentation preparation view
- WHEN the user activates Stage Mode
- THEN the backing track MUST continue playing without pause, reset, or glitch, and the Stage Mode track control MUST reflect the active playing state and current elapsed time

#### Scenario: Exit Stage Mode during active playback without interruption
- GIVEN an audio track is actively playing in Stage Mode
- WHEN the user exits Stage Mode back to the preparation view
- THEN the backing track MUST continue playing uninterrupted and the preparation view control MUST reflect the active playing state

#### Scenario: Exclusive playback enforced within Stage Mode
- GIVEN Track A is actively playing in Stage Mode
- WHEN the user taps play on Track B in Stage Mode
- THEN the system MUST immediately stop Track A, begin playing Track B, and ensure only Track B is audible

#### Scenario: Navigate away from presentation stops active playback
- GIVEN an audio track is playing in Stage Mode or preparation view
- WHEN the user navigates away from the presentation to the home library
- THEN the system MUST immediately stop audio playback and release audio resources

### Requirement: Stage Mode Progressive Fullscreen and Screen Wake Lock

1. When entering Stage Mode, the system MUST progressively attempt to request browser fullscreen mode via "requestFullscreen()" when available and permitted by the user agent.
2. When entering Stage Mode, the system MUST progressively attempt to acquire a Screen Wake Lock via "navigator.wakeLock.request('screen')" to prevent the device display from dimming or sleeping while mounted on a music stand.
3. If fullscreen access is denied by the user or unsupported by the browser, Stage Mode MUST continue functioning normally as a fixed full-viewport overlay without displaying alarming error messages.
4. If Screen Wake Lock is unsupported by the browser or denied by system permissions, Stage Mode MUST continue functioning normally without blocking Shirley or repeatedly prompting for permission.
5. When the user exits Stage Mode, the system MUST release the Screen Wake Lock and exit browser fullscreen.
6. If the browser tab becomes hidden or backgrounded while Stage Mode is active, the system MUST handle automatic wake lock release and attempt to reacquire the wake lock when the tab becomes visible again while Stage Mode remains active.

#### Scenario: Acquire fullscreen and wake lock on entering Stage Mode
- GIVEN a browser environment supporting fullscreen and Screen Wake Lock APIs
- WHEN the user enters Stage Mode
- THEN the system MUST request browser fullscreen, acquire the screen wake lock, and display the performance surface

#### Scenario: Fullscreen permission denied or unsupported fallback (fullscreen denegado)
- GIVEN a browser where "requestFullscreen()" is rejected, denied by permission policy, or unsupported
- WHEN the user enters Stage Mode
- THEN the system MUST NOT crash or show an error modal, and MUST render Stage Mode as a fixed full-viewport overlay covering the browser window

#### Scenario: Screen Wake Lock unsupported or denied fallback (wake-lock no soportado)
- GIVEN a browser or operating system where the Screen Wake Lock API is unsupported or denied
- WHEN the user enters Stage Mode
- THEN the system MUST continue rendering Stage Mode normally, MUST NOT interrupt the performance with modal warnings, and MAY display a subtle status indicator

#### Scenario: Release wake lock and fullscreen on exiting Stage Mode
- GIVEN Stage Mode is active with acquired wake lock and active fullscreen
- WHEN the user exits Stage Mode
- THEN the system MUST release the Screen Wake Lock and exit browser fullscreen

#### Scenario: Tab visibility change reacquires wake lock while in Stage Mode
- GIVEN Stage Mode is active with an acquired Screen Wake Lock
- WHEN the user switches tabs causing the document to become hidden and the browser releases the wake lock
- THEN upon the user returning to the tab while Stage Mode remains active, the system MUST attempt to reacquire the Screen Wake Lock automatically
