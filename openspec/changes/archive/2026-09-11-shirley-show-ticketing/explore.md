# SDD Exploration: `shirley-show-ticketing`

**Change**: `shirley-show-ticketing`  
**Date**: 2026-09-11  
**Status**: Completed  
**Author**: Gentle AI (SDD Explore Phase)  
**Artifact Store**: `openspec` (`openspec/changes/shirley-show-ticketing/explore.md`)  
**Delivery Strategy**: `ask-on-risk` (Budget: 400 líneas / slice)  

---

## 1. Visión y Propósito del Cambio

**Shirley Show** amplía sus capacidades de escenario incorporando un sistema integrado de **Ticketing y Finanzas de Eventos**. La propuesta de valor central se mantiene intacta: el foco principal de la artista y la aplicación sigue siendo el repertorio musical y el rendimiento en vivo ("Shows"), pero ahora se añade una segunda sección hermana ("Eventos") para la venta de entradas, control de interesados, gastos de producción, balance financiero y validación de acceso en puerta.

### Principios Rectores:
1. **Prioridad Musical Preservada:** La sección "Presentaciones / Repertorios" continúa siendo la primera opción y flujo predeterminado. "Eventos" se sitúa al mismo nivel jerárquico como una pestaña o sección complementaria.
2. **Operación Asistida por IA (Ressy):** Todo el trabajo manual y repetitivo de ticketing (registrar ventas, emitir entradas, confirmar pagos, anotar gastos y consultar balances) puede gestionarse conversacionalmente mediante Ressy, su asistente inteligente.
3. **Simplicidad Operativa sin Fricción:** Sin cuentas de usuario ni login (uso exclusivo de la artista y su equipo directo), sin pasarelas de pago externas (control de Yape / Efectivo / Cortesía manual), y sin bots de WhatsApp en el servidor (generación de enlaces `https://wa.me/...` directos para un solo toque desde el navegador).

---

## 2. Insumo de Producción (Mapeo de Entidades y Reglas)

Del análisis del sistema de producción (Go+React en `production/master` y datos en `production-data/`), se extraen las siguientes reglas de negocio e invariantes esenciales:

- **Entidades de Dominio:**
  - `eventos`: id, nombre, fecha_hora, lugar, tipos_de_entrada (JSON array), finalizado (bool), imagen (afiche opcional), fecha_creacion.
  - `entradas`: id, evento_id, nombre_comprador, contacto, tipo_entrada, precio, medio_pago (`Yape`, `Efectivo`, `Cortesía`), estado_pago (`Pagado`, `Pendiente`), codigo_qr (secreto 32-hex), scan_token (token público de acceso), estado_checkin (`No ingresó`, `Ingresó`), hora_ingreso.
  - `interesados`: id, evento_id, nombre, contacto, canal, personas (mínimo 1), promotora, estado (`Pendiente`, `Confirmado`, `Cancelado`), fecha_creacion.
  - `gastos`: id, evento_id, categoria (cerrada), monto, descripcion, fecha, fecha_creacion.
  - `usuarios` (Auth): **EXCLUIDO EXPLÍCITAMENTE** (sin login).
- **Venta Multi-Fila:** Cada venta crea exactamente $N$ filas en `entradas` (1 fila por entrada individual), cada una con su propio `id`, `scan_token` y `codigo_qr` único.
- **Invariante de Seguridad - Doble Token (`scan_token` vs `codigo_qr`):**
  - El enlace web que se entrega al comprador utiliza el `scan_token` (`#/ticket/{scan_token}`).
  - El gráfico del código QR codifica internamente el `codigo_qr` (32 caracteres hexadecimales).
  - La puerta de ingreso solo valida `codigo_qr`. Nadie puede validar una entrada con solo conocer la URL de visualización.
- **Invariante de Pago - QR Oculto si Pendiente:**
  - Si una entrada tiene `estado_pago == 'Pendiente'`, el código QR **NUNCA** se genera ni se expone (ni en API ni en UI). Solo se muestra la instrucción de pago.
- **Métricas Financieras:**
  - `Cobrado` = $\sum \text{precio}$ de entradas con `estado_pago = 'Pagado'`.
  - `Pendiente` = $\sum \text{precio}$ de entradas con `estado_pago = 'Pendiente'`.
  - `Gastos` = $\sum \text{monto}$ de gastos del evento.
  - `Ganancia Neta` = `Cobrado` - `Gastos`.
- **Aforo Dinámico:**
  - Capacidad ocupada/emitida: conteo de entradas emitidas.
  - Aforo dinámico proyectado: $\text{Entradas Emitidas} + \sum \text{personas}$ de interesados confirmados.

---

## 3. Mapeo a la Arquitectura Rust + Axum (Backend)

Conforme a `AGENTS.md` (modularidad cohesiva y responsabilidades explícitas), el backend extiende el monolito modular existente sin reescrituras innecesarias.

### 3.1 Base de Datos y Migración (`migrations/0002_ticketing.sql`)

Se agregará la migración `0002_ticketing.sql`, empaquetada mediante `include_str!` en `src/infra/sqlite/migrate.rs`:

```sql
-- Eventos
CREATE TABLE IF NOT EXISTS eventos (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL CHECK (length(trim(nombre)) > 0),
  fecha_hora TEXT NOT NULL,
  lugar TEXT NOT NULL CHECK (length(trim(lugar)) > 0),
  tipos_de_entrada TEXT NOT NULL, -- JSON: [{"nombre":"General","precio":25.0,"capacidad":100}]
  finalizado INTEGER NOT NULL DEFAULT 0 CHECK (finalizado IN (0, 1)),
  imagen TEXT, -- Nombre del archivo en disco (data/posters/)
  fecha_creacion TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_eventos_fecha ON eventos(fecha_hora);

-- Entradas individuales (1 fila por entrada)
CREATE TABLE IF NOT EXISTS entradas (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
  nombre_comprador TEXT NOT NULL CHECK (length(trim(nombre_comprador)) > 0),
  contacto TEXT NOT NULL,
  tipo_entrada TEXT NOT NULL,
  precio REAL NOT NULL CHECK (precio >= 0.0),
  medio_pago TEXT NOT NULL CHECK (medio_pago IN ('Yape', 'Efectivo', 'Cortesía')),
  estado_pago TEXT NOT NULL CHECK (estado_pago IN ('Pagado', 'Pendiente')),
  codigo_qr TEXT NOT NULL UNIQUE, -- Secreto 32-hex escaneado en puerta
  scan_token TEXT NOT NULL UNIQUE, -- Token URL público para el visor del ticket
  estado_checkin TEXT NOT NULL DEFAULT 'No ingresó' CHECK (estado_checkin IN ('No ingresó', 'Ingresó')),
  hora_ingreso TEXT,
  fecha_creacion TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_entradas_evento ON entradas(evento_id);
CREATE INDEX IF NOT EXISTS idx_entradas_scan_token ON entradas(scan_token);
CREATE INDEX IF NOT EXISTS idx_entradas_codigo_qr ON entradas(codigo_qr);

-- Interesados / Leads
CREATE TABLE IF NOT EXISTS interesados (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
  nombre TEXT NOT NULL CHECK (length(trim(nombre)) > 0),
  contacto TEXT NOT NULL,
  canal TEXT NOT NULL,
  personas INTEGER NOT NULL DEFAULT 1 CHECK (personas >= 1),
  promotora TEXT NOT NULL,
  estado TEXT NOT NULL DEFAULT 'Pendiente' CHECK (estado IN ('Pendiente', 'Confirmado', 'Cancelado')),
  fecha_creacion TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_interesados_evento ON interesados(evento_id);

-- Gastos de Producción
CREATE TABLE IF NOT EXISTS gastos (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
  categoria TEXT NOT NULL CHECK (categoria IN (
    'Local/Alquiler',
    'Sonido/Luces',
    'Músicos/Personal',
    'Publicidad/Marketing',
    'Bebidas/Catering',
    'Transporte',
    'Otros'
  )),
  monto REAL NOT NULL CHECK (monto >= 0.0),
  descripcion TEXT NOT NULL CHECK (length(trim(descripcion)) > 0),
  fecha TEXT NOT NULL,
  fecha_creacion TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gastos_evento ON gastos(evento_id);
```

### 3.2 Generación de QR con crate `qrcode`

- **Dependencia:** Añadir `qrcode = { version = "0.14", default-features = false, features = ["svg"] }` a `Cargo.toml`.
- **Ventaja Técnica:** Produce cadenas `<svg ...>` vectoriales puras en memoria, sin requerir bindings C ni librerías de compresión PNG. Rendimiento instantáneo y compatible de forma nativa con entornos NixOS / Linux herméticos.
- **Contrato de Entrega:** La API pública `GET /api/public/ticket/{scan_token}` solo incluye el SVG de QR si `estado_pago == "Pagado"`.

### 3.3 Endpoints REST (Axum)

1. **Eventos:**
   - `GET /api/eventos`: Lista de eventos con contadores resumidos (aforo, ganancia).
   - `POST /api/eventos`: Creación de evento con validación de tipos de entrada.
   - `GET /api/eventos/{id}`: Detalle de evento con tipos de entrada y estado.
   - `PATCH /api/eventos/{id}`: Actualización de datos (lugar, fecha, nombre).
   - `DELETE /api/eventos/{id}`: Eliminación de evento y cascada.
   - `POST /api/eventos/{id}/finalizar`: Alternar estado finalizado (cerrar o reabrir evento).
   - `POST /api/eventos/{id}/imagen`: Carga multipart de afiche con tope de 10 MiB (`DefaultBodyLimit::max(10 * 1024 * 1024)`).
   - `GET /api/eventos/{id}/imagen`: Servido de afiche con encabezados `Content-Type` y `Cache-Control`.
   - `GET /api/eventos/{id}/resumen`: Resumen financiero detallado (Cobrado, Pendiente, Gastos, Ganancia neta, Aforo dinámico, desglose por categoría y medio de pago).

2. **Entradas:**
   - `GET /api/eventos/{id}/entradas`: Listado de entradas con búsqueda textual (`q`) y filtro de estado.
   - `POST /api/eventos/{id}/entradas`: Venta por lote (crea $N$ filas atómicamente en una sola transacción).
   - `GET /api/entradas/{id}`: Detalle de entrada.
   - `PATCH /api/entradas/{id}`: Edición de comprador o contacto.
   - `DELETE /api/entradas/{id}`: Anulación / eliminación de entrada.
   - `POST /api/entradas/{id}/confirmar-pago`: Transición de `Pendiente` a `Pagado`, habilitando la generación del QR.
   - `POST /api/entradas/checkin`: Validación de código QR (`{ "codigo": "32hex" }`) con bloqueo ante doble ingreso o pago pendiente.
   - `GET /api/public/ticket/{scan_token}`: Endpoint público sin autenticación para mostrar la entrada al comprador.

3. **Interesados:**
   - `GET /api/eventos/{id}/interesados`: Listado de leads.
   - `POST /api/eventos/{id}/interesados`: Registro de nuevo lead.
   - `PATCH /api/interesados/{id}`: Modificación de personas, contacto o promotora.
   - `DELETE /api/interesados/{id}`: Baja de interesado.
   - `POST /api/interesados/{id}/confirmar`: Marcado como confirmado (con cálculo dinámico de aforo).

4. **Gastos:**
   - `GET /api/eventos/{id}/gastos`: Listado de egresos.
   - `POST /api/eventos/{id}/gastos`: Registro de egreso con categoría cerrada.
   - `PATCH /api/gastos/{id}`: Edición de monto o descripción.
   - `DELETE /api/gastos/{id}`: Eliminación de gasto.

---

## 4. Integración de la Asistente IA (Ressy)

Ressy asume las tareas operativas de ticketing conversacionalmente desde el panel de chat actual.

### 4.1 Herramientas Nuevas para Ressy (`AgentToolDispatcher` & `tool_schemas`)

1. `crear_evento`: `nombre`, `fecha_hora`, `lugar`, `tipos_de_entrada` (`[{ nombre, precio, capacidad }]`).
2. `listar_eventos`: Consulta eventos disponibles, marcando el más próximo como sugerencia.
3. `vender_entradas`: `evento_id` (opcional si hay evento activo en contexto), `nombre_comprador`, `contacto`, `tipo_entrada`, `cantidad`, `medio_pago`, `estado_pago`. Emite las $N$ filas y retorna los enlaces directos a los tickets para compartir por WhatsApp.
4. `confirmar_pago_entrada`: Permite confirmar el pago de una entrada por su ID o buscando por nombre/contacto del comprador.
5. `registrar_gasto`: `evento_id`, `categoria`, `monto`, `descripcion`.
6. `resumen_evento`: Retorna el balance de cobrado, pendiente, gastos, ganancia neta y aforo proyectado.
7. `registrar_interesado`: `evento_id`, `nombre`, `contacto`, `canal`, `personas`, `promotora`.
8. `checkin_entrada`: Valida el código 32-hex escaneado o dictado por Shirley.

### 4.2 Actualización de Contexto y System Prompt

El prompt de sistema de Ressy (`SHIRLEY_SYSTEM_PROMPT`) incorporará las siguientes directrices:
- Claridad contable: distinguir siempre entre dinero cobrado efectivamente y reservas pendientes.
- Facilitación de entrega: al vender entradas, entregar siempre el enlace limpio formateado para WhatsApp (`https://wa.me/...`) para que Shirley pueda enviarlo con un solo toque.
- Aclaración proactiva: si Shirley dice "Véndele 2 entradas a Juan", Ressy pregunta o confirma para qué evento y qué tipo de entrada antes de generar filas en la base de datos.

---

## 5. Arquitectura de Frontend (Vanilla JS Cohesivo)

El frontend amplía el router por hash existente y la estructura visual sin romper los estilos de escenario.

### 5.1 Navegación Principal

- **App Shell & Topbar:** Se incorpora un selector de sección principal en la barra superior o cabecera:
  - `[ 🎵 Repertorios | 🎟️ Eventos ]`
  - "Repertorios" continúa siendo la vista inicial (`#/presentations`).
  - Al pulsar "Eventos", navega a `#/eventos`.
- **Rutas del Hash Router (`frontend/js/router/parse-route.js`):**
  - `#/presentations` -> Catálogo de shows.
  - `#/presentations/:id` -> Detalle de presentación y Stage Mode.
  - `#/eventos` -> Catálogo de eventos, estado (activos / finalizados) y botón de nuevo evento.
  - `#/eventos/:id` -> Vista detallada del evento con navegación por pestañas:
    1. **Resumen:** Tarjetas de balance (Cobrado, Gastos, Ganancia neta), aforo dinámico y barra de progreso.
    2. **Entradas:** Listado con buscador, filtro por estado de pago, botón de venta rápida y botón directo de WhatsApp.
    3. **Interesados:** Lista de leads, contador de personas y acción de confirmar.
    4. **Gastos:** Desglose categorizado de gastos y formulario rápido de egreso.
  - `#/puerta/:evento_id` -> Modo Puerta (pantalla de escaneo rápido de QR o ingreso manual de código para recepción de público).
  - `#/ticket/:scan_token` -> Vista pública de la entrada (optimizada para smartphones de los asistentes, muestra QR únicamente si Pagado).

### 5.2 Enlaces de WhatsApp (`wa.me`)

Sin necesidad de servidores ni librerías pesadas, el frontend construye URLs seguras `https://wa.me/{telefono}?text={mensaje}`:
- **Mensaje de Entrada Pagada:**  
  *"¡Hola {nombre}! Aquí tienes tu entrada para {evento}: {url_publica}. ¡Nos vemos en el show! 🎟️"*
- **Mensaje de Reserva Pendiente:**  
  *"¡Hola {nombre}! Tu reserva para {evento} está lista. Para activar tu entrada QR, puedes yapear al número {telefono_yape} y confirmarnos por aquí."*

---

## 6. Riesgos, Mitigaciones y Límites

| Riesgo / Desafío | Impacto | Estrategia de Mitigación |
| :--- | :--- | :--- |
| **Doble Token (`scan_token` vs `codigo_qr`)** | Falsificación o validación no autorizada de accesos. | Separación estricta: `scan_token` solo sirve para leer la vista web. La puerta de acceso exige exclusivamente el `codigo_qr` de 32 hex que reside dentro del gráfico QR. |
| **QR en entradas pendientes** | Fuga de accesos sin pago verificado. | El backend **omite completamente** el SVG del QR si `estado_pago != 'Pagado'`. El frontend muestra un aviso amarillo de "Pago Pendiente" sin código escaneable. |
| **Venta multi-fila en SQLite** | Inconsistencia si una inserción falla a mitad de una orden de varias entradas. | Ejecutar la inserción de las $N$ filas dentro de una única transacción SQLite atómica (`tx.commit()`), garantizando todo o nada. |
| **Aforo Dinámico** | Cálculo erróneo de capacidad o sobreventa. | Fórmula matemática explícita: Aforo = Entradas Vendidas + Entradas Pendientes + $\sum \text{personas}$ de interesados confirmados. |
| **Almacenamiento de Afiches (10MB)** | Ataques de path traversal o saturación de disco. | Límite estricto de 10 MiB en Axum (`DefaultBodyLimit`). Guardado en `data/posters/{evento_id}_{uuid}.jpg` con validación de tipo MIME y nombres saneados. |
| **Colisión de Servidor en Puerto :3100** | Daño a la instancia viva de la usuaria. | Invariante operativo: NO tocar `.run/`, NO levantar procesos en `:3100`, pruebas herméticas con SQLite en memoria y carpetas temporales. |

---

## 7. No-Objetivos Explícitos (Fuera de Alcance)

1. **Autenticación y roles de usuario:** La aplicación es de uso directo de la artista y su equipo de confianza local.
2. **Bot de WhatsApp o pasarela Whatsmeow:** No se integran daemons de mensajería ni librerías de conexión con servidores de WhatsApp; solo enlaces estándar `wa.me`.
3. **Pasarela de pagos en línea (Stripe, Niubiz, MercadoPago):** Los pagos se efectúan vía Yape / Efectivo de forma presencial o directa y se concilian manualmente.

---

## 8. Plan de Slices (Cumplimiento de Presupuesto ≤ 400 líneas)

Para asegurar revisiones atómicas y seguras bajo el límite de 400 líneas:

- **Slice 1: Esquema de base de datos, modelos de dominio y repositorios núcleo (Eventos, Gastos, Resumen)** (~350 líneas)  
  Migración `0002_ticketing.sql`, entidades `Evento`, `Gasto`, `ResumenEvento`, repositorios SQLite y use cases de eventos/gastos con pruebas herméticas.
- **Slice 2: Entradas, venta multi-fila, QR SVG doble token y check-in en puerta** (~380 líneas)  
  Crate `qrcode` (svg), generación de tokens, use case `VenderEntradas` (transaccional), `ConfirmarPago`, `CheckinEntrada`, `GetPublicTicket` y endpoints correspondientes.
- **Slice 3: Interesados, almacenamiento de afiches (posters 10MB) y endpoints de administración** (~340 líneas)  
  Entidad y repositorio de interesados, almacenamiento local de afiches en `data/posters/`, endpoints multipart y cierre/reapertura de eventos.
- **Slice 4: Capacidades de Agente IA (Herramientas Ressy para Ticketing y Finanzas)** (~320 líneas)  
  Definición de schemas y dispatcher de herramientas de ticketing en `src/agent/`, prompt contextual actualizado y pruebas unitarias de turnos.
- **Slice 5: Frontend UI - Navegación Topbar, Catálogo de Eventos, Detalle y Pestañas** (~390 líneas)  
  Conmutador en topbar, rutas en hash router, vista de catálogo de eventos y vista detallada con pestañas (Resumen, Entradas, Interesados, Gastos).
- **Slice 6: Frontend UI - Venta Rápida, Enlaces wa.me, Vista Pública de Ticket y Modo Puerta** (~380 líneas)  
  Modal de venta rápida con generación de links `wa.me`, pantalla pública de entrada (`#/ticket/:scan_token`) con QR dinámico y vista de puerta (`#/puerta/:evento_id`) para validación rápida.

---

## Key Learnings

1. La generación de QR con la característica SVG pura del crate `qrcode` elimina completamente las dependencias de C y compresión de imagen, manteniendo el binario ligero y compatible con NixOS.
2. El desacoplamiento de tokens (`scan_token` para la vista web y `codigo_qr` para el escaneo en puerta) previene la validación prematura o no autorizada sin requerir capas pesadas de autenticación.
3. Condicionar la emisión del SVG del código QR en el backend según el estado de pago previene de raíz el ingreso accidental de entradas pendientes en puerta.
4. Mantener la sección de Repertorios como primera opción visual en el app shell preserva el valor original de Shirley Show como herramienta de escenario.
