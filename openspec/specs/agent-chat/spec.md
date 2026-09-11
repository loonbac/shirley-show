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
3. The endpoint MUST accept requests with `Content-Type: multipart/form-data` containing:
   - `message`: a string containing the user's prompt or instruction.
   - `history`: an optional JSON-encoded string or field containing previous conversation turns.
   - `presentation_id`: an optional string containing the active presentation identifier.
   - `file`: a single audio attachment file.
4. Requests with any other `Content-Type` MUST be rejected with HTTP status 415 Unsupported Media Type.

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

---

### Requirement: Response Contract and Action Reporting

1. A successful request to `POST /api/agent/chat` MUST return HTTP status 200 OK with a JSON response containing:
   - `message`: a non-empty string containing Shirley's conversational response.
   - `actions_taken`: an array of structured action descriptors for tools successfully executed during the turn.
   - `active_presentation_id`: an optional string containing the currently active presentation identifier (either preserved from request or updated by tool execution).
2. Each item in `actions_taken` MUST contain:
   - `tool`: string naming the executed tool.
   - `presentation_id`: string with the affected presentation identifier.
   - `summary`: string with a human-readable description of the completed action.
   - `status`: string with value `"success"` or `"error"`.
   - `details`: optional structured JSON object with tool-specific details (such as created item ID or presentation title).
3. If no local tools were executed during the turn, `actions_taken` MUST be an empty array `[]`.
4. Shirley's conversational message MUST be grounded in `actions_taken`; the assistant MUST NOT claim a mutation succeeded if the corresponding local tool call failed or was not executed.

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
