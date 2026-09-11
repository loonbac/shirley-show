# Delta for Agent Tools

## ADDED Requirements

### Requirement: Vocal Separation Tool and GPU Execution Guardrails

The system MUST provide a secure, local execution tool `separate_vocals` to perform isolated vocal and instrumental separation on supported audio sources using approved pipelines, producing a validated MP3 audio track staged for repertoire addition.

1. Parameter schema:
   - `mode` (required, string enum: `"chorus"` | `"instrumental"`):
     Specifies the vocal separation pipeline to execute. Any other value or omission MUST be rejected.
   - Audio source parameters (at least one MUST be provided; if multiple are provided, priority is `staged_audio_id`, then `track_id`, then `youtube_url`):
     - `staged_audio_id` (optional, string): opaque identifier of an active, unconsumed staged audio entry in `StagedAudioRegistry`.
     - `track_id` (optional, string): identifier of an existing track item in the active show or library.
     - `youtube_url` (optional, string): public YouTube URL to download and stage prior to separation.
2. Conversational preconditions:
   - Ressy MUST have obtained clear confirmation of both `mode` (`chorus` or `instrumental`) and destination presentation before invoking `separate_vocals` or acquiring the GPU mutex.
   - If either mode or destination presentation is missing, ambiguous, or unconfirmed, the assistant MUST prompt the user for clarification and MUST NOT execute `separate_vocals` or allocate GPU resources.
3. Preflight duration validation:
   - Before acquiring the GPU mutex, the system MUST invoke `ffprobe` to verify that the source audio has a valid, readable duration and that the duration does not exceed 10 minutes (600 seconds).
   - If `ffprobe` reports a duration exceeding 600 seconds, or if duration cannot be verified, the tool MUST reject the operation immediately, purge any temporary files, and return a friendly error in Spanish without acquiring the GPU.
4. GPU Exclusivity and Global Mutex:
   - A single global mutex/semaphore with capacity 1 MUST serialize all separation operations across the entire application.
   - At most one vocal separation process MUST run on the GPU at any given time.
   - If the GPU is currently occupied by another separation process, incoming separation requests MUST wait for the mutex up to the request deadline.
   - The GPU mutex MUST be released deterministically upon success, error, timeout, cancellation, client disconnect, or process panic.
5. Subprocess execution and infrastructure:
   - The tool MUST execute `karaoke-separator` using structured process arguments via `tokio::process::Command`. Raw shell evaluation (such as `sh -c` or `bash -c`) MUST NOT be used.
   - The binary path MUST be resolved from `VOCAL_SEP_BIN` when set in the environment, falling back to resolving `karaoke-separator` from the system `PATH`.
   - If `VOCAL_SEP_BIN` is missing, unreadable, or not executable, the tool MUST return a friendly, actionable error in Spanish explaining the missing server configuration without attempting fallback shell commands.
6. Pipeline `chorus`:
   - MUST execute sequentially: `gabox-invertido`, `frazer-becruily`, and `anvuew` on the same original audio under the single GPU lock.
   - Execution MUST take place inside a dedicated temporary directory created specifically for the operation using the output template `NAME_MODEL_STEM`.
   - Output stem resolution MUST select the karaoke/non-vocal stem from each model's output directory and MUST strictly exclude any file matching case-insensitive `(?i)vocal`.
   - Combining via `AVG_WAVE`:
     - Reads the three extracted karaoke WAV files using `hound`.
     - Validates that all three stems share identical sample rate, channel count, and sample length.
     - Computes float PCM average with fixed equal weights `1:1:1` (`(sample_a + sample_b + sample_c) / 3.0`).
     - Clamps float output samples to `[-1.0, 1.0]` before writing 16-bit or 24-bit PCM WAV.
     - If any of the three stems is missing, extra, unreadable, or has mismatched sample rate, channel count, or length, the entire operation MUST fail; NO partial average or model fallback is permitted.
7. Pipeline `instrumental`:
   - MUST execute ONLY the PolarFormer model (`model_bs_polarformer_float16.ckpt` and its associated YAML configuration).
   - Preflight check: MUST verify the physical existence and readability of the PolarFormer checkpoint and YAML configuration file before acquiring the GPU mutex.
   - If the checkpoint or YAML is missing, the tool MUST return a friendly, actionable error in Spanish stating that instrumental mode is not currently configured/available on the server, suggesting chorus mode or retaining the staged audio within its TTL.
   - Strict invariant: The system MUST NOT attempt fallback to Demucs, Leap, chorus models, or cascades.
8. Output transcoding and staging:
   - The output WAV from `AVG_WAVE` (chorus) or PolarFormer (instrumental) MUST be transcoded to CBR 320 kbps MP3 using `ffmpeg`.
   - The resulting MP3 file MUST be verified for physical existence and non-zero size.
   - The validated MP3 MUST be registered in `StagedAudioRegistry` with an opaque `staged_audio_id`, a TTL of 15 minutes, and a single-use flag.
   - The tool MUST return `{ "staged_audio_id": string, "mode": "chorus" | "instrumental", "title": string, "duration_secs": u32, "size_bytes": u64 }`.
   - Output MUST NOT contain filesystem paths, shell commands, or raw process stderr.
9. Process guard and cleanup (`VocalProcessGuard`):
   - Subprocesses and temporary directories MUST be managed by a deterministic guard (`VocalProcessGuard`).
   - On any error, timeout, cancellation, client disconnect, or failure, all child processes MUST be terminated immediately via kill-and-wait, the GPU mutex released, and all partial temporary directories purged from disk.

#### Scenario: Valid chorus separation executes 3 models and blends stems via AVG_WAVE into 320k MP3
- GIVEN a valid staged audio file of 3 minutes duration, user requested mode "chorus", and destination presentation "pres-1"
- WHEN the model invokes `separate_vocals` with `{"mode": "chorus", "staged_audio_id": "staged-src-1"}`
- THEN the system MUST acquire the GPU mutex, execute `gabox-invertido`, `frazer-becruily`, and `anvuew` sequentially, combine their karaoke stems with `AVG_WAVE 1:1:1`, transcode the result to CBR 320 kbps MP3 via `ffmpeg`, register the MP3 in `StagedAudioRegistry`, release the GPU mutex, and return `{ "staged_audio_id": "staged-vocal-1", "mode": "chorus", "title": "...", "duration_secs": 180, "size_bytes": 7200000 }`

#### Scenario: Valid instrumental separation with PolarFormer produces 320k MP3
- GIVEN an installed and verified PolarFormer checkpoint and YAML, a valid source audio under 10 minutes, and user requested mode "instrumental"
- WHEN the model invokes `separate_vocals` with `{"mode": "instrumental", "staged_audio_id": "staged-src-2"}`
- THEN the system MUST acquire the GPU mutex, execute PolarFormer, select the non-vocal stem, transcode to CBR 320 kbps MP3, register the MP3 in `StagedAudioRegistry`, release the GPU mutex, and return a success descriptor with mode "instrumental"

#### Scenario: PolarFormer checkpoint missing returns friendly error without fallback
- GIVEN the server environment lacks `model_bs_polarformer_float16.ckpt` or its YAML configuration
- WHEN the model invokes `separate_vocals` with `mode: "instrumental"`
- THEN the tool MUST reject the operation before acquiring the GPU mutex, MUST NOT execute any fallback model (such as Demucs, Leap, or chorus models), and MUST return a friendly, actionable error explaining that instrumental mode is not currently configured on the server

#### Scenario: Reject audio exceeding 10 minutes limit before acquiring GPU
- GIVEN a source audio file with a duration of 11 minutes (660 seconds)
- WHEN `separate_vocals` performs the duration preflight check via `ffprobe`
- THEN the tool MUST abort immediately before acquiring the GPU mutex, purge any temporary directories, and return `{ "error": "duration_exceeded", "message": "El audio supera el límite máximo de 10 minutos para separación vocal" }`

#### Scenario: Reject audio when duration cannot be verified by ffprobe
- GIVEN a corrupted or unreadable audio source where `ffprobe` cannot extract a valid duration
- WHEN `separate_vocals` performs the preflight check
- THEN the tool MUST reject the request before acquiring the GPU mutex and return `{ "error": "invalid_audio_duration", "message": "No se pudo verificar la duración del audio fuente" }`

#### Scenario: Audio source from staged_audio_id separates successfully
- GIVEN an active `staged_audio_id: "staged-yt-10"` in `StagedAudioRegistry`
- WHEN `separate_vocals` is called with `{"mode": "chorus", "staged_audio_id": "staged-yt-10"}`
- THEN the tool MUST resolve the source audio from `StagedAudioRegistry`, perform separation, stage the resulting MP3, and return a new `staged_audio_id`

#### Scenario: Audio source from existing track_id separates successfully
- GIVEN an existing track item "track-42" with stored audio in presentation "pres-1"
- WHEN `separate_vocals` is called with `{"mode": "chorus", "track_id": "track-42"}`
- THEN the tool MUST locate the managed audio file for "track-42", execute the chorus separation pipeline, stage the resulting MP3, and return a new `staged_audio_id`

#### Scenario: Audio source from youtube_url downloads and separates successfully
- GIVEN a user prompt with a YouTube URL to separate
- WHEN `separate_vocals` is called with `{"mode": "chorus", "youtube_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"}`
- THEN the tool MUST download the YouTube audio into staging, validate duration <= 10 minutes, execute the chorus pipeline, stage the 320k MP3 result, and return the new `staged_audio_id`

#### Scenario: Reject missing or invalid mode parameter
- GIVEN a call to `separate_vocals` with `mode: "acapella"` or missing `mode`
- WHEN the dispatcher validates arguments
- THEN the tool MUST reject the call with a validation error stating that `mode` must be either "chorus" or "instrumental"

#### Scenario: Inconsistent stems in sample rate, channels, or length aborts AVG_WAVE and cleans up
- GIVEN the three chorus models produce WAV stems where one stem has a sample rate of 48000 Hz and the others have 44100 Hz, or where channel counts differ
- WHEN `AVG_WAVE` validates the stems
- THEN `AVG_WAVE` MUST reject the combination, abort the pipeline, purge all temporary stems, release the GPU mutex, and return a structured processing error without producing an audio output

#### Scenario: GPU busy waits for mutex or notifies queue
- GIVEN a separation process currently running on the GPU holding the global mutex
- WHEN a second separation request arrives
- THEN the second request MUST wait for the GPU mutex; if the wait exceeds the request deadline, it MUST return `{ "error": "gpu_busy", "message": "El procesador de audio está ocupado con otra separación. Por favor, intenta de nuevo en unos minutos" }`

#### Scenario: Separation timeout at 600 seconds terminates processes, purges partials, and frees GPU mutex
- GIVEN a long separation operation where total turn duration reaches 600 seconds
- WHEN the 600-second deadline expires
- THEN `VocalProcessGuard` MUST terminate any active child processes immediately via kill-and-wait, release the GPU mutex, purge all temporary working files, and return a timeout response

#### Scenario: Client disconnect or cancellation triggers kill-and-wait and purges temp directory
- GIVEN a separation operation actively processing on the GPU
- WHEN the client closes the connection or navigates away
- THEN `VocalProcessGuard` MUST abort execution, terminate child processes immediately, release the GPU mutex, and delete the operation's temporary directory from disk

#### Scenario: Missing or non-executable VOCAL_SEP_BIN returns friendly configuration error
- GIVEN `VOCAL_SEP_BIN` points to a missing path or `karaoke-separator` is not installed
- WHEN `separate_vocals` is invoked
- THEN the tool MUST return `{ "error": "separator_not_configured", "message": "La herramienta de separación vocal no está disponible en el servidor" }` without leaking internal paths or executing raw shell fallback

---

## MODIFIED Requirements

### Requirement: Fixed Tool Allowlist and Schema Definition

1. The agent system MUST expose exactly eight local tools to the model:
   - `create_presentation`
   - `list_presentations`
   - `get_presentation`
   - `add_lyrics`
   - `add_note`
   - `add_track_from_staged`
   - `download_youtube_track`
   - `separate_vocals`
2. The agent system MUST NOT expose any tool for deleting, renaming, editing, or reordering presentations or items, nor any tool for audio playback, Stage Mode manipulation, filesystem access, arbitrary shell execution, arbitrary HTTP requests, or raw SQL queries.
3. If the model invokes an unknown tool or a tool not present in the allowlist:
   - The dispatcher MUST NOT execute any system action.
   - The dispatcher MUST return a structured tool error `{"error": "unknown_tool", "message": "Herramienta no permitida"}` back to the model within the loop.
4. Tool parameter parsing and validation MUST occur strictly on the server before invoking domain operations.
(Previously: Allowed exactly six tools; excluded download_youtube_track and separate_vocals)

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

#### Scenario: Model calls separate_vocals with valid parameters
- GIVEN a user prompt asking Shirley to separate vocals from a staged audio file in chorus mode
- WHEN the model calls `separate_vocals` with `{"mode": "chorus", "staged_audio_id": "staged-123"}`
- THEN the tool dispatcher MUST accept the call as an authorized tool and execute the vocal separation pipeline

---

### Requirement: Staged Track Creation Tool (add_track_from_staged)

1. `add_track_from_staged`:
   - Input arguments: `presentation_id` (required, string), `title` (required, string), and `staged_audio_id` (optional, string).
   - Validates that `presentation_id` exists in storage; returns error if missing.
   - Trims whitespace and validates that `title` is non-empty; returns validation error if empty.
   - Resolves the audio source from either:
     a) `staged_audio_id`: if provided, looks up the identifier in `StagedAudioRegistry` (originating from YouTube download or vocal separation). If missing, expired, or already consumed, returns structured error (`staged_audio_not_found` or `audio_already_consumed`).
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
(Previously: Consumed only request-scoped multipart attachments without staged_audio_id support from downloads or vocal separation)

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

#### Scenario: Add track from staged_audio_id originating from vocal separation
- GIVEN an active `staged_audio_id: "staged-sep-321"` produced by `separate_vocals` in `StagedAudioRegistry` and existing presentation "pres-2"
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-2", "title": "Pista (Coros)", "staged_audio_id": "staged-sep-321"}`
- THEN the system MUST move the file from staging into managed audio, mark "staged-sep-321" as consumed, persist the `track` item at contiguous position N, and return the item descriptor

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
4. Chaining and destination resolution for vocal separation:
   - Ressy MUST confirm both mode and destination presentation before starting GPU execution.
   - If an active `presentation_id` is present in the request context or resolved during the turn, the agent loop MAY chain `separate_vocals` followed by `add_track_from_staged` within the same turn.
   - If no active `presentation_id` is present and no destination presentation was confirmed during the turn: the agent MUST NOT call `separate_vocals` at all and MUST NOT acquire the GPU mutex; it MUST ask the user which presentation the separated track should go to (and which mode, if still unknown) before spending GPU time.
   - If the user specifies a presentation name in a second turn and multiple presentations match, the assistant MUST list matching presentations and ask for clarification, never adding the track to an arbitrary presentation.
(Previously: Did not define chaining or destination resolution rules for separate_vocals)

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

#### Scenario: Same-turn chaining of separate_vocals and add_track_from_staged when presentation is active
- GIVEN an active `presentation_id: "pres-active-10"`, user requested mode "chorus", and a valid staged source audio
- WHEN the agent processes the turn
- THEN the agent MUST call `separate_vocals`, receive `staged_audio_id`, immediately call `add_track_from_staged` targeting "pres-active-10", and confirm in the final message that the separated track was added to the active show

#### Scenario: Two-turn flow when destination is unconfirmed asks first and never runs GPU
- GIVEN no active presentation in request context (`presentation_id: null`), mode "chorus" selected, and source audio staged
- WHEN the agent processes the turn
- THEN the agent MUST NOT call `separate_vocals`, MUST NOT acquire the GPU mutex, and MUST reply asking which presentation the separated track should go to

---

### Requirement: Bounded Model Loop and Request Deadline

1. The agent orchestration loop MUST execute at most 5 model iterations per chat request.
2. Each model iteration consists of sending the conversation context to Groq, receiving either a final assistant message or a set of tool calls, executing local tools, and appending tool results back to the context.
3. If the loop reaches 5 iterations without producing a final assistant message:
   - The server MUST terminate the loop.
   - Any tool mutations that were already committed to the database MUST remain committed.
   - The server MUST return HTTP status 200 with an assistant message explaining that Shirley reached her turn limit, detailing the actions that were actually completed, and inviting the user to continue.
4. Request lifetime and deadline bounds:
   - Standard conversational turns (including presentations, notes, lyrics, and uploads) MUST be bounded by a global 30-second deadline.
   - Turns executing `download_youtube_track` MUST be bounded by a global 120-second deadline (configurable via `SHIRLEY_AGENT_TIMEOUT_SECS`).
   - Turns executing `separate_vocals` MUST be bounded by an extended 600-second deadline (configurable via `SHIRLEY_SEPARATION_TIMEOUT_SECS`, defaulting to 600 seconds).
5. If the applicable request deadline is exceeded:
   - All pending network, dispatch, and subprocess operations MUST be cancelled immediately via kill-and-wait.
   - The GPU mutex MUST be released if held.
   - Any unconsumed request-scoped staged audio files, temporary download files, and partial separation files MUST be purged from disk by the guards.
   - Any database operations committed before the timeout MUST remain intact.
   - The server MUST return HTTP status 200 with a clear message indicating a timeout occurred, listing confirmed committed actions in `actions_taken`.
(Previously: Bounded by a fixed global 30-second deadline without separation 600s deadline)

#### Scenario: Multi-tool chain completes within iteration limit
- GIVEN a user prompt requiring 3 sequential tool executions (e.g. create show, add note, add lyrics)
- WHEN the agent executes the tools across 3 iterations and finishes on iteration 4 with a final message
- THEN the server returns HTTP status 200 with all 3 committed actions and the assistant confirmation

#### Scenario: Agent loop reaches 5-iteration limit safely
- GIVEN a complex or recursive prompt causing the model to continuously issue tool calls
- WHEN the 5th iteration completes without a terminal text message
- THEN the server MUST stop the loop, assemble `actions_taken` from the completed tools, and return a bounded summary message without looping infinitely

#### Scenario: Global request deadline cancels long-running loop after deadline
- GIVEN a slow provider response or network latency causing a non-separation turn to exceed its deadline
- WHEN the deadline expires
- THEN the server MUST cancel processing, purge temporary staged files, and return HTTP status 200 reporting the timeout and any partially committed actions

#### Scenario: Extended 600-second deadline applies specifically to vocal separation turn
- GIVEN a conversational turn that executes `separate_vocals` with multiple models
- WHEN the separation takes 180 seconds to complete
- THEN the server MUST NOT terminate the request at 30 or 120 seconds, and MUST allow the request to run up to 600 seconds before enforcing the deadline

#### Scenario: Vocal separation subprocess timeout at 600s terminates process and frees GPU mutex
- GIVEN a vocal separation subprocess or queue wait reaching 600 seconds
- WHEN the 600-second deadline expires
- THEN the system MUST terminate child processes via kill-and-wait, release the GPU mutex, purge temporary working files, and return HTTP status 200 reporting the timeout

---

### Requirement: Duplicate Tool Call Prevention within Turn

1. The tool dispatcher MUST track tool call identifiers and parameter signatures within each turn.
2. If the model issues identical duplicate tool calls within the same iteration or across iterations of the same turn:
   - For mutations (`create_presentation`, `add_lyrics`, `add_note`, `add_track_from_staged`, `download_youtube_track`, `separate_vocals`), the dispatcher MUST execute the mutation only once.
   - Subsequent identical calls MUST return a structured warning `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`.
3. Idempotent read-only tools (`list_presentations`, `get_presentation`) MAY be served from the initial result if repeated.
(Previously: Mutation list did not include separate_vocals)

#### Scenario: Duplicate mutation tool call in same turn executed only once
- GIVEN the model issues two identical parallel tool calls to `add_note` with `{"presentation_id": "pres-1", "text": "Revisar afinación"}`
- WHEN the dispatcher processes the tool calls
- THEN the note MUST be created exactly once in the database, and the second call MUST return a duplicate warning instead of inserting a second duplicate note

#### Scenario: Duplicate separate_vocals call in same turn executed only once
- GIVEN the model issues two identical tool calls to `separate_vocals` with the same `mode` and source within the same turn
- WHEN the dispatcher processes the second call
- THEN the dispatcher MUST NOT launch a second separation pipeline or acquire a second GPU lock and MUST return `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`
