# Proposal: Ressy agent parity

**Change:** `shirley-show-agent-parity`  
**Status:** Proposed  
**Artifact store:** OpenSpec  
**Delivery:** `ask-on-risk`, review budget of 400 changed lines per slice

## Intent

Close the operational gap between Shirley Show's web interface and Ressy so that administrative and management actions backed by existing application use cases can also be completed conversationally. The change expands Ressy's allowlist from 16 to 32 tools while preserving domain ownership in the current use cases, adding an enforced two-turn safety boundary for destructive actions, and allowing an attached image to be used as an event poster.

The intended product outcome is operational parity for persistent application actions: users can manage presentations, show items, events, tickets, expenses, and interested parties without leaving the chat, while irreversible changes remain deliberate and poster uploads remain bounded and type-safe.

## Current-state gap

Ressy currently exposes 16 tools, but 16 persistent actions available through buttons and forms have no conversational equivalent. Existing use cases already implement these operations, so the missing capability is at the agent schema, dispatch, prompt, turn-context, and attachment boundaries rather than in the domain or persistence layers.

The chat attachment path is audio-specific today. It cannot accept an event-poster image or safely distinguish image content from audio independently of client-provided metadata.

## Scope

### Included

Add the following 16 tools to the agent allowlist:

- Safe editing and lifecycle actions: `renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, and `confirmar_interesado`.
- Destructive or critical lifecycle actions: `eliminar_presentacion`, `eliminar_elemento`, `eliminar_evento`, `finalizar_evento`, `eliminar_entrada`, `eliminar_gasto`, and `eliminar_interesado`.
- Poster management: `subir_afiche`.

The tools will:

- Use strict JSON schemas with `additionalProperties: false` and domain-appropriate required fields, types, ranges, and enums.
- Dispatch to the existing application use cases identified in `explore.md`; they will not duplicate business rules or directly mutate repositories.
- Return domain failures without representing them as successful operations.
- Produce the existing action/update signals needed for affected UI views to refresh.

### Destructive-action policy

For the seven destructive or critical lifecycle tools:

1. Ressy must not execute the tool in the first turn that expresses destructive intent.
2. Ressy must identify the target and explain the relevant consequence, including cascade effects for event or presentation deletion.
3. Ressy must request an explicit second-turn confirmation using `sí, elimínalo` for deletion or `sí, finalízalo` for finalization.
4. Execution is permitted only for the same pending operation and target in the immediately following user turn after the required explicit confirmation.
5. A cancellation, hesitation, changed target, unrelated request, or missing confirmation clears or invalidates the pending action without invoking the tool.
6. This policy must be enforced at the execution boundary as well as communicated by the system prompt; prompt compliance or client-authored conversation history alone is not adequate authorization.

### Image attachment and poster upload

- Extend the chat composer and API validation to accept JPEG, PNG, and WebP images alongside the currently supported audio types.
- Keep the existing 100 MiB audio limit and enforce a 10 MiB image limit.
- Detect supported image formats from magic bytes in the backend rather than trusting filename extensions or declared MIME types.
- Stage at most the image associated with the active turn and expose it through turn context to `subir_afiche`.
- Consume the staged image when `subir_afiche` succeeds; if no valid image is attached, return a guided error and do not alter the event.
- Reuse `ReplaceEventPoster` for storage and replacement behavior.

## Non-goals

- Controlling audio playback or pause state, because that state belongs to the browser's local media element.
- Entering or leaving Stage Mode, because it is a local presentation concern without a persistent backend action.
- Starting or controlling the door camera, because continuous WebRTC capture is device-local; conversational check-in remains available through `checkin_entrada` with a supplied code.
- Adding new persistence rules, repository mutations, or replacement use cases for operations already implemented.
- Redesigning the chat interface beyond the attachment affordance and validation feedback required for supported images.
- Broad multimodal support for arbitrary documents, multiple images, image interpretation, or image editing.

## Affected areas

| Area | Expected change |
| --- | --- |
| `src/agent/tool_schemas.rs` | Register 16 strict tool schemas and update the allowlist count from 16 to 32. |
| `src/agent/tool_dispatcher.rs` | Parse arguments, enforce execution preconditions, call existing presentation and ticketing use cases, and emit existing action signals. |
| `src/agent/system_prompt.rs` | Describe the added operations and the compact, mandatory two-turn confirmation policy. |
| Agent confirmation/request boundary | Carry server-trusted evidence of the pending action and target across the two requests; client-authored history alone must not authorize execution. |
| Agent turn context | Carry one staged image for the active turn and expose its consumption state to the dispatcher. |
| `src/handlers/agent/multipart_request.rs` | Classify attachments using content bytes, retain the audio path, and stage bounded supported images. |
| `frontend/js/data/agent-chat-api.js` | Validate supported audio/image types and their separate size limits before upload. |
| `frontend/js/components/organisms/agent-chat-composer.js` | Allow supported image selection alongside audio. |
| Attachment presentation | Represent image name and size without coupling the component to backend behavior. |
| Agent and ticketing tests | Cover schemas, dispatch, destructive confirmation, attachment classification, poster replacement, errors, and allowlist count. |

No public domain contract is expected to change. The agent-facing tool catalog and multipart attachment contract do change and their consumers must be verified together.

## Delivery slices

The change is divided into three independently reviewable slices:

1. **Safe tools:** add and verify the eight non-destructive editing, reordering, reopening, and conversion tools.
2. **Destructive tools:** add the seven destructive tools and enforce the bound two-turn confirmation flow.
3. **Poster upload:** add `subir_afiche`, image-aware chat attachments, backend magic-byte validation, and turn-scoped image staging.

Each slice targets at most 400 changed lines. Under `ask-on-risk`, implementation must pause for a delivery decision if a slice forecast exceeds that budget; no size exception or chain strategy is inferred by this proposal.

## Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| An accidental, forged, or stale confirmation triggers data loss | High | Bind server-trusted confirmation state to one named action and target, require the next user turn, clear it on cancellation or context change, and enforce the rule before use-case execution. |
| Prompt wording is treated as the only safety mechanism | High | Add deterministic execution-boundary checks and test that direct or first-turn destructive calls are rejected. |
| Event deletion removes related tickets, expenses, interests, and poster data | High | State cascade consequences before confirmation and continue using the existing `DeleteEvent` use case as the sole owner of deletion behavior. |
| The larger catalog reduces tool-selection accuracy or inflates prompt size | Medium | Keep names orthogonal, schemas strict, descriptions concise, and confirmation guidance centralized. |
| Spoofed MIME types or extensions admit invalid poster data | High | Enforce the 10 MiB limit and supported format using backend magic-byte detection before staging or dispatch. |
| Image support regresses existing audio uploads | Medium | Preserve the current audio staging path and add classification tests for valid audio, valid images, unsupported bytes, and oversized payloads. |
| Partial edits overwrite fields unexpectedly | Medium | Match each schema to the existing use-case input contract and test required/full-field semantics before exposing the tool. |
| UI views remain stale after conversational mutations | Medium | Reuse existing `ToolAction`/reactive refresh conventions and verify affected event and presentation views. |

## Rollback

Rollback can be performed by slice:

1. Remove the corresponding new schemas from the allowlist and dispatcher while leaving existing domain use cases untouched.
2. For destructive tools, disable their registration first so no new destructive request can execute, then remove confirmation state and prompt guidance.
3. For poster upload, remove image acceptance in the frontend and image staging in multipart handling while retaining the pre-existing audio-only path and poster management available through the existing UI.
4. If a production issue affects only one tool, remove that tool from the allowlist as an immediate kill switch without reverting unrelated tools.

No data migration is introduced, so rollback does not require schema reversal. Operations already completed through valid use cases remain authoritative and are not automatically undone.

## Success criteria

1. Ressy advertises exactly 32 allowed tools, including all 16 additions with strict schemas.
2. Each new safe tool successfully invokes its existing use case and returns meaningful success or domain-error output.
3. None of the seven destructive tools can execute on an initial request, without a matching pending target, after a cancellation/context change, or with an invalid second-turn confirmation.
4. A valid immediate second-turn `sí, elimínalo` or `sí, finalízalo` executes exactly the previously disclosed operation against the same target once.
5. JPEG, PNG, and WebP attachments of at most 10 MiB are accepted based on their content bytes and can replace an event poster through `subir_afiche`.
6. Unsupported, spoofed, oversized, or missing image attachments are rejected without changing poster data and with actionable user feedback.
7. Existing supported audio attachments and all 16 pre-existing tools continue to behave as before.
8. Mutations trigger the established client refresh behavior for the affected presentation or event view.
9. Tests cover schema registration, dispatch success/failure paths, two-turn confirmation boundaries, multipart classification, size limits, image consumption, and audio regression behavior under the project's strict-TDD workflow.

## Proposal assumptions confirmed by handoff

- The 16-tool inventory and three-slice boundary are closed product decisions for this proposal.
- Existing use cases remain the source of truth for permissions, invariants, cascades, and persistence.
- The required destructive confirmation is a two-turn conversational flow, not inferred consent from the initial instruction.
- Operational parity excludes browser-local playback, Stage Mode, and camera control for the ownership reasons stated above.
