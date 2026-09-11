# Delta spec: ressy-remote-ux (mirror Engram sdd/shirley-show-deploy/spec)

## ADDED Requirements

### Requirement: Composed Remote Deadline
1. Remote separation turns MUST allow `wake (600s) + work (600s) + margin (300s) = 1500s` via `SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS`.
2. Local mode keeps the 600s separation deadline.

#### Scenario: 12-minute remote turn survives
- GIVEN 8 min wake + 4 min GPU
- WHEN the turn runs remotely
- THEN the server MUST NOT cancel at 600s.

### Requirement: PC-Offline Signal and Verbatim Copy
1. Offline probe MUST return `notice: { code: "wake_wait", retryable: true }` keeping `staged_audio_id`.
2. Messages MUST be verbatim:
   - Wait: `Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo.`
   - Energy: `Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.`
   - Give-up: `No pude conectarme con la PC de Joshua después de 10 minutos. Cuando esté encendida, vuelve a pedirme la separación.`
3. MUST NOT claim work done while waiting; MUST NOT leak IPs, users, flags, paths, stderr.

#### Scenario: Offline PC asks to power on
- GIVEN offline PC and active show
- WHEN the turn runs
- THEN response carries `wake_wait`, keeps staging, and quotes the wait text.

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
