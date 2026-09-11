# Ressy Remote UX Specification

## Purpose

Defines the composed remote deadline, the PC-offline signal with verbatim conversational copy, and the waiting typing indicator for Ressy turns that delegate vocal separation to the remote GPU PC.

## Requirements

### Requirement: Composed Remote Deadline
1. Remote separation turns MUST allow `wake (600s) + work (600s) + margin (300s) = 1500s` via `SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS`.
2. Local mode keeps the 600s separation deadline.

#### Scenario: 12-minute remote turn survives
- GIVEN 8 min wake + 4 min GPU
- WHEN the turn runs remotely
- THEN the server MUST NOT cancel at 600s.

### Requirement: PC-Offline Signal and Verbatim Copy
1. GPU presence MUST be queryable via `GET /api/agent/gpu-status` returning `{ mode: "local"|"remote", pc_online: bool }` (read-only 3-second probe without side effects). Upon vocal separation intent, client probes `gpu-status`; if `remote` and `pc_online == false`, the client shows `esperando la PC de Joshua…` while holding the request in-turn up to 10 minutes without SSE. When applicable, the turn returns the result or `notice: { code: "wake_wait"|"pc_wake_timeout", retryable: true }` keeping `staged_audio_id`.
2. Messages MUST be verbatim:
   - Wait: `Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo.`
   - Energy: `Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.`
   - Give-up: `No pude conectarme con la PC de Joshua después de 10 minutos. Cuando esté encendida, vuelve a pedirme la separación.`
3. MUST NOT claim work done while waiting; MUST NOT leak IPs, users, flags, paths, stderr.

#### Scenario: Offline PC asks to power on
- GIVEN offline PC and active show
- WHEN vocal separation is requested
- THEN client probes `gpu-status` and displays `esperando la PC de Joshua…`, and if unreachable after deadline returns give-up copy with `pc_wake_timeout` notice preserving staged audio.

#### Scenario: Give-up after 10 minutes
- GIVEN 600s unreachable
- WHEN the deadline hits
- THEN response quotes the give-up text with `pc_wake_timeout`.

### Requirement: Waiting Typing Indicator
1. While waiting MUST show exactly `esperando la PC de Joshua…` (amber pulse, `aria-live="polite"`, no fake percentages).
2. On resolve/fail/timeout/close MUST restore `Ressy está pensando...` and the composer.

#### Scenario: Indicator lifecycle
- GIVEN a wake-wait turn
- WHEN it resolves
- THEN the label disappears and the composer re-enables.