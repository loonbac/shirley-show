# Gastos Specification

## Purpose

Defines the recording, categorization, validation, and management of production costs and expenditures associated with events in Shirley Show. Accurately tracking expenses is critical for computing real net profit.

## Requirements

### Requirement: Closed Expense Category Catalog

The system MUST restrict expense categorization strictly to the predefined seven production categories.

1. The `categoria` attribute MUST match exactly one of:
   - `'Local/Alquiler'`
   - `'Sonido/Luces'`
   - `'Músicos/Personal'`
   - `'Publicidad/Marketing'`
   - `'Bebidas/Catering'`
   - `'Transporte'`
   - `'Otros'`
2. Any creation or update request with a category outside this closed set MUST be rejected with HTTP 400 Bad Request.

#### Scenario: Register expense with valid category
- GIVEN an active event "evt-1"
- WHEN the user records an expense with category "Sonido/Luces", amount 450.0, description "Alquiler de consola y micrófonos inalámbricos", and date "2026-10-14"
- THEN the system MUST persist the record in `gastos` associated with "evt-1".

#### Scenario: Reject expense with unapproved category
- GIVEN an expense registration request with category "Vestuario" or "Misceláneos"
- WHEN the user submits the expense
- THEN the system MUST reject the request with HTTP 400 Bad Request indicating invalid category.

---

### Requirement: Expense Field Validation and Monetary Invariants

The system MUST validate that expense amounts are non-negative, descriptions are non-empty, and dates follow valid calendar formatting.

1. `monto` MUST be a non-negative number (`monto >= 0.0`). Zero-amount expenses are permitted (e.g. barter or waived costs). Negative amounts MUST be rejected.
2. `descripcion` MUST be trimmed of whitespace and MUST NOT be empty.
3. `fecha` MUST be a valid date string (e.g., "YYYY-MM-DD").
4. `evento_id` MUST reference an existing event.

#### Scenario: Successfully record zero-cost or positive expense
- GIVEN an event "evt-1"
- WHEN an expense of S/ 0.0 is registered for "Bebidas/Catering" with description "Auspicio canje agua"
- THEN the system MUST persist the expense with `monto = 0.0`.

#### Scenario: Reject negative expense amount
- GIVEN an expense payload with `monto = -50.0`
- WHEN the client submits the expense
- THEN the system MUST reject the request with HTTP 400 Bad Request.

#### Scenario: Reject empty description
- GIVEN an expense payload with `descripcion = "   "`
- WHEN the client submits the expense
- THEN the system MUST reject the request with HTTP 400 Bad Request.

---

### Requirement: Expense Modification and Deletion

The system MUST permit updating and deleting recorded expenses, reflecting changes immediately in financial summaries.

1. The system MUST support updating `categoria`, `monto`, `descripcion`, and `fecha` for an existing expense.
2. The system MUST support deleting an individual expense by ID.
3. Deleting an expense MUST reduce the event's total expenses and increase net profit accordingly.

#### Scenario: Update expense details
- GIVEN an existing expense of S/ 200.0 with category "Transporte"
- WHEN the user updates the amount to S/ 250.0
- THEN the system MUST save the updated amount and return the modified expense record.

#### Scenario: Delete expense adjusts event balances
- GIVEN an event with total expenses of S/ 1000.0
- WHEN an expense of S/ 300.0 is deleted
- THEN the system MUST delete the row and the event's total expenses MUST recalculate to S/ 700.0.
