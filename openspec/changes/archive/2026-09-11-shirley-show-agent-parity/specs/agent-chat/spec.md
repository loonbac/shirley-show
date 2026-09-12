# Delta for Agent Chat

## ADDED Requirements

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

## MODIFIED Requirements

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
