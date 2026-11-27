# Global agent chat: separate and save from anywhere

The agent chat must separate voices and save the result from the main screen, into the repertoire the user chooses, and keep working while the user navigates. A separation must not answer "falló" while time remains for a retry.

## Specs

- S1: «no deberia responder fallo, probar reintentos, otra cosa». When a separation attempt fails and the turn still has time, the system retries (same warm GPU if healthy, otherwise another GPU/offer) before answering. The user sees a failure only when no attempt can finish within the turn.
- S2: «no me gusta que tenga que entrar a un repertorio para que el chat recien pueda separar audios y todo eso, deberia poder hacer todo desde la pantalla principal». From the main screen (no repertoire open), the chat can separate a track that exists in any repertoire, a YouTube link, or an attached file.
- S3: «el chat debe poder seguir separando y guardando en el repertorio que yo elija». The user names the destination repertoire in the conversation (by name); the result is saved there. If it is not clear, the agent asks which one, listing the existing repertoires. The destination chosen in the conversation is kept for the rest of the conversation.
- S4: «yo si quiero me muevo a donde se me cante y el chat debe poder seguir separando». Navigating between screens while a separation is running does not cancel it, does not lose the conversation, and the result still lands in the chosen repertoire; the open repertoire view refreshes when the result arrives.
- S5 (live-production invariant, AGENTS.md §0): existing API fields, routes, error codes, QR, ticketing and check-in are unchanged; `presentation_id` stays optional and its current meaning when present is preserved.

## Tasks

- T1 [S1] Retry a failed Vast/Colab separation within the turn deadline — route: gentle-ai-worker — commit: pending
- T2 [S2, S3, S5] Backend: agent chat without an open repertoire resolves track/destination by name and saves into the chosen repertoire — route: gentle-ai-worker — commit: pending
- T3 [S4, S5] Frontend: chat state and in-flight request survive navigation; chosen destination is not overwritten by route changes; views refresh on result — route: gentle-ai-worker — commit: pending

## Log

- L1 (user, 2026-11-27): «Si responde «falló», que la vuelva a pedir enseguida. La segunda vez funciona y tarda unos 2 minutos: lo comprobé hace un rato con «Gitana» y la pista separada quedó agregada al show. no deberia responder fallo, probar reintentos, otra cosa, no me gusta que tenga que entrar a un repertorio para que el chat recien pueda separar audios y todo eso, deberia poder hacer todo desde la pantalla principal, yo si quiero me muevo a donde se me cante y el chat debe poder seguir separando y guardando en el repertorio que yo elija»
- L2 (evidence): the chat is already a global floating bubble (`frontend/js/controllers/agent-chat-controller.js:113`), but `presentation_id` comes from the route context (`:162`, `chat-state.js:110`), and backend tool routing keys on `active_presentation_id` (`src/use_cases/agent_chat.rs:428`). "Repertorio" in UI = presentation (`frontend/js/components/organisms/app-topbar.js`).
- L3 (evidence): live tests on 2026-11-27 — cold Vast start failed with `Connection refused` right after `running` (fixed by PR #118, b8cf1c1); warm reuse succeeded end to end (Gitana, 218.46 s, item 8a4e9fcc).
