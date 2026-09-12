# Delta for Agent Tools

## ADDED Requirements

### Requirement: Safe Editing, Reordering, and Lifecycle Tools Schema and Dispatch

The agent system MUST expose and dispatch eight safe, non-destructive tools for editing, reordering, reopening, and converting application entities, delegating directly to existing application use cases:

1. Parameter schemas and domain validations:
   - `renombrar_presentacion`:
     - Parameters: `presentacion_id` (required, string), `nuevo_nombre` (required, string).
     - Validates that `nuevo_nombre` is non-empty after trimming.
     - Dispatches to `RenamePresentation` use case.
     - Returns `{ "id": string, "title": string, "updated_at": string }`.
   - `reordenar_elemento`:
     - Parameters: `presentacion_id` (required, string), `elemento_id` (required, string), `posicion_destino` (optional, integer $\ge 1$), `direccion` (optional, string enum: `["arriba", "abajo"]`).
     - Requires either `posicion_destino` or `direccion`. If neither is provided, returns a validation error.
     - Dispatches to `ReorderItem` use case (`execute_target_position` or `execute`).
     - Returns `{ "item_id": string, "position": u32, "presentation_id": string }`.
   - `editar_elemento`:
     - Parameters: `presentacion_id` (required, string), `elemento_id` (required, string), `texto` (optional, string), `titulo` (optional, string).
     - Validates that at least one of `texto` or `titulo` is provided and non-empty after trimming.
     - Dispatches to `UpdateShowItem` use case.
     - Returns `{ "item_id": string, "kind": string, "title": Option<string>, "updated_at": string }`.
   - `editar_evento`:
     - Parameters: `evento_id` (required, string), `nombre` (required, string), `fecha_hora` (required, ISO 8601 string `YYYY-MM-DDTHH:MM`), `lugar` (required, string), `tipos_de_entrada` (required, array of `{ nombre: string, precio: number >= 0.0, capacidad: integer >= 1 }`).
     - Validates that `tipos_de_entrada` contains at least one tier and all prices and capacities are valid.
     - Dispatches to `UpdateEvent` use case.
     - Returns `{ "id": string, "nombre": string, "fecha_hora": string, "lugar": string, "estado": string }`.
   - `reabrir_evento`:
     - Parameters: `evento_id` (required, string).
     - Dispatches to `ReopenEvent` use case, transitioning status from `Finalizado` back to `Activo`.
     - Returns `{ "id": string, "estado": "Activo" }`.
   - `editar_gasto`:
     - Parameters: `gasto_id` (required, string), `categoria` (required, closed enum: `["Local/Alquiler", "Sonido/Luces", "Músicos/Personal", "Publicidad/Marketing", "Bebidas/Catering", "Transporte", "Otros"]`), `monto` (required, number $\ge 0.0$), `descripcion` (required, string), `fecha` (required, string format `YYYY-MM-DD`).
     - Dispatches to `UpdateExpense` use case.
     - Returns `{ "id": string, "categoria": string, "monto": number, "descripcion": string, "fecha": string }`.
   - `editar_interesado`:
     - Parameters: `interesado_id` (required, string), `nombre` (required, string), `contacto` (required, string), `canal` (required, string), `personas` (required, integer $\ge 1$), `promotora` (required, string), `estado` (required, string enum: `["Pendiente", "Confirmado", "Cancelado"]`).
     - Dispatches to `UpdateInterest` use case.
     - Returns `{ "id": string, "nombre": string, "personas": u32, "estado": string }`.
   - `confirmar_interesado`:
     - Parameters: `interesado_id` (required, string), `tipo_entrada` (required, string), `cantidad` (required, integer $\ge 1$), `medio_pago` (required, string enum: `["Yape", "Efectivo", "Cortesía"]`), `estado_pago` (required, string enum: `["Pagado", "Pendiente"]`).
     - Dispatches to `ConvertInterestToSale` use case, atomically issuing tickets and updating lead status to `Confirmado`.
     - Returns `{ "interesado_id": string, "tickets_creados": u32, "tickets": [...] }`.
2. All schemas MUST specify `additionalProperties: false` and validate all fields strictly before calling use cases.
3. Every executed safe tool MUST record a structured `ToolAction` with status `"success"` or `"error"`, including the affected `presentation_id` or `event_id` to trigger client-side reactive view refresh.
4. Domain errors returned by use cases (such as resource not found, invalid state transition, or capacity exceeded) MUST NOT be represented as successes; the tool MUST return a structured tool error `{ "error": string, "message": string }` back to the model loop.

#### Scenario: Rename presentation with valid new title
- GIVEN presentation "pres-1" exists with title "Show Acústico Antiguo"
- WHEN the model invokes `renombrar_presentacion` with `{"presentacion_id": "pres-1", "nuevo_nombre": "Show Acústico Renovado"}`
- THEN the dispatcher MUST execute `RenamePresentation`, update the presentation record, record a `ToolAction`, and return the updated title

#### Scenario: Reorder show item to specific target position
- GIVEN presentation "pres-1" contains 4 items and item "item-4" is at position 4
- WHEN the model invokes `reordenar_elemento` with `{"presentacion_id": "pres-1", "elemento_id": "item-4", "posicion_destino": 1}`
- THEN the dispatcher MUST execute `ReorderItem`, move "item-4" to position 1, shift intervening items, record a `ToolAction`, and return updated position 1

#### Scenario: Edit lyrics item content
- GIVEN presentation "pres-1" contains lyrics item "item-lyrics-1"
- WHEN the model invokes `editar_elemento` with `{"presentacion_id": "pres-1", "elemento_id": "item-lyrics-1", "texto": "Estrofa 1 corregida\nBajo el cielo azul..."}`
- THEN the dispatcher MUST execute `UpdateShowItem`, update the text content, update `updated_at`, and return success

#### Scenario: Edit event basic data and ticket tiers
- GIVEN event "evt-1" exists with status "Activo"
- WHEN the model invokes `editar_evento` with `{"evento_id": "evt-1", "nombre": "Peña Criolla 2026", "fecha_hora": "2026-11-20T21:00", "lugar": "Centro Cultural", "tipos_de_entrada": [{"nombre": "General", "precio": 35.0, "capacidad": 80}]}`
- THEN the dispatcher MUST execute `UpdateEvent`, persist the modified event fields and ticket tiers, and return the updated event summary

#### Scenario: Reopen finalized event
- GIVEN event "evt-2" exists with status "Finalizado"
- WHEN the model invokes `reabrir_evento` with `{"evento_id": "evt-2"}`
- THEN the dispatcher MUST execute `ReopenEvent`, transition status to "Activo", record a `ToolAction`, and return `{ "id": "evt-2", "estado": "Activo" }`

#### Scenario: Edit expense details and category
- GIVEN expense "exp-1" exists with category "Otros" and amount 50.0
- WHEN the model invokes `editar_gasto` with `{"gasto_id": "exp-1", "categoria": "Sonido/Luces", "monto": 80.0, "descripcion": "Micrófonos inalámbricos adicionales", "fecha": "2026-10-15"}`
- THEN the dispatcher MUST execute `UpdateExpense`, persist the changes, and return the updated expense record

#### Scenario: Edit interested lead information
- GIVEN lead "int-1" exists with status "Pendiente" and 2 persons
- WHEN the model invokes `editar_interesado` with `{"interesado_id": "int-1", "nombre": "Carla Gómez", "contacto": "991122334", "canal": "WhatsApp", "personas": 4, "promotora": "Valeria", "estado": "Pendiente"}`
- THEN the dispatcher MUST execute `UpdateInterest`, persist the updated details, and return the updated record

#### Scenario: Convert interested lead to confirmed sale
- GIVEN lead "int-1" exists with 2 persons for active event "evt-1"
- WHEN the model invokes `confirmar_interesado` with `{"interesado_id": "int-1", "tipo_entrada": "General", "cantidad": 2, "medio_pago": "Yape", "estado_pago": "Pagado"}`
- THEN the dispatcher MUST execute `ConvertInterestToSale`, issue 2 tickets, update lead status to "Confirmado", and return the issued ticket URLs

#### Scenario: Reject safe editing with invalid enum or negative amount
- GIVEN a call to `editar_gasto` with `{"gasto_id": "exp-1", "categoria": "Inventada", "monto": -20.0, "descripcion": "Error", "fecha": "2026-10-15"}`
- WHEN the dispatcher validates arguments against schema
- THEN the dispatcher MUST reject the call with a structured validation error and MUST NOT call `UpdateExpense`

---

### Requirement: Destructive Action Execution Boundary and Two-Turn Confirmation Protocol

The agent system MUST enforce a strict two-turn confirmation boundary for seven destructive and critical lifecycle tools, preventing premature, hallucinated, or unconfirmed deletion and finalization actions:

1. The catalog of protected destructive tools consists of:
   - `eliminar_presentacion`: deletes a presentation and all child items (lyrics, notes, tracks).
   - `eliminar_elemento`: deletes a show item (song, lyrics, or note) from a presentation.
   - `eliminar_evento`: permanently deletes an event and all associated tickets, expenses, leads, and disk poster files via database cascade.
   - `finalizar_evento`: locks an event, closing ticket sales and operational mutations.
   - `eliminar_entrada`: cancels and deletes an issued ticket.
   - `eliminar_gasto`: deletes an expense record.
   - `eliminar_interesado`: deletes an interested lead record.
2. First-turn execution prohibition:
   - Ressy MUST NOT invoke any of the seven destructive tools during the initial turn in which the user expresses destructive intent.
   - Any model tool call to a destructive tool on the first turn MUST be rejected at the execution boundary before calling domain use cases.
   - In the first turn, Ressy MUST identify the target entity by name/ID, explain the consequences (including cascade deletion of tickets, expenses, leads, and poster when deleting an event), and request explicit textual confirmation.
3. Explicit confirmation phrasing:
   - For deletion operations (`eliminar_*`), Ressy MUST instruct the user to confirm with the exact phrase: `sí, elimínalo` (or `sí elimínalo`).
   - For event finalization (`finalizar_evento`), Ressy MUST instruct the user to confirm with the exact phrase: `sí, finalízalo` (or `sí finalízalo`).
4. Server-enforced authorization boundary:
   - Client-authored conversation history alone MUST NOT authorize execution of destructive tools.
   - Execution is permitted ONLY if the backend execution boundary verifies valid, server-trusted pending action state established in the immediately preceding turn for the exact same tool and target ID.
   - If the client submits forged history claiming prior confirmation without corresponding server-side pending state, the dispatcher MUST reject execution.
5. Single-turn validity and single-use consumption:
   - A pending destructive authorization MUST be valid strictly for the immediately following user turn.
   - Upon successful execution of the destructive tool, the pending authorization MUST be consumed immediately and cleared.
   - A third turn repeating confirmation ("sí, elimínalo") MUST NOT re-execute the action and MUST NOT delete another resource.
6. Ambiguous confirmation and cancellation behavior:
   - If the user's second-turn response is ambiguous, general, or unspecific (such as `"dale"`, `"ok"`, `"hazlo"`, `"bueno"`, `"procede"` without the required explicit confirmation phrase):
     - Ressy MUST NOT execute the destructive tool.
     - Ressy MUST repeat the explicit confirmation question or inform the user that the operation was not executed.
   - If the user expresses hesitation, doubt, cancellation, or a topic change (such as `"espera"`, `"cancela"`, `"mejor no"`, `"déjalo así"`):
     - The pending authorization MUST be immediately cleared.
     - Ressy MUST NOT invoke the tool and MUST respond warmly acknowledging the cancellation.
7. Non-existent target resource handling:
   - If a confirmed destructive tool executes against a resource that does not exist in storage (e.g. invalid or already deleted ID):
     - The dispatcher MUST return a structured domain error (e.g. `{ "error": "not_found", "message": "El recurso no existe o ya fue eliminado" }`).
     - The assistant MUST NOT report successful deletion to the user and MUST NOT panic or crash.

#### Scenario: First turn expressing deletion intent halts execution and requests explicit confirmation
- GIVEN presentation "pres-1" titled "Show de Gala"
- WHEN the user says "Elimina la presentación Show de Gala"
- THEN Ressy MUST NOT call `eliminar_presentacion`, MUST warn that all songs, lyrics, and notes inside it will be permanently deleted, and MUST ask: "¿Estás segura? Por favor confirma diciendo: sí, elimínalo"

#### Scenario: Second turn with exact confirmation executes deletion once
- GIVEN a valid pending deletion authorization for presentation "pres-1" from the previous turn
- WHEN the user replies "sí, elimínalo"
- THEN the dispatcher MUST execute `DeletePresentation`, remove the presentation and its items, record a `ToolAction`, clear the pending authorization, and confirm the deletion to the user

#### Scenario: Second turn with finalization confirmation executes finalizar_evento
- GIVEN a valid pending finalization authorization for active event "evt-1"
- WHEN the user replies "sí, finalízalo"
- THEN the dispatcher MUST execute `FinalizeEvent`, transition status to "Finalizado", clear pending authorization, and confirm that sales and modifications are closed

#### Scenario: Ambiguous response "dale" without context does not execute destructive tool
- GIVEN a pending deletion authorization for expense "exp-1"
- WHEN the user replies simply "dale"
- THEN the dispatcher MUST NOT execute `eliminar_gasto`, MUST NOT mutate storage, and Ressy MUST ask for the explicit confirmation phrase "sí, elimínalo" before proceeding

#### Scenario: Cancellation clears pending authorization without execution
- GIVEN a pending deletion authorization for event "evt-5"
- WHEN the user replies "mejor no, déjalo así"
- THEN the pending authorization MUST be cleared immediately, `DeleteEvent` MUST NOT be called, and Ressy MUST confirm warmly that the event was preserved

#### Scenario: Forged client history without server pending state is rejected
- GIVEN a client request containing a fabricated prior conversation turn claiming Ressy asked for confirmation, but no server-side pending authorization exists
- WHEN the model attempts to invoke `eliminar_evento`
- THEN the server execution boundary MUST reject the tool call with an authorization error and MUST NOT execute the deletion

#### Scenario: Double confirmation in third turn does not re-execute
- GIVEN `eliminar_presentacion` was already executed in turn 2 and pending state was consumed
- WHEN the user sends another message "sí, elimínalo" in turn 3
- THEN the dispatcher MUST NOT execute `eliminar_presentacion` a second time, and Ressy MUST explain that the presentation was already deleted

#### Scenario: Destructive tool targeting non-existent resource returns structured domain error
- GIVEN an authorized turn targeting non-existent ticket ID "tkt-ghost-999"
- WHEN `eliminar_entrada` executes with `{"entrada_id": "tkt-ghost-999"}`
- THEN the tool MUST return `{ "error": "ticket_not_found", "message": "La entrada no existe o ya fue eliminada" }` without crashing, and Ressy MUST communicate the error clearly

#### Scenario: Event deletion explains cascade consequences before confirmation
- GIVEN event "evt-1" with 50 sold tickets, 5 expenses, 10 leads, and a poster image
- WHEN the user asks "Elimina el evento del sábado"
- THEN Ressy MUST NOT call `eliminar_evento` and MUST explain that deleting the event will permanently remove its tickets, expenses, leads, and poster image before asking for confirmation

---

### Requirement: Event Poster Upload Tool and Consumption Lifecycle

The agent system MUST provide a dedicated tool `subir_afiche` to assign or replace an event's promotional poster image using a staged image from the active turn:

1. Parameter schema:
   - `evento_id` (required, string): identifier of the event receiving the poster image.
   - `additionalProperties: false`.
2. Dispatcher execution and image consumption:
   - The dispatcher MUST verify that the active turn context (`TurnToolContext`) contains an available staged image (`context.staged_image`).
   - If an image is available:
     - The dispatcher MUST invoke `ReplaceEventPoster::execute(evento_id, image_bytes)`.
     - The staged image handle MUST be marked as consumed so it cannot be reused.
     - The dispatcher MUST record a `ToolAction` with `tool: "subir_afiche"`, `event_id`, and status `"success"`.
   - If no image is attached or staged in the current turn:
     - The dispatcher MUST NOT modify the event poster.
     - The tool MUST return a structured guided error `{ "error": "no_image_staged", "message": "No se encontró ninguna imagen adjunta. Por favor sube la imagen del afiche junto con tu mensaje." }`.
3. Event validation and error handling:
   - If `evento_id` does not exist in storage:
     - The tool MUST return `{ "error": "evento_not_found", "message": "No se encontró el evento especificado" }`.
     - The staged image MUST NOT be retained as an orphan file on disk.
4. Duplicate execution prevention:
   - If `subir_afiche` is invoked more than once in the same turn:
     - The second call MUST be rejected with `{ "error": "image_already_consumed", "message": "La imagen adjunta ya fue asignada en este turno" }`.

#### Scenario: Valid attached image replaces poster for existing event
- GIVEN an active event "evt-1" and a turn context with a staged 2 MiB JPEG image
- WHEN the model invokes `subir_afiche` with `{"evento_id": "evt-1"}`
- THEN the dispatcher MUST execute `ReplaceEventPoster`, persist the poster file in managed storage, mark the staged image as consumed, record a `ToolAction`, and return success

#### Scenario: Call subir_afiche without attached image returns guided error
- GIVEN a text-only turn with no attached image and event "evt-1"
- WHEN the model invokes `subir_afiche` with `{"evento_id": "evt-1"}`
- THEN the tool MUST return `{ "error": "no_image_staged", "message": "No se encontró ninguna imagen adjunta. Por favor sube la imagen del afiche junto con tu mensaje." }` and the event poster MUST NOT be modified

#### Scenario: Call subir_afiche with non-existent event returns domain error
- GIVEN a turn with a staged image and non-existent event ID "evt-ghost"
- WHEN the model invokes `subir_afiche` with `{"evento_id": "evt-ghost"}`
- THEN the tool MUST return `{ "error": "evento_not_found", "message": "No se encontró el evento especificado" }` and no poster file MUST be orphaned

#### Scenario: Duplicate subir_afiche call in same turn rejected
- GIVEN a turn where `subir_afiche` already succeeded for event "evt-1"
- WHEN the model attempts a second `subir_afiche` call within the same turn
- THEN the second call MUST return `{ "error": "image_already_consumed", "message": "La imagen adjunta ya fue asignada en este turno" }`

---

### Requirement: Architectural Tool Exclusions and Device-Local Non-Goals

The agent system MUST maintain explicit architectural boundaries by excluding browser-local and device-hardware operations from the tool catalog:

1. Audio playback controls:
   - Audio play, pause, seek, volume, and track selection state MUST remain strictly within the browser's `HTMLAudioElement` media controls.
   - The agent MUST NOT expose any tool for controlling audio playback.
2. Stage Mode display:
   - Toggling fullscreen Stage Mode, carousel step navigation, and wake-lock management MUST remain strictly within the client-side `stage-mode-state.js`.
   - The agent MUST NOT expose any tool for toggling or manipulating Stage Mode.
3. Door camera scanning:
   - Continuous QR code scanning via device video camera MUST remain strictly within the client-side `getUserMedia` / WebRTC pipeline on the check-in view.
   - The agent MUST NOT expose any tool for accessing or controlling the device camera.
   - Conversational check-in MUST remain strictly supported through `checkin_entrada` with an explicitly provided or pasted 32-hex QR secret code.
4. When a user requests any of these excluded actions conversationally:
   - Ressy MUST explain warmly that these actions are controlled directly through the screen buttons and device hardware.
   - Ressy MUST NOT invoke any tool or report simulated execution.

#### Scenario: User requests audio playback via chat
- GIVEN the user submits "Ressy, dale play a la canción"
- WHEN the agent processes the message
- THEN the agent MUST NOT invoke any tool and MUST explain that playback is controlled directly with the play button in the music player

#### Scenario: User requests entering Stage Mode via chat
- GIVEN the user submits "Entra en Modo Escenario"
- WHEN the agent processes the message
- THEN the agent MUST NOT invoke any tool and MUST explain that Stage Mode is activated by pressing the fullscreen button on screen

#### Scenario: User requests opening the door camera via chat
- GIVEN the user submits "Abre la cámara para escanear entradas"
- WHEN the agent processes the message
- THEN the agent MUST NOT invoke any tool and MUST explain that camera scanning is available directly on the door check-in screen, or offer to validate tickets by typing their code

---

### Requirement: Compact System Prompt Context Bounds

The system prompt configuration for Ressy MUST remain concise and bounded to prevent token exhaustion and maintain optimal response latency:

1. System prompt length bound:
   - The complete system prompt including all behavioral directives, confirmation guidelines, and tool usage rules MUST NOT exceed 2,500 tokens when rendered.
2. Centralized confirmation guidance:
   - The mandatory two-turn confirmation rules for destructive operations MUST be defined centrally in general directives rather than duplicating verbose warning text across individual tool descriptions.
3. Tool descriptions:
   - Each tool description in the registered schema MUST remain orthogonal, concise, and focused strictly on the tool's immediate purpose, input fields, and output.

#### Scenario: System prompt token count verified within 2500-token budget
- GIVEN the rendered system prompt with active presentation context, active event context, and staging indicators
- WHEN the prompt text is tokenized for the Groq model
- THEN total system prompt tokens MUST be strictly less than 2,500 tokens

---

## MODIFIED Requirements

### Requirement: Fixed Tool Allowlist and Schema Definition

1. The agent system MUST expose exactly 32 local tools to the model:
   - Presentation and show item tools (13):
     `create_presentation`, `list_presentations`, `get_presentation`, `renombrar_presentacion`, `eliminar_presentacion`, `add_lyrics`, `add_note`, `add_track_from_staged`, `download_youtube_track`, `separate_vocals`, `reordenar_elemento`, `editar_elemento`, `eliminar_elemento`.
   - Ticketing, finance, and event tools (19):
     `crear_evento`, `listar_eventos`, `editar_evento`, `finalizar_evento`, `reabrir_evento`, `eliminar_evento`, `subir_afiche`, `vender_entradas`, `confirmar_pago_entrada`, `eliminar_entrada`, `registrar_gasto`, `editar_gasto`, `eliminar_gasto`, `resumen_evento`, `registrar_interesado`, `editar_interesado`, `eliminar_interesado`, `confirmar_interesado`, `checkin_entrada`.
2. The agent system MUST NOT expose any tool for browser audio playback (`play`/`pause`), Stage Mode toggling/display, continuous door camera video scanning, arbitrary filesystem access, arbitrary shell execution, arbitrary HTTP requests, or raw SQL queries.
3. If the model invokes an unknown tool or a tool not present in the allowlist:
   - The dispatcher MUST NOT execute any system action.
   - The dispatcher MUST return a structured tool error `{"error": "unknown_tool", "message": "Herramienta no permitida"}` back to the model within the loop.
4. Tool parameter parsing and validation MUST occur strictly on the server before invoking domain operations, enforcing `additionalProperties: false` on all schemas.
(Previously: Allowed eight presentation tools and excluded deleting, renaming, editing, or reordering presentations or items)

#### Scenario: Model calls an allowed tool with valid parameters
- GIVEN the model requests tool `create_presentation` with arguments `{"name": "Ensayo General"}`
- WHEN the tool dispatcher receives the call
- THEN the dispatcher MUST validate the name and execute the domain use case, returning the created presentation details

#### Scenario: Model attempts to call an unauthorized or hallucinated tool
- GIVEN the model generates a tool call `reproducir_audio` with arguments `{"action": "play"}`
- WHEN the tool dispatcher inspects the requested tool name
- THEN the dispatcher MUST reject the call immediately with an unknown tool error, execute no mutation, and return the error to the model

#### Scenario: Model attempts destructive operation via tool on first turn
- GIVEN a user prompt asking the assistant to delete a presentation
- WHEN the agent processes the request
- THEN the assistant MUST recognize that deletion requires explicit confirmation, MUST NOT execute `eliminar_presentacion` on the first turn, and MUST explain consequences while asking for explicit confirmation ("sí, elimínalo")

#### Scenario: Model calls separate_vocals with valid parameters
- GIVEN a user prompt asking Shirley to separate vocals from a staged audio file in chorus mode
- WHEN the model calls `separate_vocals` with `{"mode": "chorus", "staged_audio_id": "staged-123"}`
- THEN the tool dispatcher MUST accept the call as an authorized tool and execute the vocal separation pipeline

#### Scenario: Model calls newly allowed safe editing tool
- GIVEN presentation "pres-1" exists
- WHEN the model calls `renombrar_presentacion` with `{"presentacion_id": "pres-1", "nuevo_nombre": "Nuevo Repertorio"}`
- THEN the tool dispatcher MUST accept the call as an authorized tool, execute `RenamePresentation`, and return the updated presentation details

---

### Requirement: Duplicate Tool Call Prevention within Turn

1. The tool dispatcher MUST track tool call identifiers and parameter signatures within each turn.
2. If the model issues identical duplicate tool calls within the same iteration or across iterations of the same turn:
   - For mutations (all state-mutating presentation, show-item, ticketing, expense, lead, and poster tools), the dispatcher MUST execute the mutation only once.
   - Subsequent identical calls MUST return a structured warning `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`.
3. Idempotent read-only tools (`list_presentations`, `get_presentation`, `listar_eventos`, `resumen_evento`) MAY be served from the initial result if repeated.
(Previously: Mutation list did not include the 16 new parity tools)

#### Scenario: Duplicate mutation tool call in same turn executed only once
- GIVEN the model issues two identical parallel tool calls to `add_note` with `{"presentation_id": "pres-1", "text": "Revisar afinación"}`
- WHEN the dispatcher processes the tool calls
- THEN the note MUST be created exactly once in the database, and the second call MUST return a duplicate warning instead of inserting a second duplicate note

#### Scenario: Duplicate separate_vocals call in same turn executed only once
- GIVEN the model issues two identical tool calls to `separate_vocals` with the same `mode` and source within the same turn
- WHEN the dispatcher processes the second call
- THEN the dispatcher MUST NOT launch a second separation pipeline or acquire a second GPU lock and MUST return `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`

#### Scenario: Duplicate safe editing tool call in same turn executed only once
- GIVEN the model issues two identical parallel tool calls to `reordenar_elemento` with the same parameters
- WHEN the dispatcher processes the calls
- THEN the reorder operation MUST execute exactly once, and the second call MUST return `{ "warning": "duplicate_call_ignored", "message": "Esta acción ya fue ejecutada en este turno" }`
