# Agent Tools Specification

## Purpose

Defines the bounded local tool allowlist, parameter schemas, presentation ID integrity, execution dispatch, iteration limits, 30-second global deadline, and safety invariants for Shirley's tool-calling loop in Shirley Show.

## Requirements

### Requirement: Fixed Tool Allowlist and Schema Definition

1. The agent system MUST expose exactly six local tools to the model:
   - `create_presentation`
   - `list_presentations`
   - `get_presentation`
   - `add_lyrics`
   - `add_note`
   - `add_track_from_staged`
2. The agent system MUST NOT expose any tool for deleting, renaming, editing, or reordering presentations or items, nor any tool for audio playback, Stage Mode manipulation, filesystem access, shell execution, arbitrary HTTP requests, or raw SQL queries.
3. If the model invokes an unknown tool or a tool not present in the allowlist:
   - The dispatcher MUST NOT execute any system action.
   - The dispatcher MUST return a structured tool error `{"error": "unknown_tool", "message": "Herramienta no permitida"}` back to the model within the loop.
4. Tool parameter parsing and validation MUST occur strictly on the server before invoking domain operations.

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
   - Input arguments: `presentation_id` (required, string) and `title` (required, string).
   - Validates that `presentation_id` exists in storage; returns error if missing.
   - Trims whitespace and validates that `title` is non-empty; returns validation error if empty.
   - Validates that the current request has an active, unconsumed staged audio file handle.
   - If no audio file was attached to the request, returns `{ "error": "no_audio_staged", "message": "No hay archivo de audio adjunto en esta solicitud" }`.
   - If the staged audio file was already consumed by an earlier call in the same request, returns `{ "error": "audio_already_consumed", "message": "El archivo de audio adjunto ya fue consumido" }`.
2. When all preconditions are met:
   - Moves the staged audio file into the application's managed audio directory with a generated safe filename.
   - Marks the staged file handle as consumed so it cannot be reused.
   - Persists a new item of kind `track` with the given title and relative audio file reference at contiguous position index `N`.
   - Updates the parent presentation's `updated_at` timestamp.
   - Returns `{ "item_id": string, "kind": "track", "title": string, "position": u32, "presentation_id": string }`.

#### Scenario: Add track from staged file with valid title and presentation
- GIVEN a multipart request with a staged 8 MiB `.mp3` file, and an existing presentation "pres-1" with 0 items
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": "Pista Base Guitarra"}`
- THEN the system MUST save the audio into managed storage, create a `track` item at position 0, mark the staged file as consumed, and return `{ "item_id": "item-...", "kind": "track", "title": "Pista Base Guitarra", "position": 0, "presentation_id": "pres-1" }`

#### Scenario: Call add_track_from_staged without staged audio file in request
- GIVEN a text-only JSON request without any audio attachment
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": "Pista Fantasma"}`
- THEN the tool MUST return `{ "error": "no_audio_staged", "message": "No hay archivo de audio adjunto en esta solicitud" }` and MUST NOT create an item or modify storage

#### Scenario: Reject duplicate call to add_track_from_staged in same request
- GIVEN a multipart request with one staged audio file
- WHEN the model calls `add_track_from_staged` successfully, and then issues a second `add_track_from_staged` call within the same turn
- THEN the second call MUST return `{ "error": "audio_already_consumed", "message": "El archivo de audio adjunto ya fue consumido" }` and MUST NOT create a second track item

#### Scenario: Reject add_track_from_staged with empty title
- GIVEN a valid staged audio file and existing presentation "pres-1"
- WHEN the model calls `add_track_from_staged` with `{"presentation_id": "pres-1", "title": ""}`
- THEN the tool MUST reject the call with a validation error, leave the staged file available for a retry or cleanup, and create no item

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

---

### Requirement: Bounded Model Loop and Request Deadline

1. The agent orchestration loop MUST execute at most 5 model iterations per chat request.
2. Each model iteration consists of sending the conversation context to Groq, receiving either a final assistant message or a set of tool calls, executing local tools, and appending tool results back to the context.
3. If the loop reaches 5 iterations without producing a final assistant message:
   - The server MUST terminate the loop.
   - Any tool mutations that were already committed to the database MUST remain committed.
   - The server MUST return HTTP status 200 with an assistant message explaining that Shirley reached her turn limit, detailing the actions that were actually completed, and inviting the user to continue.
4. The entire request lifetime (including network calls to Groq and local tool operations) MUST be bounded by a global 30-second deadline.
5. If the 30-second deadline is exceeded:
   - All pending network and dispatch operations MUST be cancelled immediately.
   - Any unconsumed staged audio file MUST be deleted from disk by the staging guard.
   - Any database operations committed before the timeout MUST remain intact.
   - The server MUST return HTTP status 200 with a clear message indicating a timeout occurred, listing confirmed committed actions in `actions_taken`.

#### Scenario: Multi-tool chain completes within iteration limit
- GIVEN a user prompt requiring 3 sequential tool executions (e.g. create show, add note, add lyrics)
- WHEN the agent executes the tools across 3 iterations and finishes on iteration 4 with a final message
- THEN the server returns HTTP status 200 with all 3 committed actions and the assistant confirmation

#### Scenario: Agent loop reaches 5-iteration limit safely
- GIVEN a complex or recursive prompt causing the model to continuously issue tool calls
- WHEN the 5th iteration completes without a terminal text message
- THEN the server MUST stop the loop, assemble `actions_taken` from the completed tools, and return a bounded summary message without looping infinitely

#### Scenario: Global request deadline cancels long-running loop after 30 seconds
- GIVEN a slow provider response or high network latency causing the request duration to reach 30 seconds
- WHEN the 30-second deadline expires
- THEN the server MUST cancel processing, purge temporary staged files, and return a response reporting the timeout and any partially committed actions

---

### Requirement: Duplicate Tool Call Prevention within Turn

1. The tool dispatcher MUST track tool call identifiers and parameter signatures within each turn.
2. If the model issues identical duplicate tool calls within the same iteration or across iterations of the same turn:
   - For mutations (`create_presentation`, `add_lyrics`, `add_note`, `add_track_from_staged`), the dispatcher MUST execute the mutation only once.
   - Subsequent identical calls MUST return a structured warning `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`.
3. Idempotent read-only tools (`list_presentations`, `get_presentation`) MAY be served from the initial result if repeated.

#### Scenario: Duplicate mutation tool call in same turn executed only once
- GIVEN the model issues two identical parallel tool calls to `add_note` with `{"presentation_id": "pres-1", "text": "Revisar afinación"}`
- WHEN the dispatcher processes the tool calls
- THEN the note MUST be created exactly once in the database, and the second call MUST return a duplicate warning instead of inserting a second duplicate note
