# Check-in Specification

## Purpose

Defines access control, credential verification, and door entry validation at live performances in Shirley Show. The system evaluates scanned QR codes or manually entered codes against event admission records and provides immediate, unambiguous feedback in three discrete states.

## Requirements

### Requirement: Exclusive Verification by Secret QR Code

The system MUST validate door entries exclusively using the secret 32-character hexadecimal `codigo_qr` payload, strictly rejecting public URLs or tokens.

1. The door validation endpoint `POST /api/entradas/checkin` MUST require:
   - `codigo`: string representing the scanned or entered QR secret.
   - `evento_id`: identifier of the active event at the door.
2. The system MUST evaluate `codigo` against the `codigo_qr` column in SQLite.
3. The system MUST NOT accept, look up, or validate a `scan_token` as a valid door check-in credential. Any submission of a `scan_token` MUST result in an invalid verification outcome.

#### Scenario: Valid secret QR code verification
- GIVEN an active event "evt-1" and a paid ticket with secret `codigo_qr = "a1b2c3d4e5f60718293a4b5c6d7e8f90"`
- WHEN the door scanner submits `POST /api/entradas/checkin` with `codigo = "a1b2c3d4e5f60718293a4b5c6d7e8f90"` and `evento_id = "evt-1"`
- THEN the system MUST recognize the credential and proceed to admission processing.

#### Scenario: Reject scan_token submitted as check-in credential
- GIVEN an active event and an unconsumed paid ticket with `scan_token = "tok_xyz789"` and `codigo_qr = "a1b2c3d4e5f60718293a4b5c6d7e8f90"`
- WHEN the client submits `POST /api/entradas/checkin` with `codigo = "tok_xyz789"`
- THEN the system MUST return status "Invalida" and leave the ticket's `estado_checkin = 'No ingresó'`.

---

### Requirement: Three Discrete Validation Outcomes

The system MUST resolve any door check-in verification into exactly one of three discrete outcomes: `Valida`, `Ya Usada`, or `Invalida`.

1. **Outcome `Valida`**:
   - The `codigo_qr` matches an existing ticket belonging to the specified `evento_id`.
   - The ticket has `estado_pago == 'Pagado'`.
   - The ticket has `estado_checkin == 'No ingresó'`.
   - The system MUST update `estado_checkin` to `'Ingresó'`, record `hora_ingreso` with the current ISO timestamp, and return status `Valida` along with buyer name and ticket type.
2. **Outcome `Ya Usada`**:
   - The `codigo_qr` matches a ticket belonging to the specified `evento_id` with `estado_pago == 'Pagado'`.
   - The ticket has `estado_checkin == 'Ingresó'`.
   - The system MUST return status `Ya Usada` including the recorded `hora_ingreso` when the ticket was first used, without modifying any data.
3. **Outcome `Invalida`**:
   - Case A (Unknown code): The code does not exist in `entradas`.
   - Case B (Cross-event): The code belongs to a valid ticket, but for a different `evento_id`. The response MUST specify that the ticket belongs to another event.
   - Case C (Pending payment): The code belongs to the event, but `estado_pago == 'Pendiente'`. The response MUST specify that the ticket has pending payment and cannot enter without payment confirmation.

#### Scenario: Successful first check-in of a paid ticket
- GIVEN a paid ticket for event "evt-1" with `estado_checkin = 'No ingresó'`
- WHEN check-in is requested with the ticket's valid `codigo_qr` for "evt-1"
- THEN the system MUST return `{ "resultado": "Valida", "nombre_comprador": "...", "tipo_entrada": "...", "hora_ingreso": "..." }` and persist `estado_checkin = 'Ingresó'`.

#### Scenario: Re-scanning an already-used ticket
- GIVEN a paid ticket that already entered at "2026-10-15T21:15:00Z" (`estado_checkin = 'Ingresó'`)
- WHEN check-in is requested with the same `codigo_qr`
- THEN the system MUST return `{ "resultado": "Ya Usada", "hora_ingreso": "2026-10-15T21:15:00Z" }` without updating any timestamps.

#### Scenario: Check-in with unknown or malformed code
- GIVEN a code "random-invalid-code-999" that does not exist in SQLite
- WHEN check-in is requested with this code
- THEN the system MUST return `{ "resultado": "Invalida", "motivo": "Codigo no encontrado" }`.

#### Scenario: Reject ticket belonging to a different event
- GIVEN a ticket valid and paid for event "evt-barranco"
- WHEN check-in is requested for active event "evt-miraflores" with that ticket's code
- THEN the system MUST return `{ "resultado": "Invalida", "motivo": "Codigo pertenece a otro evento" }` and perform no check-in.

#### Scenario: Reject pending payment ticket presented at door
- GIVEN a ticket for event "evt-1" with `estado_pago = 'Pendiente'`
- WHEN check-in is requested for "evt-1" with that ticket's code
- THEN the system MUST return `{ "resultado": "Invalida", "motivo": "Pago pendiente" }` and refuse entry.

---

### Requirement: Atomic and Concurrency-Safe Entry Registration

The system MUST execute check-in updates using an atomic conditional write to guarantee that duplicate concurrent scans cannot register duplicate entries.

1. Entry registration MUST be performed via an atomic database condition:
   `UPDATE entradas SET estado_checkin = 'Ingresó', hora_ingreso = ? WHERE id = ? AND estado_checkin = 'No ingresó' AND estado_pago = 'Pagado'`
2. If two concurrent validation requests for the same ticket execute simultaneously:
   - Exactly ONE request MUST succeed and receive `Valida`.
   - The second request MUST observe rows affected = 0, re-evaluate ticket status, and return `Ya Usada`.

#### Scenario: Race-free concurrent check-in on the same code
- GIVEN a paid ticket with `estado_checkin = 'No ingresó'`
- WHEN two concurrent check-in requests are submitted simultaneously for this ticket
- THEN exactly one request MUST receive `{ "resultado": "Valida" }` and the other MUST receive `{ "resultado": "Ya Usada" }`.
