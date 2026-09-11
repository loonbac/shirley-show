# Agent Chat UI Specification

## Purpose

Defines the frontend conversational interface for Shirley, including the 580px slide-over desktop panel, fullscreen mobile overlay, golden typing indicator, audio attachment composer, keyboard shortcuts, active context badge, and reactive view synchronization in Shirley Show.

## Requirements

### Requirement: Large Slide-Over and Mobile Fullscreen Chat Surface

1. The system MUST provide a prominent chat toggle action in the primary navigation and header with a minimum touch target of at least 48 × 48 px.
2. On desktop viewports (screen width greater than 768 px):
   - The chat panel MUST render as a slide-over panel with a width of 580 px anchored to the right edge of the viewport.
   - The panel MUST feature a semi-transparent backdrop overlay over the main application content.
3. On mobile viewports (screen width 768 px or less):
   - The chat panel MUST render as a full-viewport overlay (100% width and height) covering the entire screen.
   - The panel MUST account for safe-area insets (e.g., notch and home bar).
4. The chat surface MUST provide an always-visible, accessible close button with a touch target of at least 48 × 48 px.
5. When opened, focus MUST automatically transfer to the message composer input, and when closed, focus MUST be returned to the element that triggered the panel.

#### Scenario: Open slide-over panel on desktop viewport
- GIVEN a desktop browser viewport width of 1280 px
- WHEN the user clicks the chat action button
- THEN the system MUST slide open a 580 px wide chat panel from the right edge with a backdrop overlay, and set focus to the message input field

#### Scenario: Open fullscreen panel on mobile viewport
- GIVEN a mobile browser viewport width of 390 px
- WHEN the user taps the chat action button
- THEN the system MUST open the chat interface as a full-screen overlay occupying 100% of the viewport width and height

#### Scenario: Close panel via backdrop click or close button restores previous focus
- GIVEN the chat panel is currently open and focus was previously on the header chat trigger
- WHEN the user clicks the backdrop overlay or taps the close button
- THEN the chat panel MUST close and focus MUST return directly to the header chat trigger

---

### Requirement: Active Repertoire Context Badge

1. The chat header MUST display an active context badge informing the user of the target presentation:
   - When the user is viewing a presentation detail: the badge MUST display the active presentation's name (e.g., "Contexto: Viernes 24 Acústico").
   - When the user is on the home catalog or no show is active: the badge MUST display an explanatory state (e.g., "Modo catálogo (creará un nuevo show)").
2. When Shirley creates a new presentation via tool execution during the conversation:
   - The context badge MUST update dynamically to display the newly created presentation's title.
   - Subsequent item addition instructions in the conversation MUST target the newly created presentation by default.

#### Scenario: Chat header displays active show context when opened from show detail
- GIVEN the user is viewing presentation "Domingo Íntimo"
- WHEN the user opens the chat panel
- THEN the chat header MUST display a badge showing "Contexto: Domingo Íntimo"

#### Scenario: Chat header displays catalog context when opened from home view
- GIVEN the user is viewing the presentation catalog view
- WHEN the user opens the chat panel
- THEN the chat header MUST display a badge indicating "Modo catálogo (creará un nuevo show)"

#### Scenario: Context badge updates dynamically when presentation is created by assistant
- GIVEN the chat panel open in catalog mode
- WHEN the user asks Shirley to create "Show Acústico Primavera" and the tool succeeds
- THEN the context badge in the chat header MUST immediately update to reflect "Contexto: Show Acústico Primavera"

---

### Requirement: Conversational Message Rendering and Animation

1. Messages in the conversation transcript MUST be visually distinguished by role:
   - User messages MUST be right-aligned with distinct user bubble styling.
   - Shirley assistant messages MUST be left-aligned with a warm golden/amber accent, avatar icon, and readable typography.
2. Assistant messages MUST be rendered safely:
   - Multiline text, verses, and paragraphs MUST be preserved with appropriate whitespace.
   - Model text MUST be sanitized text; raw unescaped HTML tags or script injection MUST NOT be rendered as active DOM elements.
3. Message bubbles MUST animate smoothly into view with subtle opacity and vertical translation.
4. When the user agent indicates `prefers-reduced-motion: reduce`:
   - Sliding and bouncing animations MUST be disabled.
   - Messages and panel transitions MUST appear instantly or with a simple opacity fade.
5. The message list MUST automatically scroll to the bottom when a new message is added, unless the user has actively scrolled up to inspect previous turns.

#### Scenario: Render user and assistant message bubbles with visual distinction
- GIVEN a conversation transcript with one user turn and one assistant turn
- WHEN the message list renders
- THEN the user message MUST appear right-aligned with user styling and the assistant message MUST appear left-aligned with warm gold accents and assistant avatar

#### Scenario: Sanitize assistant message text preventing script injection
- GIVEN an assistant response containing raw HTML or script tags such as `<script>alert('xss')</script>` or `<img src=x onerror=alert(1)>`
- WHEN the message is rendered in the transcript
- THEN the system MUST sanitize or escape the content as plain text, rendering the characters harmlessly without executing script

#### Scenario: Smooth message entry respects prefers-reduced-motion
- GIVEN a browser where the user has enabled `prefers-reduced-motion: reduce`
- WHEN a new message is appended to the transcript
- THEN the message MUST appear immediately without sliding or bouncy transform animations

---

### Requirement: Golden Typing and Pending Feedback

1. While an HTTP request to `/api/agent/chat` is in flight:
   - The chat transcript MUST display a warm golden typing indicator featuring animated pulsing dots in Shirley's signature amber/gold palette.
   - The typing indicator MUST display the text label "Shirley está pensando...".
   - An accessible ARIA live region (`aria-live="polite"`) MUST announce the pending state to assistive technologies.
2. While a request is in flight:
   - The send button MUST be disabled to prevent duplicate concurrent submissions.
   - The message input and attachment controls MUST be disabled or visually guarded against conflicting edits.
3. Upon receiving the server response or an error:
   - The golden typing indicator MUST be removed from the transcript.
   - The send button MUST be re-enabled.

#### Scenario: Golden typing indicator displayed while response is pending
- GIVEN the user sends a message to Shirley
- WHEN the request is awaiting completion from the server
- THEN the transcript MUST display the pulsing golden dots indicator with label "Shirley está pensando..." and announce pending status to screen readers

#### Scenario: Send button disabled during active request to prevent duplicate submission
- GIVEN a request currently awaiting response
- WHEN the user attempts to click the send button or press Enter
- THEN the send button MUST be disabled, and no secondary HTTP request MUST be dispatched

---

### Requirement: Audio Attachment Composer and Staging Chip

1. The composer MUST provide a dedicated attachment action button with a touch target of at least 48 × 48 px, styled with an audio/paperclip icon.
2. Activating the attachment button MUST trigger a file picker restricted to audio MIME types and extensions: `.mp3`, `.wav`, and `.m4a`.
3. When an audio file is selected:
   - The composer MUST render an attachment chip immediately above the text input displaying the file name, formatted size (e.g. "5.4 MB"), and a distinct remove button.
   - The system MUST restrict attachments to at most one audio file per turn. Selecting another file MUST replace the current attachment.
4. Client-side size guard:
   - If the selected file exceeds 100 MiB (104,857,600 bytes), the composer MUST NOT accept the file, MUST display an immediate inline warning that audio files cannot exceed 100 MiB, and MUST NOT allow submitting the turn.
5. Activating the remove button on the chip MUST clear the file and return the composer to a text-only turn.
6. After a successful turn that consumed the attachment, the attachment chip MUST be cleared from the composer.

#### Scenario: Attach valid audio file displays attachment chip
- GIVEN the composer in an empty state
- WHEN the user selects an audio file "pista_voz.mp3" of size 8.2 MB
- THEN the composer MUST display an attachment chip showing "pista_voz.mp3", "8.2 MB", and an accessible remove button

#### Scenario: Remove attached audio clears attachment chip
- GIVEN an attachment chip for "pista_voz.mp3" displayed in the composer
- WHEN the user activates the remove button on the chip
- THEN the attachment chip MUST be removed and the composer state reset to text-only

#### Scenario: Client rejects audio file exceeding 100 MiB limit immediately
- GIVEN a user selecting an audio file of 110 MiB in the file picker
- WHEN the file is chosen
- THEN the composer MUST reject the file, display a clear inline message stating that files cannot exceed 100 MiB, and MUST NOT attach the file or allow sending it

---

### Requirement: Keyboard Navigation and Shortcuts

1. The application MUST support global keyboard shortcut `Ctrl+K` (and `Cmd+K` on macOS) to toggle the chat panel open or closed from any view (home catalog, presentation detail, Stage Mode excluded).
2. While the chat panel is open:
   - Pressing `Escape` MUST close the chat panel and return keyboard focus to the triggering button.
   - Pressing `Enter` in the message textarea MUST submit the message (provided the send button is enabled).
   - Pressing `Shift+Enter` in the message textarea MUST insert a newline without submitting.
3. In Stage Mode:
   - The chat panel MUST NOT open, and `Ctrl+K` / `Cmd+K` MUST be suppressed to preserve the distraction-free performance surface.

#### Scenario: Toggle chat panel using Ctrl+K keyboard shortcut
- GIVEN the user is on the presentation catalog view and the chat panel is closed
- WHEN the user presses `Ctrl+K` (or `Cmd+K` on macOS)
- THEN the chat panel MUST slide open and focus MUST be set on the message textarea

#### Scenario: Close chat panel using Escape key and restore focus
- GIVEN the chat panel is open and focused
- WHEN the user presses the `Escape` key
- THEN the chat panel MUST close immediately and focus MUST return to the previous trigger element

#### Scenario: Submit message with Enter and multiline with Shift+Enter
- GIVEN text typed into the composer textarea
- WHEN the user presses `Enter`
- THEN the message MUST be submitted; and WHEN the user presses `Shift+Enter`, a newline MUST be inserted into the textarea without submitting

#### Scenario: Suppress chat shortcut during Stage Mode
- GIVEN the user is performing in Stage Mode
- WHEN the user presses `Ctrl+K`
- THEN the chat panel MUST NOT open and the Stage Mode performance surface MUST remain undisturbed

---

### Requirement: Reactive UI Refresh After Confirmed Actions

1. When the chat receives a response with non-empty `actions_taken`:
   - For each action in `actions_taken`, the assistant message card MUST render a visual confirmation chip indicating the executed tool and affected item or presentation.
   - If any action created a presentation (`create_presentation`):
     - The application MUST emit a catalog refresh signal so the home library updates immediately.
   - If any action added show items (`add_lyrics`, `add_note`, `add_track_from_staged`):
     - If the affected presentation is currently open in the show detail view, the application MUST reload the item sequence reactively, rendering the new item at the bottom of the list without a page refresh.
2. If `active_presentation_id` changes in the response, the active context badge MUST update to match.

#### Scenario: Reactive catalog refresh after assistant creates presentation
- GIVEN the user is on the home library view and asks Shirley to create a show
- WHEN Shirley confirms creation with an action in `actions_taken`
- THEN the home library MUST refresh reactively and display the new presentation card in the catalog

#### Scenario: Reactive sequence reload after assistant adds lyrics, note, or track
- GIVEN the user has presentation "pres-1" open in the preparation view
- WHEN Shirley adds a lyrics item via `add_lyrics`
- THEN the show view sequence MUST reactively append the new lyrics block to the vertical sequence without losing the user's scroll position

#### Scenario: Render confirmation badges for executed tool actions
- GIVEN Shirley's response contains two successful actions: one `create_presentation` and one `add_note`
- WHEN the assistant message renders
- THEN the message bubble MUST include two distinctive visual confirmation badges summarizing the completed actions

---

### Requirement: Ephemeral Chat State and Safe Draft Recovery

1. Conversation history MUST be maintained strictly as in-memory ephemeral state in the browser session.
2. Navigating between views within the single-page application MUST preserve conversation history.
3. Reloading the page or closing the browser tab MUST clear conversation history.
4. If a turn fails due to network error, server error, or provider outage:
   - The user's typed message text MUST remain in the composer input.
   - If an audio file was attached and was not consumed, the attachment chip MUST remain attached in the composer.
   - An actionable error message with a retry option MUST be displayed in the transcript.
5. If the chat panel is closed or dismantled while a request is in flight:
   - A subsequent late server response MUST NOT cause application errors, duplicate mutations, or unwanted audio triggers.

#### Scenario: Ephemeral conversation survives view navigation in same page session
- GIVEN an active chat conversation with three turns
- WHEN the user closes the chat panel, navigates to another presentation, and re-opens the chat panel
- THEN the previous three conversation turns MUST still be present in the transcript

#### Scenario: Page reload clears ephemeral chat history cleanly
- GIVEN an active chat conversation
- WHEN the user reloads the browser page (`F5` or refresh)
- THEN the chat state MUST reset to its initial clean state with no previous messages restored

#### Scenario: Failed request retains draft text and attachment for retry
- GIVEN the user attached "demo.mp3" and typed "Añade esta pista" and the request failed due to a network drop
- WHEN the error occurs
- THEN the composer MUST retain "Añade esta pista" and the "demo.mp3" attachment chip so the user can re-send without re-selecting

#### Scenario: Ignore stale response after component teardown
- GIVEN an active request in flight
- WHEN the user navigates away or closes the browser panel before the response returns
- THEN when the response eventually arrives, the system MUST cleanly discard the payload without attempting to update dismantled DOM elements
