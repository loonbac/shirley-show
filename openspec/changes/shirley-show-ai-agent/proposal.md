# Proposal: Shirley Show AI Agent

**Change:** `shirley-show-ai-agent`  
**Status:** Proposed  
**Product owner:** Shirley  
**Execution mode:** Auto  
**Artifact store:** OpenSpec  
**Delivery strategy:** Ask on risk  
**Review budget:** 400 changed lines per review unit  
**Planned slices:** S1 backend agent; S2 conversational UI

## Intent

Add Shirley, a large, warm conversational assistant that lets the user create presentations and add lyrics, notes, and attached audio tracks through natural language. The assistant will use Groq's OpenAI-compatible chat-completions API with `llama-3.3-70b-versatile`, while all product mutations remain constrained to explicit local tools backed by Shirley Show's existing domain operations.

The assistant complements rather than replaces the canonical catalog, preparation, and Stage Mode workflows. If Groq is unavailable or no API key is configured, the existing application must remain fully usable and the chat must explain the limitation without exposing secrets or presenting false success.

## Product problem

Shirley Show already offers reliable structured controls, but building a repertoire still requires navigating presentation and item-specific forms one operation at a time. This creates friction when the user already knows the desired result in conversational terms, such as “crea un show para el viernes, añade esta letra y sube esta pista”.

A conversational surface can reduce that translation effort, especially while preparing a show from a phone, but only if its actions are predictable, bounded, and visibly confirmed. An unconstrained model would introduce unacceptable ambiguity around persistence, files, and destructive actions, so the product needs a narrow tool-mediated agent rather than free-form model access to storage or the filesystem.

## Target users and situations

The primary in-scope user is Shirley, using the existing local or trusted-network application without application-level authentication.

The assistant is intended for preparation workflows on phone and desktop:

1. Starting a new presentation from a natural-language request.
2. Finding or inspecting presentations before adding content.
3. Adding lyrics or a stage note without navigating separate creation flows.
4. Attaching one supported audio file and asking Shirley to add it as a named track.
5. Continuing a short conversation whose history lives only in the current browser session.

Live voice operation, autonomous performance control, and replacing Stage Mode are not part of this change.

## Product outcome

After this change:

- The user can open a prominent, visually rich chat and describe presentation-building actions naturally.
- Shirley can create, list, and inspect presentations and can add lyrics, notes, or one staged audio track by calling a bounded set of local tools.
- Every persisted result is reported from actual tool output; model text alone never counts as a successful mutation.
- An attached audio file is available only to the current request's staged-track tool and is cleaned up if unused or if the request fails.
- The client keeps the conversation while the page session remains active, without adding server-side chat persistence.
- Missing configuration, provider errors, invalid tool arguments, timeouts, and iteration limits produce calm, actionable messages while leaving canonical manual workflows intact.

## Current-state gap

The application has canonical APIs and use cases for presentations and show items, including safe staged audio handling, but no conversational orchestration layer. The browser has no chat state, assistant components, attachment composer, or API client. The server has no Groq configuration, provider adapter, tool schema registry, tool dispatcher, bounded agent loop, or `/api/agent/chat` route.

The existing `.env.example` identifies `GROQ_API_KEY` and the default `GROQ_MODEL`, while `.env` is git-ignored. Runtime loading and graceful availability behavior still need to be integrated without sending provider credentials to the browser.

## Product rules and invariants

This change MUST preserve `openspec/specs/presentation/spec.md` and `openspec/specs/show-item/spec.md` as canonical behavior.

1. The only model available by default is `llama-3.3-70b-versatile`, configurable server-side through `GROQ_MODEL`; the Groq credential remains server-only in `GROQ_API_KEY`.
2. The agent may invoke only: `create_presentation`, `list_presentations`, `get_presentation`, `add_lyrics`, `add_note`, and `add_track_from_staged`.
3. The agent has no delete, rename, edit, reorder, playback, arbitrary HTTP, arbitrary SQL, shell, or filesystem tool.
4. Local tools MUST call existing domain/use-case boundaries or focused reusable equivalents; the model never accesses SQLite or files directly.
5. Tool arguments MUST be parsed and validated server-side, including stable presentation identifiers, trimmed non-empty text, item-kind rules, supported audio formats, and the canonical 100 MiB limit.
6. Mutations MUST preserve stable presentation IDs, append new items at contiguous position `N`, and retain all existing upload cleanup and data-integrity guarantees.
7. A successful assistant statement about a mutation MUST be grounded in a successful local tool result. Failed tool calls must not be represented as completed work.
8. The agent loop is capped at five model iterations per request and the entire request is bounded by a 30-second timeout.
9. `POST /api/agent/chat` accepts JSON for text-only turns and multipart for turns with one audio attachment. Multipart metadata carries the same conversation/message contract as JSON.
10. The staged audio handle is scoped to one request. Only `add_track_from_staged` may consume it, at most once, and all unconsumed or failed staging artifacts MUST be cleaned up.
11. A track cannot be synthesized from a filename or model claim. It requires the current request's valid staged file, a visible non-empty title, and a valid target presentation.
12. Client conversation history is ephemeral in-memory state. Reloading or closing the page may clear it, and the server does not persist chat transcripts.
13. The server treats client-supplied history as untrusted input, bounds its accepted size, and never trusts it as proof that a tool already ran.
14. Provider output and tool arguments are untrusted. Unknown tools, malformed calls, incompatible arguments, and repeated calls are rejected safely.
15. If `GROQ_API_KEY` is absent, the server MUST NOT attempt a provider call; the UI displays a friendly configuration state and all non-AI workflows remain available.
16. Secrets, full provider payloads containing sensitive data, and raw attached audio MUST NOT be written to normal application logs.
17. No assistant action autoplays audio or alters the singleton playback state.
18. The chat must be keyboard-operable, screen-reader understandable, mobile-first, and respectful of `prefers-reduced-motion`.

## Scope

### Slice 1 — Backend agent and bounded tool execution

- Add server-side Groq configuration with `GROQ_API_KEY` and a default `GROQ_MODEL=llama-3.3-70b-versatile`.
- Add a minimal OpenAI-compatible Groq chat-completions client behind a narrow provider port.
- Define provider request/response DTOs, assistant/tool message mapping, and an explicit local tool schema registry.
- Implement focused adapters for `create_presentation`, `list_presentations`, `get_presentation`, `add_lyrics`, `add_note`, and `add_track_from_staged` using canonical local use cases.
- Implement a bounded agent-chat use case that calls Groq, dispatches requested local tools, returns tool results to the model, and stops on a final assistant response, five iterations, or the 30-second request deadline.
- Add `POST /api/agent/chat` with text-only JSON and single-attachment multipart handling.
- Reuse the existing staging guard for attached audio so success, provider failure, invalid calls, timeout, disconnect, and unused attachments all have explicit cleanup behavior.
- Normalize expected errors into a stable client-facing envelope that distinguishes configuration, validation, timeout, provider, iteration-limit, and local-tool failures without leaking secrets.
- Add focused tests for tool allowlisting, argument validation, loop/timeout bounds, no-key behavior, grounded mutation results, and staged-file cleanup.

### Slice 2 — Large conversational UI

- Add a prominent chat entry point and a large responsive conversation surface rather than a small utility widget.
- Compose the UI from focused atoms, molecules, and organisms: assistant/user message bubbles, status indicator, attachment chip, composer, send control, typing/loading treatment, error/retry state, and chat panel.
- Add component-scoped styles and reusable motion tokens for a polished, warm experience consistent with Shirley Show's visual language.
- Add a headless chat-state module for ephemeral messages, pending state, attachment state, and request cancellation/staleness guards.
- Add a focused data client that chooses JSON or multipart according to attachment presence and maps the backend error envelope to user-facing states.
- Keep draft text and attachment available when a request fails before the file is consumed, where browser file semantics allow retry; never duplicate a successful mutation on retry.
- Announce loading, tool outcomes, and errors accessibly; return focus predictably after sending.
- Disable duplicate submission while a turn is pending and ignore stale responses after chat teardown or navigation.
- Reduce or remove nonessential animation when the user prefers reduced motion.

## Affected areas

### Backend

- Runtime configuration and environment documentation.
- Groq provider port and infrastructure adapter.
- Agent message/tool domain DTOs and the bounded orchestration use case.
- Local tool schemas, dispatch, and adapters to existing presentation/show-item use cases.
- Multipart staging lifecycle and API error mapping.
- Axum route registration and focused HTTP/use-case tests.

### Frontend

- Agent API client and multipart request construction.
- Ephemeral chat signal/store and lifecycle management.
- New chat atoms, molecules, organism, and optional view-level integration.
- Component-scoped CSS, motion, responsive layout, accessibility labels, live regions, and focus states.

### Operations and support

- Operators must provide a valid Groq key in the server's local `.env` or process environment to enable AI.
- Provider latency, quota, model availability, and outbound network access become new operational dependencies only for chat.
- Existing local CRUD, uploads, Stage Mode, and playback continue without Groq.
- The no-auth deployment posture remains unchanged: exposing the server publicly would also expose mutation-capable agent tools and is unsafe without external access control.

## Edge cases and failure behavior

- **No API key:** return a stable unavailable/configuration response; show friendly setup guidance without exposing the expected key value.
- **Invalid or revoked key:** report that Shirley cannot connect; preserve the current conversation and manual app workflows.
- **Provider timeout or slow tool loop:** cancel at the 30-second request boundary, clean staged resources, and avoid claiming incomplete actions succeeded.
- **Iteration limit reached:** stop after five model iterations and return a bounded “no pude completar” response with any tool outcomes that were actually committed.
- **Malformed provider response:** reject it as a provider failure rather than guessing tool names or arguments.
- **Unknown/disallowed tool:** do not execute it; return a structured tool error to the model within the remaining bounded loop.
- **Duplicate tool call:** do not infer idempotency for mutations. Dispatch tracks call identity within the request and prevents accidental re-execution of the same call.
- **Partial multi-action request:** report each confirmed result and the failed remainder clearly; do not roll back unrelated successful presentation/item mutations unless the canonical use case itself is transactional.
- **Missing presentation or stale identifier:** return a not-found tool result and let the assistant ask for clarification or use the list tool; never create an implicit replacement.
- **Ambiguous target presentation:** Shirley asks a clarifying product question instead of selecting by duplicate visible title.
- **Empty or oversized history:** accept a bounded current message, reject invalid payloads clearly, and prevent unbounded provider/context cost.
- **No attachment with track request:** explain that an audio attachment is required and do not create an item.
- **Attachment supplied but unused:** clean the staged file at request completion.
- **Unsupported or oversized audio:** apply canonical upload validation before tool execution and do not call Groq with raw audio bytes.
- **Track tool called twice:** only the first successful consumption may publish the staged file; subsequent attempts fail safely.
- **Client disconnect or navigation:** server cleanup remains guard-driven; the client ignores late responses and releases UI listeners.
- **Page reload:** chat history is lost by design; persisted presentations and items remain unchanged.
- **Long messages or lyrics:** wrap safely, preserve multiline text, constrain the scroll region, and never render model output as unsanitized HTML.
- **Reduced motion:** preserve hierarchy and state feedback without decorative movement.

## Constraints

- Keep Rust + Axum, SQLite, local filesystem audio, and vanilla JavaScript ES modules.
- Use OpenAI-compatible chat completions against Groq; do not add a second AI provider or a browser-side provider SDK.
- Keep provider details behind a narrow port so orchestration and tool dispatch remain locally testable.
- Follow `AGENTS.md` micro-modularity: one focused use case/action per module, minimal ports, pure UI supplied by data and callbacks, headless behavior/state, and component-scoped CSS.
- Do not expose `GROQ_API_KEY` through frontend assets, API responses, logs, or committed configuration.
- Do not send attached audio content to Groq; only textual conversation and bounded tool metadata/results enter the model context.
- Preserve canonical validation, ordering, upload, cleanup, playback, and stale-route behavior.
- Use strict TDD for backend behavior with `cargo test`; frontend work should remain separable and manually verifiable where no browser runner exists.
- Each slice is an independent review unit targeted at no more than 400 changed lines. Under `ask-on-risk`, implementation must pause for a delivery decision if either forecast exceeds the budget; no chain strategy or size exception is inferred.

## Tradeoffs and decisions

1. **Bounded tools over autonomous API access:** the assistant can complete valuable creation flows while destructive and unrelated capabilities remain impossible.
2. **Immediate tool execution over preview/confirmation:** natural-language creation stays fast, but the UI must clearly report committed outcomes. Destructive tools are excluded to keep this safe.
3. **Client memory over persisted transcripts:** implementation and privacy scope stay small, at the cost of losing context on reload.
4. **Single-turn HTTP over SSE:** the API and UI remain simpler, but users receive a completed response rather than streamed tokens during provider latency.
5. **One attachment per turn over batch upload:** staging ownership and natural-language reference stay unambiguous, while multi-track import remains manual or future work.
6. **Global 30-second deadline over best-effort completion:** resources and waiting are bounded, though complex requests may complete only partially and require a follow-up.
7. **Five model iterations over open-ended planning:** tool loops cannot run indefinitely, at the cost of asking the user to continue unusually complex requests.
8. **Large dedicated chat over a small floating widget:** conversation becomes legible and delightful on mobile, while occupying more screen space and remaining separate from Stage Mode.
9. **Provider-specific configuration behind a port:** Groq is the only in-scope provider, but domain orchestration does not become coupled to HTTP payload details.

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Hallucinated success | User trusts an action that never persisted | Generate success language only after local tool results and return structured outcomes. |
| Prompt injection requests unsafe operations | Model attempts unauthorized access | Fixed allowlist, strict schemas, no generic network/filesystem/SQL tool, and server-side validation. |
| Duplicate mutations during retries or repeated calls | Duplicate presentations or items | Disable concurrent sends, track tool call IDs per request, expose partial outcomes, and never auto-retry mutation turns. |
| Partial completion before timeout | User cannot tell what changed | Preserve committed canonical operations and report confirmed successes and failures individually. |
| Staged audio leak | Disk fills with temporary files | Reuse guard-based staging and test every exit path, including timeout and disconnect. |
| Key leakage | Groq account compromise | Server-only environment loading, redacted errors/logs, and no provider credential in frontend bundles. |
| Provider outage or quota exhaustion | Chat becomes unavailable | Friendly degradation with manual workflows fully available. |
| Long history increases latency/cost | Slow or rejected requests | Bound message count/content size and keep history ephemeral. |
| Public no-auth exposure | Anyone could create content and consume quota | Preserve trusted-network guidance and recommend external access control; auth remains out of scope. |
| Animated large chat harms accessibility or performance | Motion discomfort or weak mobile usability | Prefer transform/opacity motion, reduced-motion fallback, stable layout, and bounded scroll rendering. |
| Slice exceeds 400 changed lines | Review quality declines | Forecast S1 and S2 separately and trigger the ask-on-risk gate before oversized implementation. |

## Rollback plan

Rollback is slice-based and must preserve presentations, show items, and audio.

1. **Slice 2 rollback:** remove the chat entry point, client, ephemeral state, components, and styles. The backend endpoint may remain unused while all canonical UI workflows continue unchanged.
2. **Slice 1 rollback:** remove the agent route, Groq adapter/configuration, orchestration, and tool dispatcher together. Existing domain APIs and use cases remain the authoritative manual interface.
3. **Configuration rollback:** remove `GROQ_API_KEY` and optional model configuration from the runtime environment. No key value is stored in SQLite or frontend state.
4. **Data handling:** successful agent-created presentations and items are ordinary canonical records and MUST NOT be deleted during rollback. They remain usable through existing manual views.
5. **File handling:** successful agent-created tracks remain managed audio records. Temporary staged files are removed by guard cleanup; rollback must not manually delete the managed audio directory.
6. **Operational recovery:** stop the server, deploy the prior known-good binary/static assets, preserve the SQLite database and managed audio directory, restart, and verify catalog access, an agent-created presentation, item ordering, and representative audio playback.
7. **Compatibility gate:** this proposal expects no database migration. If implementation discovers one is required, it needs separate justification and a tested backward/down path before proceeding.

## Success criteria

1. With valid Groq configuration, a text-only request to `POST /api/agent/chat` can create a presentation and return a response grounded in the persisted tool result.
2. Shirley can list and inspect existing presentations through local tools without granting the model direct storage access.
3. Shirley can add valid lyrics and notes to a selected presentation while preserving trimmed non-empty content and contiguous append order.
4. A multipart turn with one valid audio attachment can create one named track through `add_track_from_staged`, with canonical format/size validation and no orphaned staging file.
5. A track request without an attachment, an unsupported file, a missing presentation, malformed tool arguments, or a disallowed tool causes no invalid mutation and yields actionable feedback.
6. The loop never exceeds five model iterations and the full agent request never runs beyond the 30-second deadline without returning/cancelling through controlled error handling.
7. No-key, invalid-key, provider-error, timeout, and iteration-limit states leave the rest of Shirley Show fully usable and never expose the credential.
8. The browser presents a large, polished, responsive chat with clear user/assistant roles, attachment state, pending feedback, accessible focus/live announcements, and reduced-motion support.
9. Conversation history survives ordinary turns within the current page session but is not persisted by the server or after reload.
10. Assistant content is rendered safely as text, long content remains usable, duplicate pending submissions are blocked, and stale responses do not mutate torn-down UI.
11. Existing presentation, show-item, upload, ordering, Stage Mode, and audio playback canonical specifications do not regress.
12. S1 and S2 remain independently reviewable within 400 changed lines each, or implementation pauses at the configured ask-on-risk gate before exceeding that budget.

## Non-goals

- Streaming responses, Server-Sent Events, WebSockets, or token-by-token rendering.
- Voice input, speech synthesis, transcription, audio analysis, or sending audio bytes to Groq.
- Persisted, synchronized, searchable, or server-side conversation history.
- Multiple users, accounts, permissions, collaboration, or per-user AI quotas.
- Delete, rename, edit, reorder, playback, Stage Mode control, or other destructive/autonomous agent tools.
- Multiple attachments, batch track import, cloud media storage, or remote file URLs.
- Autonomous background agents, scheduled work, proactive suggestions, or work that continues after the request ends.
- Multiple AI providers, local model hosting, embeddings, retrieval-augmented generation, or a general plugin system.
- Changes to canonical audio formats, the 100 MiB upload limit, or existing manual creation workflows.

## Delivery sequence

1. **S1 — Backend:** configuration, Groq provider adapter, bounded loop, local tools, staging lifecycle, endpoint, and tests.
2. **S2 — Chat UI:** ephemeral state, API client, large conversational composition, attachment flow, accessibility, animation, and integration.

S2 depends on the stable S1 response/error contract. Each slice must leave the application usable on its own. Before implementation, each slice receives a changed-line forecast; under `ask-on-risk`, any forecast above 400 lines pauses for an explicit delivery choice.