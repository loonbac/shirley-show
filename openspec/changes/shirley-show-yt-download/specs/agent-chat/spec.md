# Delta for Agent Chat

## ADDED Requirements

### Requirement: Ephemeral Staged Audio Registry with TTL Lifecycle

The backend HTTP server MUST maintain an in-memory `StagedAudioRegistry` to manage the lifecycle of temporary downloaded audio tracks across conversational turns with strict time-to-live and disk cleanup guarantees.

1. Registry storage and entry structure:
   - The registry MUST store temporary audio entries resulting from `download_youtube_track` executions in memory.
   - Each registry entry MUST contain:
     - `staged_audio_id`: an opaque, unique string identifier (e.g. UUID v4).
     - `file_path`: absolute path to the validated MP3 file in a dedicated staging directory.
     - `title`: sanitized track title derived from input or video metadata.
     - `duration_secs`: content duration in seconds (up to 1200 seconds).
     - `size_bytes`: file size in bytes (up to 104,857,600 bytes).
     - `created_at`: monotonic or system timestamp of registration.
     - `consumed`: boolean flag indicating whether the audio has been moved to permanent storage.
2. Time-to-live (TTL) and disk eviction:
   - Each entry MUST have a TTL of exactly 15 minutes (900 seconds) from registration.
   - Upon TTL expiration, the registry MUST invalidate the entry and the server MUST delete the associated temporary MP3 file from disk immediately.
   - Expired entries MUST NOT be retrievable or consumable by `add_track_from_staged` or any other operation.
3. Single-use consumption guarantee:
   - A valid staging reference MUST be consumable at most once.
   - When successfully consumed by `add_track_from_staged`, `consumed` MUST be set to true and the file moved out of the staging directory.
   - Any subsequent call attempting to reuse the same `staged_audio_id` MUST be rejected with a structured error `{ "error": "audio_already_consumed", "message": "El audio preparado ya fue consumido" }`.
4. Staging preservation on failed mutation:
   - If an operation attempting to add the staged audio to a presentation fails (for example, if the specified presentation does not exist), the registry entry MUST NOT be marked as consumed, allowing the user to select another presentation while the 15-minute TTL remains active.
5. Server restart and orphan cleanup:
   - Staged audio references in memory are ephemeral and MUST NOT be persisted in SQLite.
   - Restarting the server invalidates all previous in-memory references.
   - On server startup, the server MUST scan the dedicated temporary staging directory and purge any orphaned files from previous runs without altering files in the managed audio directory.

#### Scenario: Register downloaded audio in registry with 15-minute TTL
- GIVEN a successful YouTube download of an 8 MiB MP3 track titled "Zamba de mi esperanza"
- WHEN the audio is registered in `StagedAudioRegistry`
- THEN the registry MUST generate an opaque `staged_audio_id`, record its file path and metadata, and set an active TTL of 15 minutes

#### Scenario: Consume staged audio once successfully and block second consumption
- GIVEN an active staging reference "staged-123" in `StagedAudioRegistry`
- WHEN `add_track_from_staged` consumes "staged-123" to add a track to presentation "pres-1"
- THEN the file MUST be moved to managed audio, "staged-123" marked as consumed, and a second call with "staged-123" MUST be rejected with `audio_already_consumed`

#### Scenario: Staged audio entry expires after 15 minutes and purges file from disk
- GIVEN an audio file staged in `StagedAudioRegistry` at time T
- WHEN 15 minutes elapse without the audio being consumed by a show presentation
- THEN the registry entry MUST be invalidated and the temporary MP3 file MUST be deleted from disk

#### Scenario: Failed track addition preserves staged audio reference for retry
- GIVEN an active staging entry "staged-456" and a target presentation that does not exist
- WHEN `add_track_from_staged` fails with `presentation_not_found`
- THEN "staged-456" MUST remain unconsumed and available in the registry for a subsequent request within its original 15-minute TTL

#### Scenario: Server restart clears memory registry and cleans up orphan staging files
- GIVEN two temporary download files residing in the staging directory when the server restarts
- WHEN the server finishes startup initialization
- THEN the in-memory registry MUST start empty and the temporary staging directory MUST be purged of any orphaned files from the previous process run

---

### Requirement: Conversational Download Progress and Client State Continuity

The system MUST provide qualitative progress feedback during YouTube downloads and preserve structured staging context across conversation turns without requiring streaming protocols or server-sent events.

1. Client typing indicator specialization:
   - When the user submits a chat message containing a recognized YouTube URL (matching `youtube.com` or `youtu.be`), the frontend chat client MUST display the specialized qualitative indicator text "descargando el audio…" instead of the generic "Shirley está pensando...".
   - The specialized indicator MUST retain Shirley's signature warm golden/amber pulsing animation and `aria-live="polite"` announcement.
   - For any message that does not contain a recognized YouTube URL, the client MUST continue displaying "Shirley está pensando...".
   - The indicator MUST NOT display simulated or fabricated percentages, download speeds, or estimated remaining times.
   - When the HTTP request completes (whether success, error, or timeout) or the chat panel is closed, the specialized indicator MUST be dismissed and the composer restored.
2. Structured staging continuity across turns:
   - When the server response to `POST /api/agent/chat` includes a non-null `staged_audio_id`, the client MUST store this identifier in ephemeral in-memory state.
   - On the user's immediately following chat request, the client MUST automatically include the stored `staged_audio_id` in the request payload.
   - Once a turn completes where `add_track_from_staged` was executed (confirmed in `actions_taken`) or if the user starts an explicit unrelated command, the client MUST clear the stored `staged_audio_id`.
   - The client MUST NOT store `staged_audio_id` in `localStorage` or persist it across page reloads; a browser refresh resets ephemeral chat state cleanly.

#### Scenario: User message with YouTube URL switches typing indicator to qualitative download status
- GIVEN the chat composer is ready
- WHEN the user submits "Por favor descarga https://www.youtube.com/watch?v=dQw4w9WgXcQ en pista"
- THEN the client MUST display the typing indicator with the text label "descargando el audio…" while the HTTP request remains in flight

#### Scenario: Non-download user message retains default thinking indicator
- GIVEN the user submits "Crea una presentación para el show del viernes"
- WHEN the request is in flight
- THEN the client MUST display the typing indicator with the standard text label "Shirley está pensando..."

#### Scenario: Client retains staged_audio_id from response and forwards it in next turn
- GIVEN the assistant returns HTTP 200 with `staged_audio_id: "staged-yt-789"` and message "El audio está listo. ¿A qué presentación deseas agregarlo?"
- WHEN the user replies "Agrégalo a la presentación Peña del Viernes"
- THEN the client MUST send `POST /api/agent/chat` containing `message: "Agrégalo a la presentación Peña del Viernes"` and `staged_audio_id: "staged-yt-789"`

#### Scenario: Client clears staged_audio_id after track is successfully added
- GIVEN the client has an active stored `staged_audio_id: "staged-yt-789"`
- WHEN the assistant completes a turn where `actions_taken` includes a successful `add_track_from_staged` action
- THEN the client MUST clear the stored `staged_audio_id` from ephemeral state

#### Scenario: Component teardown or error restores composer and clears active indicator
- GIVEN a request currently in flight showing "descargando el audio…"
- WHEN the server returns an error or the request times out
- THEN the typing indicator MUST be removed immediately, an actionable message displayed, and the composer input re-enabled

---

## MODIFIED Requirements

### Requirement: Agent Chat API Endpoint and Content Negotiation

1. The backend HTTP server MUST provide `POST /api/agent/chat` to process conversational requests from the client.
2. The endpoint MUST accept requests with `Content-Type: application/json` containing a JSON object with:
   - `message`: a required non-empty string with the user's conversational prompt.
   - `history`: an optional array of conversation turns `[{"role": "user" | "assistant", "content": string}]`.
   - `presentation_id`: an optional string with the active presentation identifier.
   - `staged_audio_id`: an optional string referencing an active staged audio entry from a prior download turn.
3. The endpoint MUST accept requests with `Content-Type: multipart/form-data` containing:
   - `message`: a string containing the user's prompt or instruction.
   - `history`: an optional JSON-encoded string or field containing previous conversation turns.
   - `presentation_id`: an optional string containing the active presentation identifier.
   - `staged_audio_id`: an optional string referencing an active staged audio entry from a prior download turn.
   - `file`: a single audio attachment file.
4. Requests with any other `Content-Type` MUST be rejected with HTTP status 415 Unsupported Media Type.
(Previously: Did not accept staged_audio_id in JSON or multipart request payload)

#### Scenario: Text-only JSON chat request
- GIVEN the user submits a text message "Crea una presentación para el show del sábado" with no attachments
- WHEN the client sends `POST /api/agent/chat` with `Content-Type: application/json` and body `{"message": "Crea una presentación para el show del sábado"}`
- THEN the server MUST process the request as a text-only turn and return HTTP status 200 with the assistant's reply

#### Scenario: Multipart request with audio attachment and instruction
- GIVEN a user attaching an audio file "acustico.mp3" under 100 MiB with text "Añade esta pista al show activo" and active presentation "pres-1"
- WHEN the client sends `POST /api/agent/chat` as `multipart/form-data` with fields `message`, `presentation_id`, and `file`
- THEN the server MUST stage the audio file locally, process the turn with the staged audio context, and return HTTP status 200

#### Scenario: Reject request with unsupported media type
- GIVEN an HTTP request sent to `POST /api/agent/chat` with `Content-Type: text/plain`
- WHEN the server receives the request
- THEN the server MUST return HTTP status 415 Unsupported Media Type and MUST NOT initiate any agent loop or provider call

#### Scenario: JSON chat request with staged_audio_id from previous download turn
- GIVEN a user replying "Añádelo al show Peña del Sábado" with an active staging reference "staged-yt-999"
- WHEN the client sends `POST /api/agent/chat` with `{"message": "Añádelo al show Peña del Sábado", "staged_audio_id": "staged-yt-999"}`
- THEN the server MUST accept `staged_audio_id` in the request context and make it available for `add_track_from_staged` execution

---

### Requirement: Response Contract and Action Reporting

1. A successful request to `POST /api/agent/chat` MUST return HTTP status 200 OK with a JSON response containing:
   - `message`: a non-empty string containing Shirley's conversational response.
   - `actions_taken`: an array of structured action descriptors for tools successfully executed during the turn.
   - `active_presentation_id`: an optional string containing the currently active presentation identifier (either preserved from request or updated by tool execution).
   - `staged_audio_id`: an optional string containing the opaque identifier of a staged audio file if audio was downloaded and staged but not yet added to a presentation; null or omitted otherwise.
2. Each item in `actions_taken` MUST contain:
   - `tool`: string naming the executed tool.
   - `presentation_id`: string with the affected presentation identifier.
   - `summary`: string with a human-readable description of the completed action.
   - `status`: string with value `"success"` or `"error"`.
   - `details`: optional structured JSON object with tool-specific details (such as created item ID or presentation title).
3. If no local tools were executed during the turn, `actions_taken` MUST be an empty array `[]`.
4. Shirley's conversational message MUST be grounded in `actions_taken` and staging results; the assistant MUST distinguish between audio being prepared ("audio listo" in staging) and a track being added to a presentation ("pista agregada"), and MUST NOT claim a track was added if `add_track_from_staged` was not executed.
(Previously: Did not include staged_audio_id in response contract or distinguish staged audio from persisted tracks)

#### Scenario: Conversational turn with successful tool actions
- GIVEN a user prompt requesting "Crea un show Acústico y agrega la nota Afinar en Re"
- WHEN the agent executes `create_presentation` with title "Acústico" and `add_note` with text "Afinar en Re"
- THEN the server MUST return HTTP status 200 with `actions_taken` containing two success descriptors, `active_presentation_id` matching the new presentation ID, and `message` confirming the created presentation and note

#### Scenario: Informational turn with zero tool actions
- GIVEN a user asking "Hola Shirley, ¿qué puedes hacer por mí?"
- WHEN the agent processes the conversational turn without invoking mutation tools
- THEN the server MUST return HTTP status 200 with `actions_taken: []` and a conversational greeting explaining available capabilities

#### Scenario: Active presentation ID tracked across presentation creation
- GIVEN no active presentation in the request (`presentation_id: null`)
- WHEN the user asks "Crea el show Domingo Íntimo" and the agent executes `create_presentation` producing ID "pres-domingo-99"
- THEN the server response MUST set `active_presentation_id: "pres-domingo-99"`

#### Scenario: Download response when target presentation is unknown returns staged_audio_id and asks for destination
- GIVEN a download request without an active presentation where `download_youtube_track` succeeds with ID "staged-yt-555"
- WHEN the agent completes the turn without calling `add_track_from_staged`
- THEN the server MUST return HTTP status 200 with `staged_audio_id: "staged-yt-555"`, `actions_taken` containing the download descriptor, and a message confirming the audio is ready and asking which presentation to add it to

#### Scenario: Download response when target presentation is known chains track addition and returns null staged_audio_id
- GIVEN a download request targeting active presentation "pres-1" where both `download_youtube_track` and `add_track_from_staged` succeed
- WHEN the server returns the turn response
- THEN the server response MUST set `staged_audio_id: null` (or omit the field), set `actions_taken` containing both success descriptors, and confirm in `message` that the track was added to presentation "pres-1"
