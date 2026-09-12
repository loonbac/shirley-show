# Interesados Specification

## Purpose

Defines the management, tracking, capacity projection, and conversion lifecycle of prospective attendees (leads/interesados) for events in Shirley Show. Interesados allow the production team to track group reservations, promoter attribution, and marketing channels before tickets are officially purchased.

## Requirements

### Requirement: Lead Registration and Attribute Validation

The system MUST record prospective attendees with validated name, contact channel, promoter, party size, and initial status.

1. `nombre` MUST be trimmed and MUST NOT be empty.
2. `contacto` MUST be a non-empty string representing phone, WhatsApp, or email.
3. `canal` MUST represent the communication channel (e.g., "WhatsApp", "Instagram", "Presencial").
4. `personas` MUST be an integer greater than or equal to 1 (`personas >= 1`).
5. `promotora` MUST indicate the promoter or team member responsible for the contact.
6. `estado` MUST belong to the closed set: `'Pendiente'`, `'Confirmado'`, `'Cancelado'`. Default status on creation is `'Pendiente'`.

#### Scenario: Successfully create an interested lead
- GIVEN an active event "evt-1"
- WHEN the user registers a lead with name "Ana Torres", contact "+51987654321", canal "WhatsApp", personas 4, and promotora "Shirley"
- THEN the system MUST persist the record in `interesados` with an assigned ID, `estado = 'Pendiente'`, and timestamp.

#### Scenario: Reject lead with invalid party size or empty name
- GIVEN a lead creation payload with `nombre = ""` or `personas = 0`
- WHEN the client submits the lead registration
- THEN the system MUST reject the request with HTTP 400 Bad Request.

---

### Requirement: Status Lifecycle and Projected Capacity Influence

The system MUST update lead status and reflect confirmed leads in the event's dynamic capacity projection.

1. The system MUST allow updating lead fields: `nombre`, `contacto`, `canal`, `personas`, `promotora`, and `estado`.
2. When a lead transitions to `estado = 'Confirmado'`:
   - Its `personas` count MUST be immediately included in the event's dynamic projected occupancy calculation (`entradas emitidas + sum(personas de confirmados)`).
3. When a lead transitions to `'Cancelado'` or `'Pendiente'`:
   - Its `personas` count MUST NOT be included in the confirmed dynamic capacity projection.

#### Scenario: Confirming lead updates projected occupancy
- GIVEN an event with 20 emitted tickets and no confirmed leads
- WHEN a lead with 5 persons transitions to `estado = 'Confirmado'`
- THEN the event's projected occupancy MUST calculate as 25 (20 emitted + 5 confirmed lead persons).

#### Scenario: Canceling a lead removes it from projected occupancy
- GIVEN a confirmed lead with 4 persons in an event with 30 projected occupancy
- WHEN the lead's status is changed to `'Cancelado'`
- THEN the event's projected occupancy MUST decrease to 26.

---

### Requirement: Conversion of Lead to Ticket Sale (Pase a Venta)

The system MUST convert a lead into a completed ticket sale using the exact transactional issuance use case of the `entradas` domain, preventing duplicate sales or orphaned states.

1. The conversion action MUST invoke the transactional ticket issuance case with:
   - `evento_id` matching the lead's event.
   - `nombre_comprador` derived from the lead's name.
   - `contacto` derived from the lead's contact info.
   - `tipo_entrada`, `medio_pago`, and `estado_pago` specified by the operator.
   - `cantidad` matching the lead's `personas` count (or operator-adjusted quantity $\ge 1$).
2. The conversion MUST verify that the requested ticket type has sufficient remaining capacity.
3. Upon successful ticket issuance:
   - $N$ tickets MUST be created in `entradas`.
   - The lead's status MUST update to `'Confirmado'`.
4. If ticket issuance fails for any reason (insufficient capacity, invalid parameters, transaction error):
   - The transaction MUST abort completely.
   - No tickets MUST be created.
   - The lead MUST remain in its previous status without corruption.
5. A lead that has already been converted to a sale MUST NOT be converted again.

#### Scenario: Successful conversion of lead to ticket sale
- GIVEN a pending lead with 3 persons for event "evt-1"
- WHEN the operator executes conversion to 3 "General" tickets paid via "Yape"
- THEN the system MUST atomically emit 3 tickets in `entradas`, update the lead's status to `'Confirmado'`, and return the emitted tickets.

#### Scenario: Conversion failure leaves lead unchanged
- GIVEN a lead with 4 persons and an event ticket type with only 2 seats remaining
- WHEN the operator attempts to convert the lead to 4 tickets of that type
- THEN the system MUST reject the request due to capacity, create 0 tickets, and keep the lead with its original pending status.

#### Scenario: Reject duplicate conversion attempt
- GIVEN a lead that was already converted to tickets
- WHEN an operator attempts to convert the same lead a second time
- THEN the system MUST reject the request with HTTP 400 Bad Request or HTTP 409 Conflict.
