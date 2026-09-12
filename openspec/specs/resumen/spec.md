# Resumen Specification

## Purpose

Defines the mathematical consolidation, financial reporting, and operational capacity metrics for events in Shirley Show. Resumen guarantees strict parity with production accounting invariants, distinguishing verified cash flow from uncollected reservations.

## Requirements

### Requirement: Seven Core Event Metrics Calculation

The system MUST compute and report the exact seven production parity metrics for any specified event.

1. **`Cobrado`**:
   The exact sum of `precio` across all tickets of the event where `estado_pago == 'Pagado'`.
2. **`Pendiente`**:
   The exact sum of `precio` across all tickets of the event where `estado_pago == 'Pendiente'`.
3. **`Total Proyectado` (Ventas Brutas)**:
   The arithmetic sum of `Cobrado + Pendiente`.
4. **`Gastos`**:
   The exact sum of `monto` across all expenses recorded for the event.
5. **`Ganancia Neta`**:
   `Cobrado - Gastos`. The system MUST NOT include uncollected `Pendiente` amounts in net profit.
6. **`Capacidad y Entradas Emitidas`**:
   - `capacidad_total`: sum of configured capacities across all ticket types for the event.
   - `entradas_emitidas`: total count of persisted tickets (`Pagado` + `Pendiente`).
7. **`Aforo Proyectado`**:
   `entradas_emitidas + sum(personas)` of leads in `interesados` with `estado == 'Confirmado'`.

#### Scenario: Full financial metrics calculation
- GIVEN an event with:
  - 10 tickets sold at S/ 30.0 paid via Yape (Cobrado = S/ 300.0)
  - 4 tickets reserved at S/ 30.0 pending payment (Pendiente = S/ 120.0)
  - 2 courtesy tickets at S/ 0.0
  - Expenses totaling S/ 180.0
  - Total venue capacity of 50 seats
  - 1 confirmed lead with 3 persons
- WHEN the client requests `GET /api/eventos/{id}/resumen`
- THEN the system MUST return:
  - `cobrado = 300.0`
  - `pendiente = 120.0`
  - `total_proyectado = 420.0`
  - `gastos = 180.0`
  - `ganancia_neta = 120.0` (300.0 - 180.0)
  - `capacidad_total = 50`
  - `entradas_emitidas = 16` (10 + 4 + 2)
  - `aforo_proyectado = 19` (16 + 3).

#### Scenario: Empty event metrics initialization
- GIVEN a freshly created event with no tickets, no leads, and no expenses
- WHEN the client requests the summary
- THEN the system MUST report `cobrado = 0.0`, `pendiente = 0.0`, `total_proyectado = 0.0`, `gastos = 0.0`, `ganancia_neta = 0.0`, `entradas_emitidas = 0`, and `aforo_proyectado = 0`.

---

### Requirement: Payment Method and Revenue Breakdown

The system MUST provide an itemized breakdown of collected and pending funds grouped by payment medium.

1. The breakdown MUST separate revenue for:
   - `'Yape'`: total amount cobrado, total amount pendiente, count of tickets.
   - `'Efectivo'`: total amount cobrado, total amount pendiente, count of tickets.
   - `'Cortesía'`: count of courtesy tickets issued, with cobrado = 0.0 and pendiente = 0.0.
2. The sum of breakdown amounts MUST reconcile exactly with the top-level `Cobrado` and `Pendiente` values.

#### Scenario: Verify payment method breakdown reconciliation
- GIVEN an event with S/ 200.0 paid via Yape, S/ 100.0 paid via Efectivo, S/ 50.0 pending Efectivo, and 3 cortesías
- WHEN the summary is queried
- THEN the response breakdown MUST show Yape cobrado 200.0, Efectivo cobrado 100.0, Efectivo pendiente 50.0, and Cortesías count 3, reconciling exactly to top-level cobrado 300.0 and pendiente 50.0.

---

### Requirement: Categorized Expense Breakdown

The system MUST report the distribution of expenditures across the seven closed expense categories.

1. The summary MUST list the total expenditure for each of the 7 production categories:
   - `Local/Alquiler`
   - `Sonido/Luces`
   - `Músicos/Personal`
   - `Publicidad/Marketing`
   - `Bebidas/Catering`
   - `Transporte`
   - `Otros`
2. Categories with no recorded expenses MUST report `0.0`.
3. The sum of all category totals MUST equal the top-level `Gastos` value.

#### Scenario: Verify expense breakdown across categories
- GIVEN an event with expenses: S/ 500.0 in "Local/Alquiler" and S/ 300.0 in "Sonido/Luces"
- WHEN the summary is queried
- THEN the response MUST list `Local/Alquiler: 500.0`, `Sonido/Luces: 300.0`, all other 5 categories as `0.0`, and top-level `gastos = 800.0`.
