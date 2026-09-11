# Ressy Ticketing Specification

## Purpose

Defines conversational assistant capabilities, tool schemas, and operational guardrails for Shirley Show's AI assistant (Ressy) within the ticketing and event finance domain. Ressy executes domain use cases on behalf of the artist while enforcing accounting truthfulness and contextual grounding.

## Requirements

### Requirement: Fixed Eight Tool Allowlist and Dispatcher

The system MUST provide exactly eight fixed operational ticketing tools to Ressy, invoking the identical application use cases used by the REST API.

1. The assistant tool dispatcher MUST register and dispatch:
   - `crear_evento`: Parameters `nombre` (string), `fecha_hora` (ISO string), `lugar` (string), `tipos_de_entrada` (array of `{ nombre, precio, capacidad }`).
   - `listar_eventos`: Parameters `incluir_finalizados` (optional boolean).
   - `vender_entradas`: Parameters `evento_id` (string), `nombre_comprador` (string), `contacto` (string), `tipo_entrada` (string), `cantidad` (integer $\ge 1$), `medio_pago` (`"Yape"` | `"Efectivo"` | `"Cortesía"`), `estado_pago` (`"Pagado"` | `"Pendiente"`).
   - `confirmar_pago_entrada`: Parameters `entrada_id` (string).
   - `registrar_gasto`: Parameters `evento_id` (string), `categoria` (one of the 7 closed categories), `monto` (number $\ge 0$), `descripcion` (string).
   - `resumen_evento`: Parameters `evento_id` (string).
   - `registrar_interesado`: Parameters `evento_id` (string), `nombre` (string), `contacto` (string), `canal` (string), `personas` (integer $\ge 1$), `promotora` (string).
   - `checkin_entrada`: Parameters `evento_id` (string), `codigo` (string, 32-hex QR secret).
2. Any invocation of an unknown tool name MUST be rejected with a structured error.

#### Scenario: Dispatch vender_entradas tool call
- GIVEN an active event "evt-123" with ticket type "General" (S/ 25.0)
- WHEN Ressy executes tool `vender_entradas` with `evento_id = "evt-123"`, `nombre_comprador = "Luis Silva"`, `contacto = "998877665"`, `tipo_entrada = "General"`, `cantidad = 2`, `medio_pago = "Yape"`, `estado_pago = "Pagado"`
- THEN the system MUST execute the transactional multi-ticket issuance case, persist 2 tickets, and return their public tokens and generated WhatsApp share links.

#### Scenario: Dispatch checkin_entrada tool call
- GIVEN an active event and a secret QR code "c0d1g0hex1234567890abcdef123456"
- WHEN Ressy executes tool `checkin_entrada` with `evento_id = "evt-123"` and `codigo = "c0d1g0hex1234567890abcdef123456"`
- THEN the system MUST execute door verification and return the entry result (`Valida`, `Ya Usada`, or `Invalida`).

---

### Requirement: Grounded Context and Anti-Hallucination Invariants

Ressy MUST NOT invent or fabricate event identifiers, ticket identifiers, prices, or capacities.

1. Ressy MUST only supply `evento_id` values that exist in the active conversation context or were returned by a prior `listar_eventos` tool execution.
2. If `evento_id` is unknown, ambiguous, or not present in context, Ressy MUST call `listar_eventos` or ask the user which event they wish to operate on.
3. Ressy MUST NOT invent ticket prices or ticket types. Prices and type names MUST match the configured ticket types of the target event.
4. For courtesy tickets (`medio_pago = "Cortesía"`), Ressy MUST always specify `precio = 0.0` and `estado_pago = "Pagado"`.

#### Scenario: Unknown event context triggers clarification or listing
- GIVEN a turn where the user asks "Regístrame un gasto de 100 soles por sonido" but no event is selected or discussed
- WHEN Ressy processes the user request
- THEN Ressy MUST either invoke `listar_eventos` to identify the current active event or ask the user which event the expense belongs to, before calling `registrar_gasto`.

#### Scenario: Courtesy ticket enforced at zero price
- GIVEN the user asks "Dale 1 entrada de cortesía a Roberto para el acústico"
- WHEN Ressy calls `vender_entradas`
- THEN the tool arguments MUST specify `medio_pago = "Cortesía"`, `estado_pago = "Pagado"`, and price S/ 0.0.

---

### Requirement: Conversational Clarification Before Destructive or Financial Action

Ressy MUST seek clarification when a user prompt lacks necessary parameters for selling tickets or recording expenses, rather than making arbitrary assumptions.

1. If the user requests a ticket sale without specifying the buyer name, ticket type, or quantity, Ressy MUST ask for the missing details before executing `vender_entradas`.
2. If the user mentions multiple matching events, Ressy MUST ask the user to clarify which event before executing the sale or registration.

#### Scenario: Incomplete ticket sale prompt triggers question
- GIVEN the user says "Véndele entradas a Juan"
- WHEN Ressy processes the message
- THEN Ressy MUST respond asking which event, which ticket type, and how many tickets to issue, without calling `vender_entradas`.

---

### Requirement: Accounting Truthfulness in Conversational Summaries

Ressy MUST distinguish between collected funds and pending reservations in all conversational responses.

1. When reporting on event finances, Ressy MUST state `Cobrado` as actual collected money and `Pendiente` as uncollected reservations.
2. Ressy MUST NOT report `Total Proyectado` or pending reservations as realized revenue or net profit.
3. Upon successfully issuing tickets, Ressy's response MUST provide the direct WhatsApp share link for each buyer to facilitate one-tap delivery.

#### Scenario: Financial summary report distinguishes cobrado from pendiente
- GIVEN an event with S/ 500.0 cobrado, S/ 200.0 pendiente, and S/ 150.0 gastos
- WHEN the user asks "Ressy, ¿cómo vamos con el evento?"
- THEN Ressy MUST call `resumen_evento` and formulate a response stating clearly that S/ 500.0 has been collected, S/ 200.0 is still pending collection, expenses are S/ 150.0, and current net profit is S/ 350.0.
