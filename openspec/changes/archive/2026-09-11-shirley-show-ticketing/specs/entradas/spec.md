# Entradas Specification

## Purpose

Defines the transactional issuance, dual-token security model, payment lifecycle, search/filtering, and cancellation rules for admission tickets in Shirley Show. Each ticket represents an individual entry credential for a specific event.

## Requirements

### Requirement: Multi-Row Transactional Ticket Issuance

The system MUST issue multi-ticket orders by creating exactly $N$ individual ticket rows within a single atomic SQLite transaction, ensuring all-or-nothing execution.

1. A sale request for quantity $N$ ($N \ge 1$) MUST insert exactly $N$ distinct rows into the `entradas` table.
2. If any single insertion, constraint check, or capacity limit fails during the sale, the entire transaction MUST be rolled back, leaving zero tickets persisted.
3. Each emitted ticket MUST receive:
   - A unique persistent `id`.
   - A cryptographically random, unguessable `scan_token` (used exclusively for the public web view URL).
   - An independent secret 32-character hexadecimal `codigo_qr` (used exclusively for door scanning).
4. `scan_token` and `codigo_qr` MUST be completely distinct and uncorrelated values.
5. Quantity $N < 1$ MUST be rejected with HTTP 400 Bad Request.

#### Scenario: Successful multi-ticket issuance creates N independent rows
- GIVEN an active event with 10 available "General" tickets priced at S/ 30.0
- WHEN the user purchases 3 tickets for buyer "Carlos Mendoza" via Yape with status "Pagado"
- THEN the system MUST persist exactly 3 distinct ticket rows, each with unique `id`, unique `scan_token`, unique `codigo_qr`, `precio = 30.0`, `medio_pago = 'Yape'`, `estado_pago = 'Pagado'`, and `estado_checkin = 'No ingresó'`.

#### Scenario: Transactional rollback on capacity exhaustion
- GIVEN a ticket type with only 2 available spots remaining
- WHEN a client attempts to purchase 3 tickets in a single request
- THEN the system MUST reject the request, roll back the transaction, and create 0 ticket rows in the database.

#### Scenario: Reject non-positive ticket quantity
- GIVEN a sale request with quantity `0` or `-2`
- WHEN the client submits the purchase
- THEN the system MUST return HTTP 400 Bad Request and perform no database writes.

---

### Requirement: Payment Medium and Status Invariants

The system MUST enforce strict validation rules on payment mediums and payment statuses.

1. `medio_pago` MUST be one of the closed set: `'Yape'`, `'Efectivo'`, `'Cortesía'`. Any other value MUST be rejected.
2. `estado_pago` MUST be one of: `'Pagado'`, `'Pendiente'`.
3. If `medio_pago == 'Cortesía'`, the system MUST enforce `precio = 0.0` and `estado_pago = 'Pagado'`. A courtesy ticket CANNOT be marked as 'Pendiente' or have a non-zero price.
4. If `medio_pago` is `'Yape'` or `'Efectivo'`, `estado_pago` MAY be `'Pagado'` or `'Pendiente'`.

#### Scenario: Courtesy ticket automatically sets zero price and paid status
- GIVEN an active event with available tickets
- WHEN a ticket is issued with `medio_pago = 'Cortesía'` and any input price
- THEN the system MUST record the ticket with `precio = 0.0` and `estado_pago = 'Pagado'`.

#### Scenario: Pending ticket reservation via Efectivo
- GIVEN an active event with available tickets
- WHEN a ticket is issued with `medio_pago = 'Efectivo'`, `precio = 25.0`, and `estado_pago = 'Pendiente'`
- THEN the system MUST persist the ticket with `estado_pago = 'Pendiente'`.

#### Scenario: Reject unknown payment medium
- GIVEN a sale request with `medio_pago = 'Tarjeta'` or `medio_pago = 'Transferencia'`
- WHEN the client submits the sale
- THEN the system MUST reject the request with HTTP 400 Bad Request.

---

### Requirement: Payment Confirmation Workflow

The system MUST allow manual confirmation of pending tickets to transition them to paid status without affecting check-in state.

1. Confirming payment MUST update `estado_pago` from `'Pendiente'` to `'Pagado'`.
2. Confirming payment MUST NOT alter `estado_checkin` (it MUST remain `'No ingresó'`).
3. Calling payment confirmation on an already `'Pagado'` ticket MUST be idempotent and return HTTP 200 without error or modification.
4. Confirming payment on a non-existent ticket ID MUST return HTTP 404 Not Found.

#### Scenario: Transition pending ticket to paid
- GIVEN a ticket with `estado_pago = 'Pendiente'` and `estado_checkin = 'No ingresó'`
- WHEN the user executes `POST /api/entradas/{id}/confirmar-pago`
- THEN the system MUST update `estado_pago = 'Pagado'`, leave `estado_checkin = 'No ingresó'`, and return the updated ticket representation.

#### Scenario: Idempotent confirmation on already paid ticket
- GIVEN a ticket with `estado_pago = 'Pagado'`
- WHEN the user executes `POST /api/entradas/{id}/confirmar-pago`
- THEN the system MUST return HTTP 200 and maintain `estado_pago = 'Pagado'`.

---

### Requirement: QR Concealment for Pending Tickets

The system MUST NOT generate or disclose any scannable QR code asset or data for tickets whose payment status is `'Pendiente'`.

1. If a ticket has `estado_pago == 'Pendiente'`, all administrative and public API responses MUST omit the `qr_svg` payload or set it to `null`.
2. The server MUST NOT render or expose the `codigo_qr` value to public consumers while the ticket is `'Pendiente'`.
3. Only when `estado_pago == 'Pagado'`, the system MUST generate and deliver the vector QR SVG containing exclusively the `codigo_qr` payload.

#### Scenario: Pending ticket payload excludes QR code
- GIVEN a ticket in SQLite with `estado_pago = 'Pendiente'`
- WHEN the ticket is retrieved via the public ticket API or administrative ticket detail
- THEN the response MUST NOT contain any QR SVG markup or data URI.

#### Scenario: Paid ticket delivers vector QR code
- GIVEN a ticket in SQLite with `estado_pago = 'Pagado'`
- WHEN the ticket is retrieved via the public ticket API
- THEN the response MUST contain valid `<svg ...>` markup encoding the ticket's secret `codigo_qr`.

---

### Requirement: Ticket Search, Filtering, and Listing

The system MUST provide filtered and searched listings of tickets for any event.

1. The system MUST support listing tickets for an event with pagination or complete bounded list.
2. The search query parameter `q` MUST match case-insensitively against `nombre_comprador` and `contacto`.
3. The query parameter `estado_pago` MUST filter by exact match on `'Pagado'` or `'Pendiente'`.
4. The query parameter `tipo_entrada` MUST filter by exact match on ticket type name.

#### Scenario: Search tickets by buyer name substring
- GIVEN an event with tickets for "María García", "Mario Ruiz", and "Lucía Vargas"
- WHEN the user queries `/api/eventos/{id}/entradas?q=mar`
- THEN the system MUST return tickets for "María García" and "Mario Ruiz", excluding "Lucía Vargas".

#### Scenario: Filter tickets by pending payment
- GIVEN an event with 5 paid tickets and 3 pending tickets
- WHEN the user queries `/api/eventos/{id}/entradas?estado_pago=Pendiente`
- THEN the system MUST return only the 3 pending tickets.

---

### Requirement: Ticket Cancellation and Deletion Guardrails

The system MUST permit deleting unconsumed tickets while preventing deletion of tickets that have already completed door check-in.

1. Deleting an unconsumed ticket (`estado_checkin == 'No ingresó'`) MUST remove the record from `entradas` and adjust the event's emitted ticket count.
2. Deleting a ticket that has already been validated at the door (`estado_checkin == 'Ingresó'`) MUST be rejected with HTTP 409 Conflict to preserve door audit trails.

#### Scenario: Delete unconsumed ticket
- GIVEN an existing ticket with `estado_checkin = 'No ingresó'`
- WHEN the client requests deletion via `DELETE /api/entradas/{id}`
- THEN the system MUST delete the ticket row and return HTTP 200 or 204.

#### Scenario: Reject deletion of already-entered ticket
- GIVEN an existing ticket with `estado_checkin = 'Ingresó'`
- WHEN the client requests deletion via `DELETE /api/entradas/{id}`
- THEN the system MUST reject the request with HTTP 409 Conflict and preserve the ticket record in SQLite.
