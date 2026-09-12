# Agent Chat Specification

## Purpose

Defines the conversational HTTP API endpoint, message protocol, Groq chat completions orchestration, graceful degradation, and request-scoped audio staging lifecycle for Shirley, the AI assistant in Shirley Show.

## Requirements

### Requirement: Agent Chat API Endpoint and Content Negotiation

1. The backend HTTP server MUST provide `POST /api/agent/chat` to process conversational requests from the client.
2. The endpoint MUST accept requests with `Content-Type: application/json` containing a JSON object with:
       - `message`: a required non-empty string with the user's conversational prompt.
       - `history`: an optional array of conversation turns `[{"role": "user" | "assistant", "content": string}]`.
       - `presentation_id`: an optional string with the active presentation identifier.
       - `event_id`: an optional string with the active event identifier.
       - `staged_audio_id`: an optional string referencing an active staged audio entry from a prior download or separation turn.
3. The endpoint MUST accept requests with `Content-Type: multipart/form-data` containing:
       - `message`: a string containing the user's prompt or instruction.
       - `history`: an optional JSON-encoded string or field containing previous conversation turns.
       - `presentation_id`: an optional string containing the active presentation identifier.
       - `event_id`: an optional string containing the active event identifier.
       - `staged_audio_id`: an optional string referencing an active staged audio entry from a prior download or separation turn.
       - `file`: a single attachment file, which MAY be either an audio file (up to 100 MiB) or an image file (up to 10 MiB).
4. Requests with any other `Content-Type` MUST be rejected with HTTP status 415 Unsupported Media Type.
5. Request processing lifetime and timeouts:
   - Non-separation requests MUST respect the canonical request deadline (30 seconds, or 120 seconds for YouTube downloads).
   - Requests that execute vocal separation (`separate_vocals`) MUST support an extended timeout of up to 600 seconds, configurable via `SHIRLEY_SEPARATION_TIMEOUT_SECS` (defaulting to 600).
(Previously: Accepted only single audio attachment file under multipart/form-data, did not support image attachments or event_id context)

#### Scenario: Text-only JSON chat request
- GIVEN the user submits a text message "Crea una presentación para el show del sábado" with no attachments
- WHEN the client sends `POST /api/agent/chat` with `Content-Type: application/json` and body `{"message": "Crea una presentación para el show del sábado"}`
- THEN the server MUST process the request as a text-only turn and return HTTP status 200 with the assistant's reply

#### Scenario: Multipart request with audio attachment and instruction
- GIVEN a user attaching an audio file "acustico.mp3" under 100 MiB with text "Añade esta pista al show activo" and active presentation "pres-1"
- WHEN the client sends `POST /api/agent/chat` as `multipart/form-data` with fields `message`, `presentation_id`, and `file`
- THEN the server MUST stage the audio file locally, process the turn with the staged audio context, and return HTTP status 200

#### Scenario: Multipart request with image attachment for poster upload
- GIVEN a user attaching an image file "afiche.jpg" of 3 MiB with text "Actualiza el afiche del evento" and active event "evt-1"
- WHEN the client sends `POST /api/agent/chat` as `multipart/form-data` with fields `message`, `event_id`, and `file`
- THEN the server MUST validate image magic bytes, stage the image in turn context, process the turn allowing `subir_afiche`, and return HTTP status 200

#### Scenario: Reject request with unsupported media type
- GIVEN an HTTP request sent to `POST /api/agent/chat` with `Content-Type: text/plain`
- WHEN the server receives the request
- THEN the server MUST return HTTP status 415 Unsupported Media Type and MUST NOT initiate any agent loop or provider call

#### Scenario: JSON chat request with staged_audio_id from previous separation turn
- GIVEN a user replying "Añádela al show Acústico" with an active staging reference "staged-sep-999"
- WHEN the client sends `POST /api/agent/chat` with `{"message": "Añádela al show Acústico", "staged_audio_id": "staged-sep-999"}`
- THEN the server MUST accept `staged_audio_id` in the request context and make it available for `add_track_from_staged` execution
### Requirement: Response Contract and Action Reporting

1. A successful request to `POST /api/agent/chat` MUST return HTTP status 200 OK with a JSON response containing:
       - `message`: a non-empty string containing Shirley's conversational response.
       - `actions_taken`: an array of structured action descriptors for tools successfully executed during the turn.
       - `active_presentation_id`: an optional string containing the currently active presentation identifier (either preserved from request or updated by tool execution).
       - `staged_audio_id`: an optional string containing the opaque identifier of a staged audio file if audio was downloaded or separated but not yet added to a presentation; null or omitted otherwise.
2. Each item in `actions_taken` MUST contain:
       - `tool`: string naming the executed tool.
       - `presentation_id`: string with the affected presentation identifier.
       - `summary`: string with a human-readable description of the completed action.
       - `status`: string with value `"success"` or `"error"`.
       - `details`: optional structured JSON object with tool-specific details (such as created item ID or presentation title).
3. If no local tools were executed during the turn, `actions_taken` MUST be an empty array `[]`.
4. Shirley's conversational message MUST be grounded in `actions_taken` and staging results; the assistant MUST distinguish between source prepared, separation finished, and track added, and MUST NOT claim a track was added if `add_track_from_staged` was not executed.
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

#### Scenario: Separation response when target presentation is unknown returns staged_audio_id and asks for destination
- GIVEN a separation request without an active presentation where `separate_vocals` succeeds with ID "staged-sep-555"
- WHEN the agent completes the turn without calling `add_track_from_staged`
- THEN the server MUST return HTTP status 200 with `staged_audio_id: "staged-sep-555"`, `actions_taken` containing the separation descriptor, and a message confirming the separated audio is ready and asking which presentation to add it to

#### Scenario: Separation response when target presentation is known chains track addition and returns null staged_audio_id
- GIVEN a separation request targeting active presentation "pres-1" where both `separate_vocals` and `add_track_from_staged` succeed
- WHEN the server returns the turn response
- THEN the server response MUST set `staged_audio_id: null` (or omit the field), set `actions_taken` containing both success descriptors, and confirm in `message` that the track was added to presentation "pres-1"
---

### Requirement: Graceful Degradation Without API Key

1. If `GROQ_API_KEY` is not set, empty, or consists solely of whitespace in the server runtime environment:
   - The backend MUST NOT attempt any outbound network connection to Groq.
   - The endpoint `POST /api/agent/chat` MUST return HTTP status 200 OK with a friendly, warm message explaining that Shirley requires a `GROQ_API_KEY` configured in the server environment.
   - The response MUST include `actions_taken: []`.
   - The response MUST NOT leak file paths, environment variable values, or system internals.
2. All non-AI manual workflows (catalog, show detail, item addition, editing, reordering, audio playback, and Stage Mode) MUST remain fully operational.

#### Scenario: Request received when GROQ_API_KEY is missing or empty
- GIVEN the server is running without `GROQ_API_KEY` configured in the environment
- WHEN a client sends any valid request to `POST /api/agent/chat`
- THEN the server MUST return HTTP status 200 with a friendly configuration guidance message, `actions_taken: []`, and MUST NOT make outbound network calls

#### Scenario: Request received when GROQ_API_KEY is whitespace only
- GIVEN the server has `GROQ_API_KEY="   "` set in the environment
- WHEN a client sends a request to `POST /api/agent/chat`
- THEN the server MUST treat the key as absent, return HTTP status 200 with setup guidance, and log a warning without echoing whitespace or credentials
---

### Requirement: Provider Error Handling and Resilience

1. If the Groq API returns HTTP 401 Unauthorized (invalid or revoked key):
   - The backend MUST log a diagnostic warning with credentials redacted.
   - The endpoint MUST return HTTP status 200 OK with a clear message indicating that Shirley could not authenticate with the configured AI provider.
   - The response MUST NOT expose the invalid key string or raw provider error body.
2. If the Groq API returns HTTP 429 Too Many Requests (rate limit or quota exceeded):
   - The backend MUST return HTTP status 200 OK with a calm message explaining that Shirley is currently overloaded and asking the user to try again in a moment or use the direct buttons.
3. If the Groq API returns HTTP 5xx Server Error or network connection failure:
   - The backend MUST return HTTP status 200 OK explaining that the AI provider is temporarily unavailable.
4. Any tool mutations that were already committed to the database prior to a provider failure MUST remain committed and MUST be listed in `actions_taken` with status `"success"`.

#### Scenario: Groq returns 401 Unauthorized with invalid API key
- GIVEN a configured `GROQ_API_KEY` that has been revoked or is malformed
- WHEN the user sends a chat message and Groq responds with HTTP 401
- THEN the server MUST return HTTP status 200 with a friendly assistant message explaining authentication failure, MUST NOT expose the key in logs or payload, and MUST preserve conversation state

#### Scenario: Groq returns 429 Rate Limit
- GIVEN the Groq API responds with HTTP 429 Too Many Requests
- WHEN the agent processes the request
- THEN the server MUST return HTTP status 200 explaining that rate limits were reached, advising a retry or manual action, with `actions_taken` reflecting only already-committed tools

#### Scenario: Groq returns 500 or 503 Internal Server Error
- GIVEN Groq experiences an outage and returns HTTP 503 Service Unavailable
- WHEN the agent attempts a chat completion
- THEN the server MUST return HTTP status 200 with an empathetic provider-unavailable message, avoiding unhandled 500 errors to the client
---

### Requirement: Request-Scoped Audio Staging and Lifecycle Cleanup

1. When a multipart chat request includes an audio file:
   - The backend MUST immediately stream and stage the file in a dedicated temporary staging directory using a secure, random filename.
   - The staging handle MUST be managed by a deterministic guard (`StagingFileGuard`) scoped strictly to the current request execution.
   - Only the local tool `add_track_from_staged` MAY consume the staged file to move it into permanent audio storage.
2. The staged file MUST be deleted from disk immediately upon request completion if:
   - The tool `add_track_from_staged` was never called during the turn.
   - The tool `add_track_from_staged` failed during execution.
   - The agent loop timed out or reached the iteration limit before consuming the file.
   - Groq returned an error, or the client disconnected prematurely.
3. Raw audio binary content MUST NEVER be transmitted to Groq.

#### Scenario: Staged audio consumed by add_track_from_staged tool and cleaned from staging
- GIVEN a multipart request with "cancion.mp3" (5 MiB) and instruction "Añade esta pista como Pista 1"
- WHEN the agent calls `add_track_from_staged` and successfully moves the file into managed storage
- THEN the file MUST be removed from temporary staging, saved in managed audio, and no orphaned temporary file MUST remain

#### Scenario: Attached audio unused by assistant cleaned up after request
- GIVEN a multipart request containing an audio file but text asking only "Lista mis presentaciones"
- WHEN the agent lists presentations without calling `add_track_from_staged`
- THEN the server MUST automatically delete the temporary staged audio file upon request completion and return the list of presentations

#### Scenario: Provider error or timeout triggers immediate staged audio cleanup
- GIVEN a multipart request with an attached audio file that is staged
- WHEN the Groq API call times out or fails with an error
- THEN the server's staging guard MUST automatically delete the temporary staged audio file from disk before returning the response

#### Scenario: Client disconnect during multipart processing cleans up staged file
- GIVEN an active multipart upload and staging in progress
- WHEN the client terminates the connection abruptly
- THEN the server MUST abort processing and purge any partially or completely staged temporary audio file
---

### Requirement: Audio Attachment Validation and Limits

1. The backend MUST validate any attached audio file before invoking Groq:
   - Allowed file extensions: `.mp3`, `.wav`, and `.m4a` (case-insensitive).
   - Maximum file size: exactly 100 MiB (104,857,600 bytes).
2. If the attached file exceeds 100 MiB:
   - The server MUST immediately abort the upload, remove any partial staging file, and return HTTP status 400 Bad Request with an error message stating that the audio file exceeds the 100 MiB limit.
   - The server MUST NOT invoke the Groq API.
3. If the attached file has an unsupported format (such as `.flac`, `.ogg`, `.mp4`, `.pdf`, or `.exe`):
   - The server MUST reject the request with HTTP status 400 Bad Request stating that only `.mp3`, `.wav`, and `.m4a` formats are supported.
   - The server MUST NOT invoke the Groq API.
4. If a multipart request includes an audio attachment but the `message` field is empty or whitespace-only:
   - The server MUST assign a sensible default instruction (such as "Añadir esta pista de audio") and proceed to query the model or inspect active presentation context rather than rejecting the request.

#### Scenario: Reject audio attachment exceeding 100 MiB
- GIVEN a multipart request containing an audio file of 105 MiB
- WHEN the server processes the incoming upload
- THEN the server MUST reject the request with HTTP status 400 Bad Request, delete any partial bytes from disk, and MUST NOT make outbound requests to Groq

#### Scenario: Reject unsupported attachment file format
- GIVEN a multipart request containing a file named "ensayo.flac" or "documento.pdf"
- WHEN the server inspects the file extension
- THEN the server MUST reject the request with HTTP status 400 Bad Request, delete the temporary file, and MUST NOT make outbound requests to Groq

#### Scenario: Audio attached without explicit text instruction defaults gracefully
- GIVEN a multipart request containing a valid "intro.mp3" file, active `presentation_id: "pres-1"`, and an empty `message: ""`
- WHEN the server processes the request
- THEN the server MUST treat the turn as an instruction to add the attached audio track to presentation "pres-1", invoke the agent loop, and proceed safely
---

### Requirement: Ephemeral History and Context Size Bounds

1. The client conversation history supplied in `history` MUST be treated as ephemeral, untrusted input.
2. The server MUST NOT store conversation history in SQLite, log files, or permanent disk storage.
3. The server MUST enforce upper bounds on conversation history to prevent excessive latency, token exhaustion, and unbounded cost:
   - Maximum number of history messages: 20 messages (up to 10 user/assistant turn pairs).
   - Maximum total character count for history text: 32,000 characters.
4. If the client sends history exceeding either bound:
   - The server MUST retain the most recent messages that fit within the bounds, discard the oldest turns, and prepend a system note indicating that older context was truncated.
   - The server MUST NOT fail with an error due to oversized history.
5. An empty or missing `history` field MUST be processed cleanly as a new conversation turn.

#### Scenario: Turn with empty or absent conversation history
- GIVEN a request to `POST /api/agent/chat` with `history: []` or without a `history` field
- WHEN the server processes the request
- THEN the server MUST initialize the model context with the system prompt and the current user message, executing normally

#### Scenario: Turn with long conversation history pruned to size limit
- GIVEN a client request containing 35 prior conversation messages totaling 50,000 characters
- WHEN the server processes the incoming history
- THEN the server MUST safely truncate the oldest messages, retain the most recent 20 messages within the 32,000-character budget, and proceed with the request without crashing

#### Scenario: Ephemeral conversation not persisted in database
- GIVEN multiple conversation turns between user and Shirley via `POST /api/agent/chat`
- WHEN inspecting the SQLite database
- THEN zero conversation messages, chat logs, or user prompts MUST be stored in the database
---

### Requirement: Input Validation and Text Guardrails

1. A JSON request with an empty or whitespace-only `message` string MUST be rejected with HTTP status 400 Bad Request.
2. The user `message` MUST be trimmed of leading and trailing whitespace.
3. Assistant text responses MUST be delivered as sanitized text; raw unescaped HTML or executable scripts MUST NOT be generated as raw DOM instructions.
4. The system prompt MUST configure Shirley as a warm, concise, and helpful stage assistant who confirms actions clearly and asks for clarification when requests are ambiguous.

#### Scenario: Reject empty or whitespace-only message without attachment
- GIVEN a JSON request with body `{"message": "   "}`
- WHEN the client sends the request to `POST /api/agent/chat`
- THEN the server MUST return HTTP status 400 Bad Request with a validation error requiring a non-empty message

#### Scenario: Accept multiline text message with special characters
- GIVEN a request containing multiline lyrics text with accents, punctuation, and song chords
- WHEN the server receives the payload
- THEN the server MUST preserve the multiline formatting and character encoding throughout processing
---

### Requirement: Ephemeral Staged Audio Registry with TTL Lifecycle

The backend HTTP server MUST maintain an in-memory `StagedAudioRegistry` to manage the lifecycle of temporary downloaded and separated audio tracks across conversational turns with strict time-to-live and disk cleanup guarantees.

1. Registry storage and entry structure:
   - The registry MUST store temporary audio entries resulting from `download_youtube_track` and `separate_vocals` executions in memory.
   - Each registry entry MUST contain:
     - `staged_audio_id`: an opaque, unique string identifier (e.g. UUID v4).
     - `file_path`: absolute path to the validated MP3 file in a dedicated staging directory.
     - `title`: sanitized track title derived from input or source metadata.
     - `duration_secs`: content duration in seconds (up to 600 seconds for vocal separation, 1200 seconds for downloads).
     - `size_bytes`: file size in bytes (up to 104,857,600 bytes).
     - `created_at`: monotonic or system timestamp of registration.
     - `consumed`: boolean flag indicating whether the audio has been moved to permanent storage.
2. Time-to-live (TTL) and disk eviction:
   - Each entry MUST have a TTL of exactly 15 minutes (900 seconds) from registration.
   - Upon TTL expiration, the registry MUST invalidate the entry and the server MUST delete the associated temporary MP3 file from disk immediately.
   - Expired entries MUST NOT be retrievable or consumable by `add_track_from_staged` or any other operation.
3. Single-use consumption guarantee:
   - A valid staging reference MUST be consumable at most once.
   - When successfully consumed by `add_track_from_staged`, `consumed` MUST be set to true and the file moved out of the staging directory into managed audio storage.
   - Any subsequent call attempting to reuse the same `staged_audio_id` MUST be rejected with a structured error `{ "error": "audio_already_consumed", "message": "El audio preparado ya fue consumido" }`.
4. Staging preservation on failed mutation:
   - If an operation attempting to add the staged audio to a presentation fails (for example, if the specified presentation does not exist), the registry entry MUST NOT be marked as consumed, allowing the user to select another presentation while the 15-minute TTL remains active.
5. Server restart and orphan cleanup:
   - Staged audio references in memory are ephemeral and MUST NOT be persisted in SQLite.
   - Restarting the server invalidates all previous in-memory references.
   - On server startup, the server MUST scan the dedicated temporary staging directory and purge any orphaned files from previous runs without altering files in the managed audio directory.

#### Scenario: Register separated 320k MP3 audio in registry with 15-minute TTL
- GIVEN a successful vocal separation producing an MP3 CBR 320 kbps file titled "Canción (Coros)"
- WHEN the audio is registered in `StagedAudioRegistry`
- THEN the registry MUST generate an opaque `staged_audio_id`, record its file path and metadata, and set an active TTL of 15 minutes

#### Scenario: Consume separated audio once successfully and block second consumption
- GIVEN an active staging reference "staged-sep-123" in `StagedAudioRegistry`
- WHEN `add_track_from_staged` consumes "staged-sep-123" to add a track to presentation "pres-1"
- THEN the file MUST be moved to managed audio, "staged-sep-123" marked as consumed, and a second call with "staged-sep-123" MUST be rejected with `audio_already_consumed`

#### Scenario: Staged audio entry expires after 15 minutes and purges file from disk
- GIVEN an audio file staged in `StagedAudioRegistry` at time T
- WHEN 15 minutes elapse without the audio being consumed by a show presentation
- THEN the registry entry MUST be invalidated and the temporary MP3 file MUST be deleted from disk

#### Scenario: Failed track addition preserves staged audio reference for retry
- GIVEN an active staging entry "staged-sep-456" and a target presentation that does not exist
- WHEN `add_track_from_staged` fails with `presentation_not_found`
- THEN "staged-sep-456" MUST remain unconsumed and available in the registry for a subsequent request within its original 15-minute TTL

#### Scenario: Server restart clears memory registry and cleans up orphan staging files
- GIVEN temporary separation files residing in the staging directory when the server restarts
- WHEN the server finishes startup initialization
- THEN the in-memory registry MUST start empty and the temporary staging directory MUST be purged of any orphaned files from previous process runs

#### Scenario: Register downloaded audio in registry with 15-minute TTL
- GIVEN a successful YouTube download of an 8 MiB MP3 track titled "Zamba de mi esperanza"
- WHEN the audio is registered in `StagedAudioRegistry`
- THEN the registry MUST generate an opaque `staged_audio_id`, record its file path and metadata, and set an active TTL of 15 minutes

#### Scenario: Consume staged audio once successfully and block second consumption
- GIVEN an active staging reference "staged-123" in `StagedAudioRegistry`
- WHEN `add_track_from_staged` consumes "staged-123" to add a track to presentation "pres-1"
- THEN the file MUST be moved to managed audio, "staged-123" marked as consumed, and a second call with "staged-123" MUST be rejected with `audio_already_consumed`
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

### Requirement: Conversational Vocal Separation Feedback and Client State Continuity

The system MUST provide qualitative progress feedback during vocal separation operations and preserve structured staging context across conversation turns without requiring streaming protocols, WebSockets, or server-sent events.

1. Client typing indicator specialization:
   - When the user submits a chat request initiating vocal separation (or choosing mode/destination to start separation), the frontend chat client MUST display the specialized qualitative indicator text "separando voces…" instead of the generic "Shirley está pensando...".
   - The specialized indicator MUST retain Shirley's signature warm golden/amber pulsing animation and `aria-live="polite"` announcement.
   - The indicator MUST NOT display simulated or fabricated percentages, progress bars, or estimated time remaining.
   - When the HTTP request completes (whether success, error, or timeout) or the chat panel is closed, the specialized indicator MUST be dismissed and the composer restored.
2. GPU busy notification:
   - If the GPU is currently busy with another separation, the assistant MAY inform the user that the operation is waiting for the processor ("esperando turno en la GPU...").
3. Structured staging continuity across turns:
   - When the server response to `POST /api/agent/chat` includes a non-null `staged_audio_id`, the client MUST store this identifier in ephemeral in-memory state.
   - On the user's immediately following chat request, the client MUST automatically include the stored `staged_audio_id` in the request payload.
   - Once a turn completes where `add_track_from_staged` was executed (confirmed in `actions_taken`) or if the user initiates an explicit unrelated command, the client MUST clear the stored `staged_audio_id`.
   - The client MUST NOT store `staged_audio_id` in `localStorage` or persist it across page reloads; a browser refresh resets ephemeral chat state cleanly.
4. Grounded multi-stage messaging:
   - Shirley's conversational message MUST distinguish between:
     a) "fuente preparada": the original audio has been downloaded/staged,
     b) "separación terminada": vocal separation completed and MP3 is staged,
     c) "pista agregada": the track has been successfully persisted in the target presentation.
   - Shirley MUST NOT claim a track was added if `add_track_from_staged` was not successfully executed.
5. Error and edge case handling:
   - Source audio exceeding 10 minutes (600s) MUST produce a friendly message indicating the duration limit without executing separation.
   - PolarFormer absence MUST produce a clear, non-alarmist message explaining that instrumental mode is not currently installed, offering chorus mode or keeping the staged audio within its TTL.
   - Inconsistent stems (mismatched sample rate, channels, or length) MUST report a friendly error without partial output.
   - Component teardown or client disconnect MUST cancel the pending HTTP request; the backend MUST terminate child processes via kill-and-wait and purge temporary files.

#### Scenario: User confirms vocal separation and chat displays specialized typing indicator
- GIVEN the chat composer is ready with a staged audio source
- WHEN the user confirms "Separa coros para el show del viernes"
- THEN the client MUST display the typing indicator with the text label "separando voces…" while the HTTP request remains in flight

#### Scenario: Client retains staged_audio_id from separation response and forwards it in next turn
- GIVEN the assistant returns HTTP 200 with `staged_audio_id: "staged-sep-789"` and message "La separación vocal está lista. ¿A qué presentación deseas agregarla?"
- WHEN the user replies "Agrégala al show Peña del Sábado"
- THEN the client MUST send `POST /api/agent/chat` containing `message: "Agrégala al show Peña del Sábado"` and `staged_audio_id: "staged-sep-789"`

#### Scenario: Client clears staged_audio_id after separated track is added to presentation
- GIVEN the client has an active stored `staged_audio_id: "staged-sep-789"`
- WHEN the assistant completes a turn where `actions_taken` includes a successful `add_track_from_staged` action
- THEN the client MUST clear the stored `staged_audio_id` from ephemeral state

#### Scenario: Component teardown or request error restores composer and clears active indicator
- GIVEN a separation request currently in flight showing "separando voces…"
- WHEN the server returns an error or the request times out
- THEN the typing indicator MUST be removed immediately, an actionable message displayed, and the composer input re-enabled

#### Scenario: PolarFormer absent displays actionable explanation and composer ready for chorus alternative
- GIVEN the user requests instrumental separation on a server without PolarFormer installed
- WHEN the server processes the request
- THEN the assistant MUST respond explaining that instrumental mode is not available, offer chorus mode as an alternative, and keep the source audio staged within its TTL

### Requirement: Image Attachment Validation and Multipart Magic-Byte Classification

The backend HTTP server MUST classify and validate attachments in multipart chat requests based on content magic bytes, supporting promotional event poster images up to 10 MiB alongside existing audio attachments:

1. Magic-byte content classification:
   - When a multipart request arrives at `POST /api/agent/chat` containing a `file` field, the server MUST inspect the initial bytes of the stream to determine the true content type, rather than trusting the client-reported `Content-Type` header or file extension.
   - Supported image formats and signatures:
     - JPEG: starts with `0xFF, 0xD8, 0xFF`.
     - PNG: starts with `0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A`.
     - WebP: starts with `RIFF` header and contains `WEBP` format tag (`0x52, 0x49, 0x46, 0x46, ..., 0x57, 0x45, 0x42, 0x50`).
2. Size boundaries and staging:
   - Maximum allowed image size: exactly 10 MiB (10,485,760 bytes, matching `MAX_POSTER_SIZE_BYTES`).
   - If the incoming file matches a supported image signature and its size is $\le 10 \text{ MiB}$:
     - The server MUST stage the file data in memory or temporary buffer as `StagedImageHandle::Available { data, format, byte_size }`.
     - The staged image handle MUST be made accessible to the agent turn context (`TurnToolContext.staged_image`).
   - If the file exceeds 10 MiB:
     - The server MUST immediately abort the upload and return HTTP status 400 Bad Request stating that the image exceeds the 10 MiB limit.
     - The server MUST NOT invoke Groq.
   - If the file declares an image extension (`.jpg`, `.jpeg`, `.png`, `.webp`) or MIME type but has invalid, spoofed, or corrupted magic bytes:
     - The server MUST reject the upload with HTTP status 400 Bad Request stating that the file content does not match a supported image format.
     - The server MUST NOT invoke Groq.
3. Audio path coexistence:
   - If the file matches supported audio formats (`.mp3`, `.wav`, `.m4a` up to 100 MiB), the server MUST route it through the existing audio staging path (`StagingFileGuard`) without regression.
   - If the file matches neither supported audio nor supported image formats:
     - The server MUST reject the request with HTTP status 400 Bad Request.
4. Request-scoped lifecycle:
   - A staged image handle is strictly ephemeral and scoped to the active HTTP request.
   - If `subir_afiche` is not called, fails, or if the request times out or disconnects, the staged image data MUST be discarded from memory/disk upon request completion.

#### Scenario: Valid JPEG image under 10 MiB classified by magic bytes and staged
- GIVEN a multipart request with a 3 MiB file containing valid JPEG magic bytes (`FF D8 FF`) and text "Usa esta imagen como afiche"
- WHEN the server processes the incoming multipart upload
- THEN the server MUST classify the file as JPEG, stage the bytes in `TurnToolContext.staged_image`, and proceed to the agent loop with HTTP status 200

#### Scenario: Valid PNG image under 10 MiB classified and staged
- GIVEN a multipart request with a 5 MiB file containing valid PNG magic bytes and text "Ponle este afiche al evento"
- WHEN the server processes the multipart upload
- THEN the server MUST classify the file as PNG, stage the image in turn context, and invoke the agent loop

#### Scenario: Valid WebP image under 10 MiB classified and staged
- GIVEN a multipart request with a 2 MiB file containing valid WebP magic bytes and text "Nuevo afiche"
- WHEN the server processes the multipart upload
- THEN the server MUST classify the file as WebP, stage the image in turn context, and invoke the agent loop

#### Scenario: Reject image exceeding 10 MiB limit
- GIVEN a multipart request containing an image file of 12 MiB
- WHEN the server validates the incoming upload size
- THEN the server MUST immediately reject the request with HTTP status 400 Bad Request, discard the partial upload, and MUST NOT invoke Groq

#### Scenario: Reject spoofed or corrupted image file
- GIVEN a multipart request containing a text file or executable renamed to "afiche.png" without valid PNG magic bytes
- WHEN the server inspects the content bytes
- THEN the server MUST reject the upload with HTTP status 400 Bad Request and MUST NOT invoke Groq

#### Scenario: Existing audio upload remains fully functional without regression
- GIVEN a multipart request containing an 8 MiB valid MP3 file with text "Añade esta pista"
- WHEN the server inspects the upload
- THEN the server MUST classify the file as audio, stage it via `StagingFileGuard`, and expose it for `add_track_from_staged` without error

#### Scenario: Unconsumed staged image cleaned up after request
- GIVEN a multipart request containing a valid 2 MiB image but text asking only "Lista los eventos"
- WHEN the agent completes the turn without invoking `subir_afiche`
- THEN the staged image handle MUST be dropped and no temporary image data MUST remain on disk or leak in memory

---

### Requirement: Server-Trusted Two-Turn Confirmation Boundary and Context Continuity

The backend HTTP server and agent orchestration MUST enforce a server-trusted confirmation boundary across conversational turns for destructive operations, preventing unverified or client-forged deletions:

1. Pending confirmation tracking:
   - When a conversational turn expresses intent to perform any of the seven destructive operations (`eliminar_presentacion`, `eliminar_elemento`, `eliminar_evento`, `finalizar_evento`, `eliminar_entrada`, `eliminar_gasto`, `eliminar_interesado`):
     - The server MUST record a pending authorization containing:
       - `tool_name`: string naming the requested destructive tool.
       - `target_id`: string with the unique identifier of the target resource.
       - `target_label`: string with the human-readable name or title of the target.
       - `timestamp`: monotonic or system timestamp.
     - The assistant message MUST communicate the target, explain consequence details, and request explicit confirmation.
2. Server-side validation on confirmation turn:
   - On the immediately following user turn:
     - When the user message contains the required explicit confirmation phrase (`"sí, elimínalo"` or `"sí, finalízalo"`):
       - The server MUST verify that a valid server-trusted pending authorization exists and that the requested tool call matches `tool_name` and `target_id`.
       - Client-authored conversation history in the `history` array MUST NOT be treated as authorization evidence; only server-trusted pending state qualifies.
     - If the user provides an ambiguous response (`"dale"`, `"ok"`, `"hazlo"`, `"bueno"` without the explicit confirmation phrase):
       - The server MUST NOT authorize tool execution.
       - The pending state MUST NOT be consumed for execution.
     - If the user expresses cancellation, hesitation (`"espera"`, `"cancela"`, `"mejor no"`), or submits an unrelated command:
       - The pending authorization MUST be immediately cleared.
3. Single-use consumption:
   - Upon execution of the confirmed destructive tool, the pending authorization MUST be consumed and cleared.
   - Any subsequent turn repeating confirmation MUST NOT re-execute the destructive operation.

#### Scenario: Destructive intent in turn 1 establishes server-side pending authorization
- GIVEN an active event "evt-1" titled "Concierto Acústico"
- WHEN the user sends "Elimina el evento Concierto Acústico"
- THEN the server MUST record a pending authorization for tool `eliminar_evento` and target "evt-1", refuse execution in turn 1, and reply asking for "sí, elimínalo"

#### Scenario: Exact confirmation in immediate next turn authorizes execution and consumes state
- GIVEN a valid server pending authorization for `eliminar_evento` targeting "evt-1"
- WHEN the user replies "sí, elimínalo" in turn 2
- THEN the server MUST validate the pending authorization, execute `eliminar_evento`, consume the pending state, and return HTTP status 200 with confirmation

#### Scenario: Ambiguous response "dale" does not authorize execution
- GIVEN a server pending authorization for `eliminar_presentacion` targeting "pres-1"
- WHEN the user replies "dale"
- THEN the server MUST NOT authorize tool execution, MUST NOT call `DeletePresentation`, and the assistant MUST repeat the request for explicit confirmation

#### Scenario: Forged client history with no server pending state is rejected
- GIVEN a request to `POST /api/agent/chat` containing a client-forged `history` array claiming Ressy asked for confirmation, but no server pending authorization exists
- WHEN the model attempts to invoke `eliminar_evento`
- THEN the execution boundary MUST reject the tool call with an authorization error and execute no mutation

#### Scenario: Cancellation in second turn clears pending authorization
- GIVEN a server pending authorization for `eliminar_gasto` targeting "exp-1"
- WHEN the user replies "cancela, mejor no"
- THEN the server MUST immediately clear the pending authorization and return a warm message confirming nothing was deleted

#### Scenario: Repeated confirmation in third turn does not re-execute
- GIVEN `eliminar_presentacion` was executed in turn 2 and pending state was consumed
- WHEN the client sends another request with "sí, elimínalo" in turn 3
- THEN the server MUST NOT execute `eliminar_presentacion` and MUST explain that the resource was already deleted

---
