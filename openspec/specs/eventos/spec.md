# Eventos Specification

## Purpose

Defines the lifecycle, configuration, poster asset management, dynamic capacity tracking, and persistence invariants for musical and stage events in Shirley Show. Events represent public or private performances with ticketed admission, operating alongside musical repertoires.

## Requirements

### Requirement: Event Creation and Validation

The system MUST create an event with a non-empty name, date and time, venue, and a validated collection of ticket types.

1. `nombre` MUST be trimmed of leading/trailing whitespace and MUST NOT be empty.
2. `lugar` MUST be trimmed of leading/trailing whitespace and MUST NOT be empty.
3. `fecha_hora` MUST be a valid ISO-8601 date-time string.
4. `tipos_de_entrada` MUST be a non-empty array where each item contains:
   - `nombre`: non-empty trimmed string.
   - `precio`: non-negative floating-point number (`precio >= 0.0`).
   - `capacidad`: positive integer (`capacidad >= 1`).
5. A newly created event MUST have `finalizado = 0` (active) and an empty `imagen` reference.

#### Scenario: Successfully create event with valid ticket types
- GIVEN a payload with name "Concierto Acústico Barranco", venue "La Noche de Barranco", date "2026-10-15T21:00:00Z", and ticket types General (S/ 30.0, 80 seats) and VIP (S/ 50.0, 20 seats)
- WHEN the client requests event creation
- THEN the system MUST persist the event with an assigned unique identifier, active status, and store the validated ticket types.

#### Scenario: Reject event creation with empty name or venue
- GIVEN a creation payload where `nombre` is "   " or `lugar` is ""
- WHEN the client requests event creation
- THEN the system MUST reject the request with a 400 Bad Request error and persist no event records.

#### Scenario: Reject event creation with invalid ticket type pricing or capacity
- GIVEN a creation payload where a ticket type has `precio = -10.0` or `capacidad = 0`
- WHEN the client requests event creation
- THEN the system MUST reject the request with a 400 Bad Request error.

---

### Requirement: Event Modification and Status Toggle (Finalizar / Reabrir)

The system MUST allow updating event metadata and toggling the completion status (`finalizado`) reversibly without losing historical sales or attendance data.

1. Updating an event MUST allow modifying `nombre`, `lugar`, and `fecha_hora` subject to non-empty validation.
2. The completion status MUST toggle between `0` (active) and `1` (finalizado).
3. Finalizing an event MUST preserve all tickets, leads, expenses, check-ins, and financial summaries intact.
4. A finalized event MUST reject new ticket sales and lead conversions unless explicitly reopened.
5. Reopening a finalized event MUST restore its ability to receive ticket sales.

#### Scenario: Finalize an active event
- GIVEN an active event with existing ticket sales and expenses
- WHEN the user marks the event as finalized
- THEN the system MUST update `finalizado = 1` and preserve all existing child records without modification.

#### Scenario: Reject ticket sale on finalized event
- GIVEN an event with `finalizado = 1`
- WHEN a request attempts to sell tickets for that event
- THEN the system MUST reject the sale with a 400 Bad Request error stating that the event is finalized.

#### Scenario: Reopen a finalized event
- GIVEN an event with `finalizado = 1`
- WHEN the user requests to reopen the event
- THEN the system MUST update `finalizado = 0` and permit subsequent ticket sales.

---

### Requirement: Event Poster Storage, Upload Limit, and Safe Replacement

The system MUST store and serve an optional promotional poster image per event, strictly enforcing a 10 MiB upload limit and sanitizing asset paths.

1. The system MUST accept multipart image uploads for an event with a maximum body size of 10 MiB (`10 * 1024 * 1024` bytes).
2. Any upload exceeding 10 MiB MUST be rejected immediately with HTTP 413 Payload Too Large.
3. Allowed image MIME types MUST be restricted to `image/jpeg`, `image/png`, and `image/webp`. Executable or unapproved MIME types MUST be rejected.
4. Uploaded files MUST be stored under a dedicated directory (`data/posters/`) using a server-generated filename format `{evento_id}_{uuid}.{ext}` to prevent path traversal.
5. Uploading a new poster for an event that already has an image MUST delete the previous image file from disk.
6. The system MUST serve the stored poster at `GET /api/eventos/{id}/imagen` with appropriate `Content-Type` and caching headers.

#### Scenario: Upload valid poster within 10 MiB limit
- GIVEN an existing event and a 4 MiB JPEG image file
- WHEN the client uploads the image to `POST /api/eventos/{id}/imagen`
- THEN the system MUST save the file to `data/posters/` with a server-generated UUID filename, update the event record with the filename, and return HTTP 200.

#### Scenario: Reject poster upload exceeding 10 MiB
- GIVEN an existing event and an image file of 11 MiB
- WHEN the client uploads the file to `POST /api/eventos/{id}/imagen`
- THEN the system MUST abort the upload, return HTTP 413 Payload Too Large, and write no file to disk.

#### Scenario: Replace existing poster cleans up previous disk asset
- GIVEN an event with an existing poster `data/posters/evt1_old.jpg`
- WHEN a new valid poster `evt1_new.jpg` is uploaded
- THEN the system MUST store the new poster, update the event record, and remove `data/posters/evt1_old.jpg` from disk.

---

### Requirement: Event Deletion with Cascade and Asset Cleanup

The system MUST delete an event, its associated database records (tickets, leads, expenses), and its on-disk poster file using a filesystem-first order with database cascade and reconciliation backup.

1. If the event has an associated poster file on disk, the system MUST attempt to delete the poster file from the filesystem first.
2. If filesystem poster deletion fails due to an I/O error, the system MUST abort the deletion operation immediately, return HTTP 500 Internal Server Error, and leave all database records (event, tickets, leads, expenses) completely intact.
3. Upon successful deletion of the poster asset (or if the event has no associated poster), the system MUST delete the event from SQLite and cascade-delete all associated rows in `entradas`, `interesados`, and `gastos`.
4. Any orphan poster files resulting from interrupted operations or external inconsistencies MUST be purged by background/startup poster storage reconciliation (`reconcile_orphans`).
5. If the event does not exist, the system MUST return HTTP 404 Not Found without modifying any records or files.

#### Scenario: Delete event with existing tickets, expenses, and poster asset
- GIVEN an event with 10 sold tickets, 3 expenses, 2 leads, and a poster file on disk
- WHEN the client confirms deletion of the event
- THEN the system MUST remove the poster file from `data/posters/` first, delete the event and cascade delete all 10 tickets, 3 expenses, and 2 leads in SQLite, and return HTTP 200.

#### Scenario: Delete non-existent event
- GIVEN an event ID "inexistente-123" that does not exist in SQLite
- WHEN the client requests deletion of that event ID
- THEN the system MUST return HTTP 404 Not Found without modifying any records or files.

#### Scenario: Abort event deletion and keep database intact when filesystem poster deletion fails
- GIVEN an event with an associated poster file where filesystem deletion fails due to an I/O error
- WHEN the client requests deletion of the event
- THEN the system MUST abort the deletion, return HTTP 500 Internal Server Error, and keep the event and all associated database records intact.

---

### Requirement: Dynamic Capacity and Occupancy Calculation

The system MUST compute total capacity and dynamic projected occupancy for any event.

1. Event total capacity MUST be calculated as `sum(capacidad)` across all configured ticket types.
2. Emitted tickets count MUST be the count of all tickets in `entradas` for that event, regardless of payment status (`Pagado` or `Pendiente`).
3. Projected occupancy MUST be calculated as `Emitted Tickets + sum(personas)` of leads in `interesados` with `estado = 'Confirmado'`.
4. The system MUST prevent selling tickets that would cause the emitted tickets for a specific ticket type to exceed its configured `capacidad`.

#### Scenario: Compute event capacity and dynamic occupancy
- GIVEN an event with General capacity 100 and VIP capacity 20 (total capacity 120), 40 emitted tickets (30 paid, 10 pending), and 2 confirmed leads totaling 6 persons
- WHEN the client queries the event summary or detail
- THEN the system MUST report total capacity 120, emitted tickets 40, and dynamic projected occupancy 46.

#### Scenario: Reject ticket sale when requested quantity exceeds available capacity
- GIVEN a ticket type "VIP" with capacity 20 and 18 tickets already emitted
- WHEN a client requests to sell 5 "VIP" tickets
- THEN the system MUST reject the sale with a 400 Bad Request error indicating insufficient capacity (available: 2, requested: 5) and emit zero tickets.
