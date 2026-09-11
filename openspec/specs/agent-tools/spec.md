# Agent Tools Specification

## Purpose

Defines the bounded local tool allowlist, parameter schemas, presentation ID integrity, execution dispatch, iteration limits, 30-second global deadline, and safety invariants for Shirley's tool-calling loop in Shirley Show.

## Requirements

### Requirement: Fixed Tool Allowlist and Schema Definition

1. The agent system MUST expose exactly seven local tools to the model:
   - `create_presentation`
   - `list_presentations`
   - `get_presentation`
   - `add_lyrics`
   - `add_note`
   - `add_track_from_staged`
   - `download_youtube_track`
2. The agent system MUST NOT expose any tool for deleting, renaming, editing, or reordering presentations or items, nor any tool for audio playback, Stage Mode manipulation, filesystem access, arbitrary shell execution, arbitrary HTTP requests, or raw SQL queries.
3. If the model invokes an unknown tool or a tool not present in the allowlist:
   - The dispatcher MUST NOT execute any system action.
   - The dispatcher MUST return a structured tool error `{"error": "unknown_tool", "message": "Herramienta no permitida"}` back to the model within the loop.
4. Tool parameter parsing and validation MUST occur strictly on the server before invoking domain operations.
(Previously: Allowed exactly six tools; excluded download_youtube_track)

#### Scenario: Model calls an allowed tool with valid parameters
- GIVEN the model requests tool `create_presentation` with arguments `{"name": "Ensayo General"}`
- WHEN the tool dispatcher receives the call
- THEN the dispatcher MUST validate the name and execute the domain use case, returning the created presentation details

#### Scenario: Model attempts to call an unauthorized or hallucinated tool
- GIVEN the model generates a tool call `delete_presentation` with arguments `{"presentation_id": "pres-1"}`
- WHEN the tool dispatcher inspects the requested tool name
- THEN the dispatcher MUST reject the call immediately with an unknown tool error, execute no mutation, and return the error to the model

#### Scenario: Model attempts destructive operation via tool
- GIVEN a user prompt asking the assistant to delete a presentation or reorder songs
- WHEN the agent processes the request
- THEN the assistant MUST recognize that destructive and reordering tools do not exist and MUST explain that deletions and reorders must be performed directly by the user in the show view

#### Scenario: Model calls download_youtube_track tool with valid URL
- GIVEN a user prompt asking Shirley to download a YouTube track
- WHEN the model calls `download_youtube_track` with `{"url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"}`
- THEN the tool dispatcher MUST accept the call as an authorized tool and execute the download use case

---

### Requirement: Presentation Lifecycle Tools

1. `create_presentation`:
   - Input arguments: `name` (required, string).
   - Trims leading and trailing whitespace; rejects empty or whitespace-only names with an error.
   - Invokes the canonical presentation creation use case, creating an empty item sequence and assigning a stable unique identifier.
   - Returns `{ "id": string, "title": string, "created_at": string }`.
2. `list_presentations`:
   - Input arguments: none.
   - Invokes the canonical catalog use case.
   - Returns an array of presentation summaries: `[{ "id": string, "title": string, "total_items": u32, "tracks_count": u32, "lyrics_count": u32, "notes_count": u32 }]`.
3. `get_presentation`:
   - Input arguments: `presentation_id` (required, string).
   - Invokes the canonical show detail use case.
   - Returns full details including ordered sequence of items: `{ "id": string, "title": string, "items": [...] }`.
   - If `presentation_id` does not exist in storage, returns `{ "error": "presentation_not_found", "message": "No se encontró la presentación solicitada" }`.

#### Scenario: Execute create_presentation with valid title
- GIVEN the model calls `create_presentation` with `{"name": "Acústico en la Terraza"}`
- WHEN the tool executes
- THEN the system MUST persist a new presentation record, assign a unique stable ID, and return `{ "id": "pres-...", "title": "Acústico en la Terraza", "created_at": "..." }`

#### Scenario: Execute list_presentations returns repertoire overview
- GIVEN two presentations exist in the database with various item counts
- WHEN the model calls `list_presentations`
- THEN the tool MUST return an array of 2 summaries with accurate counts for tracks, lyrics, notes, and total items

#### Scenario: Execute get_presentation for existing presentation
- GIVEN presentation "pres-42" exists with 3 show items
- WHEN the model calls `get_presentation` with `{"presentation_id": "pres-42"}`
- THEN the tool MUST return the complete presentation record with its 3 ordered items

#### Scenario: Execute get_presentation with non-existent identifier returns structured error
- GIVEN presentation ID "pres-ghost" does not exist in storage
- WHEN the model calls `get_presentation` with `{"presentation_id": "pres-ghost"}`
- THEN the tool MUST return `{ "error": "presentation_not_found", "message": "No se encontró la presentación solicitada" }` without crashing

---

### Requirement: Item Creation Tools (add_lyrics, add_note)

1. `add_lyrics`:
   - Input arguments: `presentation_id` (required, string) and `text` (required, string).
   - Validates that `presentation_id` exists in storage; returns `presentation_not_found` error if missing.
   - Trims whitespace and validates that `text` is non-empty; returns validation error if empty.
   - Appends a new item of kind `lyrics` with contiguous position index `N` at the end of the presentation's sequence.
   - Updates the parent presentation's `updated_at` timestamp.
   - Returns `{ "item_id": string, "kind": "lyrics", "position": u32, "presentation_id": string }`.
2. `add_note`:
   - Input arguments: `presentation_id` (required, string) and `text` (required, string).
   - Validates that `presentation_id` exists in storage; returns `presentation_not_found` error if missing.
   - Trims whitespace and validates that `text` is non-empty; returns validation error if empty.
   - Appends a new item of kind `note` with contiguous position index `N` at the end of the presentation's sequence.
   - Updates the parent presentation's `updated_at` timestamp.
   - Returns `{ "item_id": string, "kind": "note", "position": u32, "presentation_id": string }`.

#### Scenario: Add lyrics to existing presentation appends at position N
- GIVEN presentation "pres-1" currently contains 2 items at positions 0 and 1
- WHEN the model calls `add_lyrics` with `{"presentation_id": "pres-1", "text": "Estrofa 1\nLlegaste de repente..."}`
- THEN the system MUST persist a new `lyrics` item at position 2, update presentation "pres-1", and return `{ "item_id": "item-...", "kind": "lyrics", "position": 2, "presentation_id": "pres-1" }`

#### Scenario: Add note to existing presentation appends at position N
- GIVEN presentation "pres-1" contains 1 item at position 0
- WHEN the model calls `add_note` with `{"presentation_id": "pres-1", "text": "Cambio de afinación a Mi bemol"}`
- THEN the system MUST persist a new `note` item at position 1, update presentation "pres-1", and return `{ "item_id": "item-...", "kind": "note", "position": 1, "presentation_id": "pres-1" }`

#### Scenario: Reject add_lyrics or add_note with empty text
- GIVEN an existing presentation "pres-1"
- WHEN the model calls `add_lyrics` or `add_note` with `{"presentation_id": "pres-1", "text": "   "}`
- THEN the tool MUST reject the call with a validation error and MUST NOT create any database record

#### Scenario: Reject item addition to non-existent presentation
- GIVEN presentation ID "pres-nonexistent" does not exist in storage
- WHEN the model calls `add_lyrics` with `{"presentation_id": "pres-nonexistent", "text": "Letra de prueba"}`
- THEN the tool MUST return `{ "error": "presentation_not_found", "message": "No se encontró la presentación solicitada" }`

---

### Requirement: Staged Track Creation Tool (add_track_from_staged)

1. `add_track_from_staged`:
   - Input arguments: `presentation_id` (required, string), `title` (required, string), and `staged_audio_id` (optional, string).
   - Validates that `presentation_id` exists in storage; returns error if missing.
   - Trims whitespace and validates that `title` is non-empty; returns validation error if empty.
   - Resolves the audio source from either:
     a) `staged_audio_id`: if provided, looks up the identifier in `StagedAudioRegistry`. If missing, expired, or already consumed, returns structured error (`staged_audio_not_found` or `audio_already_consumed`).
     b) Request-scoped staging: if `staged_audio_id` is omitted or null, validates that the current request has an active, unconsumed staged audio file handle via `StagingFileGuard`.
   - If neither source is available, returns `{ "error": "no_audio_staged", "message": "No hay archivo de audio adjunto o preparado en esta solicitud" }`.
   - If the referenced audio was already consumed, returns `{ "error": "audio_already_consumed", "message": "El archivo de audio ya fue consumido" }`.
2. When all preconditions are met:
   - Moves the staged audio file into the application's managed audio directory with a generated safe filename.
   - Marks the source handle or `staged_audio_id` as consumed so it cannot be reused.
   - Persists a new item of kind `track` with the given title and relative audio file reference at contiguous position index `N`.
   - Updates the parent presentation's `updated_at` timestamp.
   - Returns `{ "item_id": string, "kind": "track", "title": string, "position": u32, "presentation_id": string }`.
   - If database persistence fails, the staging reference MUST NOT be marked as consumed, allowing retry while still within its valid TTL.
(Previously: Consumed only request-scoped multipart attachments without staged_audio_id support)

#### Scenario: Add track from staged file with valid title and presentation
- GIVEN a multipart request with a staged 8 MiB `.mp3` file, and an existing presentation "pres-1" with 0 items
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": "Pista Base Guitarra"}`
- THEN the system MUST save the audio into managed storage, create a `track` item at position 0, mark the staged file as consumed, and return `{ "item_id": "item-...", "kind": "track", "title": "Pista Base Guitarra", "position": 0, "presentation_id": "pres-1" }`

#### Scenario: Call add_track_from_staged without staged audio file in request
- GIVEN a text-only JSON request without any audio attachment or `staged_audio_id`
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": "Pista Fantasma"}`
- THEN the tool MUST return `{ "error": "no_audio_staged", "message": "No hay archivo de audio adjunto o preparado en esta solicitud" }` and MUST NOT create an item or modify storage

#### Scenario: Reject duplicate call to add_track_from_staged in same request
- GIVEN a multipart request with one staged audio file
- WHEN the model calls `add_track_from_staged` successfully, and then issues a second `add_track_from_staged` call within the same turn
- THEN the second call MUST return `{ "error": "audio_already_consumed", "message": "El archivo de audio ya fue consumido" }` and MUST NOT create a second track item

#### Scenario: Reject add_track_from_staged with empty title
- GIVEN a valid staged audio file and existing presentation "pres-1"
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": ""}`
- THEN the tool MUST reject the call with a validation error, leave the staged file available for a retry or cleanup, and create no item

#### Scenario: Add track from staged_audio_id originating from YouTube download
- GIVEN an active `staged_audio_id: "staged-yt-123"` in `StagedAudioRegistry` and existing presentation "pres-2"
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-2", "title": "Solo Acústico", "staged_audio_id": "staged-yt-123"}`
- THEN the system MUST move the file from the staging directory into managed audio, mark "staged-yt-123" as consumed, persist the `track` item at contiguous position N, and return the item descriptor

#### Scenario: Reject add_track_from_staged with expired or already-consumed staged_audio_id
- GIVEN a `staged_audio_id: "staged-expired-999"` that has expired or was already consumed
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": "Pista Tardía", "staged_audio_id": "staged-expired-999"}`
- THEN the tool MUST return structured error `{ "error": "audio_already_consumed", "message": "El archivo de audio ya fue consumido o expiró" }` and MUST NOT create an item

---

### Requirement: Presentation Identifier Integrity and Grounded Context

1. The assistant loop MUST enforce presentation identifier integrity:
   - Tool arguments for `presentation_id` MUST only be accepted if they correspond to:
     a) The active `presentation_id` explicitly provided in the request context, OR
     b) A presentation ID returned by a `create_presentation` or `list_presentations` tool executed during the current turn.
2. If the model attempts to invoke an item creation tool with an invented or invalid ID:
   - The tool dispatcher MUST reject the operation with a structured error.
   - The assistant MUST NOT report that items were added to the phantom presentation.
3. If the user refers to a presentation by visible name rather than ID and multiple presentations share that name:
   - The assistant MUST call `list_presentations` and ask the user to clarify which presentation they meant, rather than guessing an arbitrary match.
4. Chaining and destination resolution for YouTube downloads:
   - If an active `presentation_id` is present in the request context or resolved during the turn, the agent loop MAY chain `download_youtube_track` followed by `add_track_from_staged` within the same turn.
   - If no active `presentation_id` is present and no destination presentation was resolved during the turn: after `download_youtube_track` completes successfully, the agent MUST NOT call `add_track_from_staged` with an invented ID; it MUST inform the user that the audio is ready in staging and ask which presentation to add it to.
   - If the user specifies a presentation name in a second turn and multiple presentations match, the assistant MUST list matching presentations and ask for clarification, never adding the track to an arbitrary presentation.
(Previously: Did not define chaining or destination resolution rules for download_youtube_track)

#### Scenario: Model uses presentation_id passed in request context
- GIVEN the user is viewing presentation "pres-100" and asks "Agrega una nota Recordar el solo"
- WHEN the model receives the request context with `presentation_id: "pres-100"`
- THEN the model invokes `add_note` targeting "pres-100", the tool executes successfully, and Shirley confirms the addition

#### Scenario: Model chains create_presentation ID into add_lyrics call
- GIVEN the user asks "Crea el show Festival y añádele la letra de Bienvenida"
- WHEN the model first calls `create_presentation` returning ID "pres-festival-55"
- THEN the model MUST use "pres-festival-55" in the subsequent `add_lyrics` call within the same request

#### Scenario: Model supplies fabricated presentation ID and receives clear error
- GIVEN the model calls `add_lyrics` with `{"presentation_id": "pres-invented-999", "text": "..."}`
- WHEN the tool executes
- THEN the tool returns `{ "error": "presentation_not_found", "message": "No se encontró la presentación solicitada" }`, and Shirley communicates the error to the user without claiming the lyrics were saved

#### Scenario: Same-turn chaining of download_youtube_track and add_track_from_staged when presentation is active
- GIVEN an active `presentation_id: "pres-active-10"` in the request context and a user message "Descarga este video youtube.com/watch?v=dQw4w9WgXcQ como pista"
- WHEN the agent processes the turn
- THEN the agent MUST call `download_youtube_track`, receive `staged_audio_id`, immediately call `add_track_from_staged` targeting "pres-active-10", and confirm in the final message that the track was added to the active show

#### Scenario: Two-turn flow when no presentation is active asks user for target presentation
- GIVEN no active presentation in request context (`presentation_id: null`) and a user message "Descarga youtube.com/watch?v=dQw4w9WgXcQ"
- WHEN the agent processes the turn
- THEN the agent MUST call `download_youtube_track`, MUST NOT call `add_track_from_staged`, MUST return `staged_audio_id`, and MUST reply confirming that the audio is ready and asking which presentation to add it to

---

### Requirement: Bounded Model Loop and Request Deadline

1. The agent orchestration loop MUST execute at most 5 model iterations per chat request.
2. Each model iteration consists of sending the conversation context to Groq, receiving either a final assistant message or a set of tool calls, executing local tools, and appending tool results back to the context.
3. If the loop reaches 5 iterations without producing a final assistant message:
   - The server MUST terminate the loop.
   - Any tool mutations that were already committed to the database MUST remain committed.
   - The server MUST return HTTP status 200 with an assistant message explaining that Shirley reached her turn limit, detailing the actions that were actually completed, and inviting the user to continue.
4. The entire request lifetime (including network calls to Groq and local tool operations) MUST be bounded by a global 120-second deadline (configurable via `SHIRLEY_AGENT_TIMEOUT_SECS`, defaulting to 120).
5. Subprocess operations within `download_youtube_track` MUST additionally be bounded by an internal 90-second timeout.
6. If the global request deadline is exceeded:
   - All pending network, dispatch, and subprocess operations MUST be cancelled immediately.
   - Any unconsumed request-scoped staged audio files and active download temporary files MUST be deleted from disk by the staging guard.
   - Any database operations committed before the timeout MUST remain intact.
   - The server MUST return HTTP status 200 with a clear message indicating a timeout occurred, listing confirmed committed actions in `actions_taken`.
(Previously: Bounded by a fixed global 30-second deadline without dedicated subprocess timeout)

#### Scenario: Multi-tool chain completes within iteration limit
- GIVEN a user prompt requiring 3 sequential tool executions (e.g. create show, add note, add lyrics)
- WHEN the agent executes the tools across 3 iterations and finishes on iteration 4 with a final message
- THEN the server returns HTTP status 200 with all 3 committed actions and the assistant confirmation

#### Scenario: Agent loop reaches 5-iteration limit safely
- GIVEN a complex or recursive prompt causing the model to continuously issue tool calls
- WHEN the 5th iteration completes without a terminal text message
- THEN the server MUST stop the loop, assemble `actions_taken` from the completed tools, and return a bounded summary message without looping infinitely

#### Scenario: Global request deadline cancels long-running loop after 120 seconds
- GIVEN a slow provider response or slow network conditions causing total request duration to reach 120 seconds
- WHEN the 120-second deadline expires
- THEN the server MUST cancel processing, terminate any active subprocess, purge temporary staged files, and return HTTP status 200 reporting the timeout and any committed actions

#### Scenario: Download subprocess terminates at 90s before global 120s deadline
- GIVEN a YouTube download subprocess that hangs or transfers excessively slowly
- WHEN 90 seconds elapse during the `download_youtube_track` execution
- THEN the tool MUST kill the child process, clean up partials, and return a timeout tool error back to the agent loop before the global 120-second request deadline expires

---

### Requirement: Duplicate Tool Call Prevention within Turn

1. The tool dispatcher MUST track tool call identifiers and parameter signatures within each turn.
2. If the model issues identical duplicate tool calls within the same iteration or across iterations of the same turn:
   - For mutations (`create_presentation`, `add_lyrics`, `add_note`, `add_track_from_staged`, `download_youtube_track`), the dispatcher MUST execute the mutation only once.
   - Subsequent identical calls MUST return a structured warning `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`.
3. Idempotent read-only tools (`list_presentations`, `get_presentation`) MAY be served from the initial result if repeated.
(Previously: Mutation list did not include download_youtube_track)

#### Scenario: Duplicate mutation tool call in same turn executed only once
- GIVEN the model issues two identical parallel tool calls to `add_note` with `{"presentation_id": "pres-1", "text": "Revisar afinación"}`
- WHEN the dispatcher processes the tool calls
- THEN the note MUST be created exactly once in the database, and the second call MUST return a duplicate warning instead of inserting a second duplicate note

#### Scenario: Duplicate download_youtube_track call in same turn executed only once
- GIVEN the model issues two identical tool calls to `download_youtube_track` with the same `url` within the same turn
- WHEN the dispatcher processes the second call
- THEN the dispatcher MUST NOT launch a second download subprocess and MUST return `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`

---

### Requirement: YouTube Audio Download Tool and Execution Guardrails

The system MUST provide a secure, local execution tool `download_youtube_track` to convert a single public YouTube video into a validated MP3 audio track staged for repertoire addition.

1. Parameter schema:
   - `url` (required, string): the public YouTube URL to download.
   - `title` (optional, string): custom visible title for the audio track.
2. URL validation and domain constraints:
   - The tool MUST validate and normalize the input URL before initiating any network connection or subprocess.
   - The host MUST be `youtube.com` (including `www.youtube.com`, `m.youtube.com`) or `youtu.be`. All other hosts and protocols other than HTTP/HTTPS MUST be rejected.
   - The URL MUST contain a valid 11-character video identifier matching `^[a-zA-Z0-9_-]{11}$`.
   - Supported URL formats MUST include:
     - Standard watch URLs: `https://www.youtube.com/watch?v={id}`
     - Shortened URLs: `https://youtu.be/{id}`
     - Shorts URLs: `https://www.youtube.com/shorts/{id}`
   - Any query parameters other than the video identifier (such as `list=`, `index=`, `t=`, `feature=`) MUST be stripped or ignored; the download invocation MUST enforce `--no-playlist` to prevent downloading multiple videos or playlist streams.
   - URLs representing livestreams, channels, user profiles, or URLs requiring authentication, cookies, or age verification MUST be rejected.
   - The URL string MUST NOT contain shell metacharacters (such as `;`, `&`, `|`, `$`, `` ` ``, `\`, `"`, `'`) and MUST NEVER be evaluated inside a shell interpreter.
3. Subprocess execution and infrastructure:
   - The tool MUST execute `yt-dlp` directly using structured process arguments via `tokio::process::Command`. Raw shell execution (such as `sh -c` or `bash -c`) MUST NOT be used.
   - The executable binary path MUST be resolved from the `YT_DLP_BIN` environment variable when set, falling back to resolving `yt-dlp` from the system `PATH`.
   - If the `yt-dlp` binary is missing or non-executable, or if `ffmpeg` is missing, the tool MUST return a structured, human-friendly error in Spanish explaining the missing server tool without attempting fallback shell commands.
   - Process arguments MUST include: `--extract-audio`, `--audio-format`, `mp3`, `--no-playlist`, and `--max-filesize 100M`.
   - The tool MUST enforce a strict 90-second subprocess timeout. If the process does not finish within 90 seconds, the server MUST terminate the process child immediately (kill-and-wait) and delete any partial or temporary files created during the attempt.
4. Duration and size limits:
   - The content duration MUST NOT exceed 20 minutes (1200 seconds). If metadata indicates a duration greater than 1200 seconds, or if the extracted audio exceeds this duration, the download MUST be aborted and rejected.
   - The resulting MP3 file size MUST NOT exceed 100 MiB (104,857,600 bytes). Any output exceeding this size MUST be deleted and rejected.
   - Success MUST only be confirmed upon verifying the physical existence, non-zero size, and MP3 format of the final file on disk; textual stdout alone MUST NOT be treated as proof of success.
5. Title resolution and staging:
   - If `title` is provided by the user or model, it MUST be trimmed of whitespace and validated as non-empty.
   - If `title` is absent or whitespace-only, the tool MUST derive a safe title from video metadata (trimmed and sanitized), or fall back to the 11-character video ID.
   - A successfully downloaded and validated MP3 MUST be registered in `StagedAudioRegistry` with an opaque unique `staged_audio_id`, a TTL of 15 minutes, and a single-use flag.
   - The tool MUST return `{ "staged_audio_id": string, "title": string, "duration_secs": u32, "size_bytes": u64 }`.
   - The tool output MUST NOT contain filesystem paths, command lines, or raw process stderr.
6. Error handling and cleanup:
   - All errors MUST be returned as structured tool error objects `{ "error": string, "message": string }` with clear, warm messages in Spanish.
   - On any failure (timeout, non-zero exit code, validation error, process crash), the server MUST immediately purge any partial download or temporary file.

#### Scenario: Valid standard YouTube watch URL downloads and stages MP3
- GIVEN a user prompt with `https://www.youtube.com/watch?v=dQw4w9WgXcQ` and title "Mi Pista"
- WHEN the model invokes `download_youtube_track` with that URL and title
- THEN the tool MUST execute `yt-dlp` with structured arguments, extract MP3, verify duration <= 1200s and size <= 100 MiB, register the file in `StagedAudioRegistry`, and return `{ "staged_audio_id": "staged-...", "title": "Mi Pista", "duration_secs": 212, "size_bytes": 3450000 }`

#### Scenario: Valid youtu.be short URL downloads and stages MP3
- GIVEN a user prompt with shortened URL `https://youtu.be/dQw4w9WgXcQ`
- WHEN the model invokes `download_youtube_track` with that URL
- THEN the tool MUST extract the 11-character video ID `dQw4w9WgXcQ`, download the audio via `yt-dlp`, stage the MP3, and return a valid `staged_audio_id`

#### Scenario: Valid YouTube Shorts URL downloads and stages MP3
- GIVEN a user prompt with Shorts URL `https://www.youtube.com/shorts/dQw4w9WgXcQ`
- WHEN the model invokes `download_youtube_track` with that URL
- THEN the tool MUST accept the Shorts format, extract the 11-character video ID, download the audio via `yt-dlp`, and stage the MP3

#### Scenario: URL with playlist parameters downloads only the single video
- GIVEN a URL containing both a video ID and playlist parameters `https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLrEnWoR732-DNX90-Y_wB9B`
- WHEN the model invokes `download_youtube_track`
- THEN the tool MUST pass `--no-playlist`, extract solely the single video `dQw4w9WgXcQ`, and stage exactly one MP3 track

#### Scenario: Reject URL with invalid host or non-YouTube domain
- GIVEN a URL from an unsupported platform such as `https://vimeo.com/123456` or `https://soundcloud.com/artist/track`
- WHEN the model invokes `download_youtube_track`
- THEN the tool MUST reject the request before executing any process and return `{ "error": "unsupported_url", "message": "Solo se admiten enlaces públicos de YouTube (youtube.com o youtu.be)" }`

#### Scenario: Reject malformed URL or invalid video ID
- GIVEN a URL with an invalid video ID format `https://www.youtube.com/watch?v=short`
- WHEN the model invokes `download_youtube_track`
- THEN the tool MUST reject the request without executing any process and return a validation error requiring a valid 11-character video identifier

#### Scenario: Reject URL containing command injection characters
- GIVEN a URL containing shell characters `https://www.youtube.com/watch?v=dQw4w9WgXcQ;rm -rf /`
- WHEN the model invokes `download_youtube_track`
- THEN the tool MUST reject the URL during domain validation, execute no subprocess, and return a validation error

#### Scenario: Reject video exceeding 20 minutes limit
- GIVEN a valid YouTube URL pointing to a video of duration 25 minutes (1500 seconds)
- WHEN the tool inspects metadata or extracted audio duration
- THEN the tool MUST abort the operation, delete any temporary files, and return `{ "error": "duration_exceeded", "message": "El video supera el límite máximo de 20 minutos" }`

#### Scenario: Reject download exceeding 100 MiB limit
- GIVEN a download attempt that produces an audio file larger than 100 MiB (104,857,600 bytes)
- WHEN the size validation occurs
- THEN the tool MUST abort, purge the partial file, and return `{ "error": "file_size_exceeded", "message": "El audio descargado supera el límite máximo de 100 MiB" }`

#### Scenario: Subprocess timeout at 90 seconds terminates process and purges partials
- GIVEN a slow download where `yt-dlp` has not completed after 90 seconds
- WHEN the 90-second subprocess timeout expires
- THEN the system MUST terminate the child process immediately via kill-and-wait, delete all temporary and partial files, and return `{ "error": "download_timeout", "message": "La descarga tardó demasiado tiempo y fue cancelada" }`

#### Scenario: Missing yt-dlp binary returns friendly configuration error
- GIVEN the server environment lacks `yt-dlp` or `YT_DLP_BIN` points to a non-existent file
- WHEN `download_youtube_track` is executed
- THEN the tool MUST return `{ "error": "downloader_not_configured", "message": "La herramienta de descarga no está disponible en el servidor" }` without leaking system paths or raw error traces
