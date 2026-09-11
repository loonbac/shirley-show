# Entrada Pública Specification

## Purpose

Defines the unauthenticated mobile-first public ticket view and WhatsApp sharing capabilities in Shirley Show. Attendees access their digital tickets via public tokens, presenting vector QR codes at the door only after payment is confirmed.

## Requirements

### Requirement: Public Ticket View by Scan Token

The system MUST serve ticket details to public attendees via the unguessable `scan_token` without requiring login or session credentials.

1. The public ticket endpoint `GET /api/public/ticket/{scan_token}` MUST be accessible without authentication headers, cookies, or administrative privileges.
2. The system MUST retrieve the corresponding ticket record by matching `scan_token`.
3. If no ticket matches the provided `scan_token`, the system MUST return HTTP 404 Not Found.

#### Scenario: Attendee opens ticket via valid scan token
- GIVEN an existing ticket with `scan_token = "tok_abc12345"` for event "Concierto Acústico"
- WHEN the browser requests `GET /api/public/ticket/tok_abc12345`
- THEN the system MUST return HTTP 200 with event and ticket display details.

#### Scenario: Attendee navigates to non-existent or invalid scan token
- GIVEN a request to `GET /api/public/ticket/tok_inexistente`
- WHEN the token is queried in SQLite
- THEN the system MUST return HTTP 404 Not Found.

---

### Requirement: Minimal Safe Public Data Surface

The system MUST restrict the public ticket endpoint response to only necessary attendee display fields, concealing internal keys, administrative data, and plaintext QR secrets.

1. The public response payload MUST contain only:
   - Event attributes: `nombre_evento`, `fecha_hora`, `lugar`.
   - Ticket attributes: `nombre_comprador`, `tipo_entrada`, `precio`, `estado_pago`, `estado_checkin`.
   - Credential payload: `qr_svg` (conditionally provided based on payment status).
2. The public response MUST NOT disclose:
   - Internal database IDs (`id`, `evento_id`).
   - Plaintext `codigo_qr` string.
   - Financial totals, expenses, or summaries of the event.
   - Information about any other attendees.

#### Scenario: Inspect public ticket response payload
- GIVEN a paid ticket in SQLite
- WHEN the public ticket endpoint is queried
- THEN the response JSON MUST NOT contain the keys `"codigo_qr"`, `"evento_id"`, or financial aggregates.

---

### Requirement: Conditional QR Rendering and Payment Pending State

The system and frontend MUST condition the generation and display of the scannable QR code strictly on verified payment status.

1. If `estado_pago == 'Pagado'`:
   - The backend MUST return a valid vector `<svg ...>` string in `qr_svg` encoding the ticket's secret `codigo_qr`.
   - The public frontend view MUST render the QR code prominently alongside instructions to present it at the door.
2. If `estado_pago == 'Pendiente'`:
   - The backend MUST set `qr_svg = null` or omit it.
   - The public frontend view MUST render an amber "Reserva Pendiente de Pago" banner.
   - The frontend view MUST display payment instructions (e.g. Yape number and recipient) and MUST NOT render any QR code graphic.

#### Scenario: Public view of paid ticket displays vector QR
- GIVEN a ticket with `estado_pago = 'Pagado'`
- WHEN an attendee loads `#/ticket/{scan_token}`
- THEN the interface MUST display the event details and a scannable SVG QR code.

#### Scenario: Public view of pending ticket displays instructions and no QR
- GIVEN a ticket with `estado_pago = 'Pendiente'`
- WHEN an attendee loads `#/ticket/{scan_token}`
- THEN the interface MUST display "Reserva Pendiente", indicate the outstanding amount to pay, and display no scannable QR code.

---

### Requirement: Client-Side WhatsApp Link Construction (wa.me)

The frontend application MUST generate direct `wa.me` sharing URLs locally in the browser without server-side WhatsApp daemons or third-party messaging services.

1. The frontend MUST format WhatsApp URLs as:
   `https://wa.me/{telefono_limpio}?text={mensaje_url_encoded}`
2. Phone number cleaning MUST strip non-digit characters (`+`, `-`, spaces, parentheses). If the cleaned number has 9 digits without a country code, the Peruvian prefix `51` MUST be prepended.
3. For a paid ticket, the message MUST include:
   - The attendee's name.
   - The event name.
   - The full public ticket URL (`{origin}/#/ticket/{scan_token}`).
4. For a pending reservation, the message MUST include:
   - The attendee's name.
   - The event name and pending amount.
   - The public reservation URL and payment instructions.

#### Scenario: Generate WhatsApp link for paid ticket
- GIVEN an issued paid ticket for "Valeria Salas" with `scan_token = "tok_val123"` and contact "987654321" for event "Cena Show"
- WHEN the user clicks the WhatsApp button in the ticket list
- THEN the browser MUST construct and open a URL beginning with `https://wa.me/51987654321?text=` containing the attendee's name, event name, and the link `#/ticket/tok_val123`.

#### Scenario: Generate WhatsApp link for pending reservation
- GIVEN a pending ticket for "Rodrigo Ruiz" with contact "+51 912 345 678" for S/ 40.0
- WHEN the user clicks the WhatsApp button for this reservation
- THEN the browser MUST generate a URL targeting `51912345678` with text prompting payment confirmation and linking to `#/ticket/{scan_token}`.
