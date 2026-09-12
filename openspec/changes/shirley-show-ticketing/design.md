# Diseño: ticketing y finanzas de eventos

**Change:** `shirley-show-ticketing`  
**Base:** rama `main`  
**Estado:** listo para tareas de implementación  
**Store:** OpenSpec

## 1. Resultado arquitectónico

El cambio amplía el monolito Rust/Axum y el frontend vanilla con un módulo cohesivo de eventos. API, Ressy y frontend convergen en los mismos casos de uso; SQLite y el filesystem quedan detrás de contratos explícitos. Repertorios conserva `#/presentations` como ruta inicial.

```text
Hash views / Axum handlers / Ressy dispatcher
                    │
                    ▼
          Casos de uso de ticketing
                    │
          dominio + puertos requeridos
             ┌──────┴──────┐
             ▼             ▼
      SQLite repository  Poster storage
             │             │
             ▼             ▼
  0002: cuatro tablas   data/posters/
```

La organización será por capacidad, sin crear un archivo por función. Cada acción seguirá siendo un caso de uso identificable, pero las acciones que cambian juntas se agruparán en archivos de eventos, entradas, interesados, gastos, afiches y resumen.

## 2. Decisiones y rationale

| Decisión | Rationale |
|---|---|
| Mantener ticketing dentro del monolito actual | API, Ressy y UI comparten proceso, datos y despliegue; un servicio separado añadiría coordinación sin beneficio actual. |
| Representar dinero como centavos enteros en dominio y SQLite | Evita errores de suma con `REAL` y garantiza que métricas y desgloses reconcilien exactamente. Los DTO exponen soles como número decimal. |
| Mantener tipos de entrada como JSON normalizado en `eventos` | Son configuración propia del evento y se leen juntos; no se necesita una quinta tabla. Capacidad y precio se validan en dominio y se revalidan dentro de la transacción de venta. |
| Usar dos nuevos tipos internos, `ScanToken` y `QrSecret` | Hace difícil intercambiar accidentalmente la URL pública y la credencial de puerta. Ambos se generan con llamadas independientes a `OsRng`. |
| Generar QR únicamente al construir una respuesta pagada | Una entrada pendiente conserva su secreto interno, pero no genera SVG ni expone `codigo_qr`. El QR pagado codifica solo `codigo_qr`. |
| Ejecutar ventas, conversiones y check-in con transacciones SQLite `IMMEDIATE` | Serializa decisiones de capacidad y actualizaciones condicionales, evitando venta parcial, reconversión y doble ingreso concurrente. |
| Definir un puerto transaccional de ticketing además de repositorios CRUD | La transacción atraviesa entradas e interesados; ocultarla detrás de CRUD separados rompería la atomicidad o filtraría `rusqlite` a aplicación. |
| Validar afiches por bytes reales, no por nombre ni solo MIME | El nombre del cliente y `Content-Type` no son confiables. Magic bytes determinan JPEG, PNG o WebP y, por tanto, la extensión persistida. |
| Conservar el router hash y el app shell existentes | Reduce regresiones y mantiene `#/presentations` como experiencia principal; Eventos es una sección hermana. |
| Añadir ocho tools de ticketing sin retirar las ocho tools actuales | “Exactamente ocho” se aplica al subconjunto operativo de ticketing. `get_tool_definitions()` tendrá 16 tools totales y una prueba fijará los ocho nombres nuevos. |
| Usar `qrcode` con salida SVG y sin backend de imagen | Evita PNG, bindings C y dependencias de compresión; el resultado se transporta como texto SVG generado por el servidor. |

## 3. Estructura de módulos

Se preserva la estructura existente y se añaden fronteras de ticketing sin reorganizar repertorios.

```text
migrations/
└── 0002_ticketing.sql

src/
├── domain/ticketing/
│   ├── mod.rs                  # superficie pública y enums compartidos
│   ├── event.rs                # Evento, TipoEntrada y reglas de capacidad
│   ├── ticket.rs               # Entrada, Money, pagos, tokens y check-in
│   ├── interest.rs             # Interesado y transición de conversión
│   ├── expense.rs              # Gasto y catálogo cerrado
│   └── summary.rs              # ResumenEvento y desgloses
├── ports/ticketing.rs          # contratos agrupados por esta capacidad
├── use_cases/ticketing/
│   ├── mod.rs
│   ├── events.rs               # create/list/get/update/finalize/reopen/delete
│   ├── tickets.rs              # sell/list/get/update/delete/confirm/public
│   ├── checkin.rs              # check-in atómico y sus resultados
│   ├── interests.rs            # CRUD y convertir a venta
│   ├── expenses.rs             # CRUD
│   ├── posters.rs              # upload/get y coordinación de reemplazo
│   └── summary.rs              # consulta consolidada
├── infra/sqlite/ticketing_repository/
│   ├── mod.rs                  # SqliteTicketingRepository y mapeos comunes
│   ├── events.rs
│   ├── tickets.rs              # venta/check-in transaccionales
│   ├── interests.rs            # conversión comparte emisión interna
│   ├── expenses.rs
│   └── summary.rs
├── infra/filesystem/local_poster_storage.rs
├── infra/random_credentials.rs
└── handlers/ticketing/
    ├── mod.rs
    ├── dto.rs                  # contratos HTTP y conversión explícita
    ├── events.rs
    ├── tickets.rs
    ├── interests.rs
    └── expenses.rs

frontend/js/modules/events/
├── api.js                      # cliente HTTP cohesivo de la funcionalidad
├── controllers.js              # catálogo y detalle administrativo
├── public-ticket-controller.js
├── door-controller.js
├── views.js                    # vistas presentacionales de catálogo/detalle
├── forms.js                    # evento, venta rápida, interesado y gasto
├── whatsapp.js                 # limpieza de teléfono y construcción wa.me
└── camera-qr-scanner.js        # getUserMedia/BarcodeDetector y cleanup
```

Si un archivo excede una responsabilidad durante implementación, podrá dividirse por vista o agregado; no se crearán wrappers que solo reenvíen argumentos. Los estilos usarán prefijo `.events-*` y se agruparán en `frontend/styles/events/`, importados desde `styles/index.css`.

`AppState` incorporará un `TicketingServices` cohesivo, compuesto una vez a partir de un `Arc<SqliteTicketingRepository>`, `Arc<LocalPosterStorage>` y `Arc<OsRandomCredentials>`. Handlers y `AgentToolDispatcher` recibirán esos mismos casos de uso.

## 4. Dominio y contratos

### 4.1 Entidades y value objects

- `Money { cents: i64 }`: acepta importes finitos, no negativos y con máximo dos decimales; suma y resta se realizan en enteros. `ganancia_neta` puede ser negativa, por lo que la resta produce un valor firmado aunque entradas y gastos individuales no puedan ser negativos.
- `TicketType { nombre, precio: Money, capacidad: u32 }`: nombre recortado, capacidad mínima 1. Los nombres son únicos sin distinguir mayúsculas/minúsculas dentro de un evento.
- `Event`: UUID, nombre, fecha/hora RFC3339 normalizada, lugar, tipos, `finalizado`, referencia opcional de afiche y timestamps.
- `PaymentMethod`: `Yape | Efectivo | Cortesia`, serializado como `Yape`, `Efectivo`, `Cortesía`.
- `PaymentStatus`: `Pagado | Pendiente`.
- `CheckinStatus`: `NoIngreso | Ingreso`, serializado como `No ingresó`, `Ingresó`.
- `Ticket`: UUID, evento, comprador/contacto, snapshot del nombre y precio del tipo, medio/estado de pago, `ScanToken`, `QrSecret`, check-in, origen opcional `interesado_id` y timestamps.
- `InterestStatus`: `Pendiente | Confirmado | Cancelado`; `converted_at` distingue una confirmación manual de una conversión ya ejecutada.
- `ExpenseCategory`: enum cerrado con las siete categorías de la spec.
- `EventSummary`: siete métricas conceptuales. Como “Capacidad y Entradas Emitidas” contiene dos valores, el DTO posee ocho escalares: `cobrado`, `pendiente`, `total_proyectado`, `gastos`, `ganancia_neta`, `capacidad_total`, `entradas_emitidas`, `aforo_proyectado`.
- `ScanToken` y `QrSecret`: newtypes no serializados directamente en respuestas públicas. El primero solo forma URLs; el segundo solo alimenta QR y check-in.

Las fechas de entrada usan `chrono::DateTime::parse_from_rfc3339`; gastos usan `NaiveDate`. DTOs convierten importes en soles a `Money` y rechazan NaN, infinito, negativos o más de dos decimales con HTTP 400.

### 4.2 Puertos

`src/ports/ticketing.rs` agrupa contratos pequeños en un mismo archivo:

- `EventRepository`: crear, listar, obtener, actualizar metadatos/tipos, cambiar estado y eliminar.
- `TicketRepository`: listar/buscar de forma acotada, obtener, editar comprador/contacto, confirmar pago y eliminar si no ingresó.
- `InterestRepository`: CRUD y cambios de estado no transaccionales.
- `ExpenseRepository`: CRUD.
- `EventSummaryReader`: lectura agregada de métricas y desgloses.
- `TicketingTransactions`: `issue_sale`, `convert_interest` y `check_in`; sus implementaciones son responsables del único límite transaccional, pero ejecutan las políticas del dominio.
- `PosterStorage`: escribir un archivo validado, abrirlo, reemplazarlo y retirar/quarantinar un asset por clave segura.
- `CredentialGenerator`: producir pares independientes `{scan_token, qr_secret}`; permite un fake determinista en pruebas.

Los puertos no exponen `rusqlite::Connection`, paths arbitrarios ni DTOs Axum.

### 4.3 Casos de uso por acción

| Grupo | Acciones públicas |
|---|---|
| Eventos | `CreateEvent`, `ListEvents`, `GetEvent`, `UpdateEvent`, `FinalizeEvent`, `ReopenEvent`, `DeleteEvent` |
| Entradas | `SellTickets`, `ListTickets`, `GetTicket`, `UpdateTicketContact`, `DeleteTicket`, `ConfirmTicketPayment`, `GetPublicTicket` |
| Puerta | `CheckInTicket` |
| Interesados | `CreateInterest`, `ListInterests`, `UpdateInterest`, `DeleteInterest`, `ConvertInterestToSale` |
| Gastos | `CreateExpense`, `ListExpenses`, `UpdateExpense`, `DeleteExpense` |
| Resumen | `GetEventSummary` |
| Afiches | `ReplaceEventPoster`, `GetEventPoster` |

`SellTickets` genera exactamente `N` pares de credenciales y llama una vez a `TicketingTransactions::issue_sale`. La implementación abre `BEGIN IMMEDIATE`, vuelve a cargar evento/tipo, comprueba que esté activo, cuenta entradas emitidas del tipo, aplica precio del evento (o cero/Pagado para cortesía), inserta `N` filas y confirma. Cualquier error revierte todo.

`ConvertInterestToSale` exige una cantidad explícita mayor que cero, precargada por la UI con `personas`. Dentro de un solo `BEGIN IMMEDIATE` vuelve a cargar el interesado, exige `converted_at IS NULL`, deriva nombre/contacto/evento, invoca la misma función interna de emisión usada por la venta normal y finalmente actualiza `estado = 'Confirmado'` y `converted_at`. Un fallo deja cero entradas y el interesado intacto. Reintentar después del commit devuelve `409 interest_already_converted`.

La actualización de tipos de entrada permite cambiar precio/capacidad. Una vez que existen entradas de un tipo, su nombre no puede eliminarse ni renombrarse y su capacidad no puede quedar por debajo de las entradas ya emitidas. El precio nuevo solo afecta ventas futuras; cada entrada conserva el precio de emisión.

## 5. Migración SQLite `0002_ticketing.sql`

`migrate.rs` incluirá `MIGRATION_0002` y aplicará una lista ordenada de `(version, sql)`; cada versión se ejecuta y registra en su propia transacción. La migración es aditiva, crea exactamente cuatro tablas de negocio y sus índices, y no modifica `0001`.

### 5.1 `eventos`

| Columna | Tipo/regla |
|---|---|
| `id` | `TEXT PRIMARY KEY` |
| `nombre` | `TEXT NOT NULL`, trim no vacío |
| `fecha_hora` | `TEXT NOT NULL`, validado como RFC3339 por dominio |
| `lugar` | `TEXT NOT NULL`, trim no vacío |
| `tipos_de_entrada` | `TEXT NOT NULL CHECK(json_valid(...))`; JSON normalizado con `precio_centimos` |
| `finalizado` | `INTEGER NOT NULL DEFAULT 0 CHECK IN (0,1)` |
| `imagen` | `TEXT NULL`; solo basename generado por servidor |
| `fecha_creacion`, `fecha_actualizacion` | `TEXT NOT NULL` |

Índices: `idx_eventos_estado_fecha(finalizado, fecha_hora)` y `idx_eventos_fecha(fecha_hora)`.

### 5.2 `interesados`

Se crea antes de `entradas` para que esta pueda referenciar su origen.

| Columna | Tipo/regla |
|---|---|
| `id` | `TEXT PRIMARY KEY` |
| `evento_id` | FK a `eventos(id) ON DELETE CASCADE` |
| `nombre`, `contacto`, `canal`, `promotora` | textos no vacíos |
| `personas` | `INTEGER NOT NULL CHECK(personas >= 1)` |
| `estado` | `Pendiente | Confirmado | Cancelado` |
| `convertido_at` | `TEXT NULL`; si existe, estado debe ser `Confirmado` |
| `fecha_creacion`, `fecha_actualizacion` | `TEXT NOT NULL` |

Índices: `idx_interesados_evento_estado(evento_id, estado)` y `idx_interesados_evento_creacion(evento_id, fecha_creacion)`.

### 5.3 `entradas`

| Columna | Tipo/regla |
|---|---|
| `id` | `TEXT PRIMARY KEY` |
| `evento_id` | FK a `eventos(id) ON DELETE CASCADE` |
| `interesado_id` | FK nullable a `interesados(id) ON DELETE SET NULL` |
| `nombre_comprador`, `contacto`, `tipo_entrada` | textos no vacíos |
| `precio_centimos` | `INTEGER NOT NULL CHECK >= 0` |
| `medio_pago` | `Yape | Efectivo | Cortesía` |
| `estado_pago` | `Pagado | Pendiente` |
| `codigo_qr` | secreto lowercase de 32 hex, no vacío |
| `scan_token` | `st_` + 64 hex aleatorios |
| `estado_checkin` | `No ingresó | Ingresó` |
| `hora_ingreso` | null hasta ingreso; check de consistencia con estado |
| `fecha_creacion`, `fecha_actualizacion` | `TEXT NOT NULL` |

Un `CHECK` adicional exige que Cortesía tenga precio cero y estado Pagado. Índices: únicos `ux_entradas_codigo_qr(codigo_qr)` y `ux_entradas_scan_token(scan_token)`; no únicos `idx_entradas_evento_creacion(evento_id, fecha_creacion)`, `idx_entradas_evento_pago(evento_id, estado_pago)` e `idx_entradas_evento_tipo(evento_id, tipo_entrada)`.

La búsqueda por `q` usa `LIKE ... COLLATE NOCASE` sobre comprador/contacto y siempre está limitada; no se añade un índice que prometa optimizar búsquedas con `%substring%`.

### 5.4 `gastos`

| Columna | Tipo/regla |
|---|---|
| `id` | `TEXT PRIMARY KEY` |
| `evento_id` | FK a `eventos(id) ON DELETE CASCADE` |
| `categoria` | check con las siete categorías exactas |
| `monto_centimos` | `INTEGER NOT NULL CHECK >= 0` |
| `descripcion` | texto trim no vacío |
| `fecha` | `TEXT NOT NULL`, validada como fecha por dominio |
| `fecha_creacion`, `fecha_actualizacion` | `TEXT NOT NULL` |

Índices: `idx_gastos_evento_fecha(evento_id, fecha)` y `idx_gastos_evento_categoria(evento_id, categoria)`.

### 5.5 Compatibilidad y rollback

- `PRAGMA foreign_keys=ON`, WAL y `busy_timeout=5000` ya se aplican a cada conexión y se conservan.
- La migración 0002 se registra solo después de crear tablas e índices con éxito.
- El rollback de binario deja las cuatro tablas y afiches intactos; no existe down migration automática ni `DROP TABLE`.
- SQLite previo sigue leyendo repertorios porque 0001 no cambia.

## 6. Tokens, QR y check-in

### 6.1 Credenciales

Se añaden:

```toml
qrcode = { version = "0.14", default-features = false, features = ["svg"] }
rand = "0.8"
```

`OsRandomCredentials` usa `rand::rngs::OsRng`:

- `scan_token`: 32 bytes aleatorios, codificados localmente como 64 hex y prefijados `st_`.
- `codigo_qr`: otros 16 bytes aleatorios, codificados como 32 hex.
- `id`: UUID v4 independiente.

No se deriva un valor de otro. Una colisión de índice único revierte la venta y permite hasta tres regeneraciones del lote completo; después devuelve error interno sin filtrar el token conflictivo.

`GetPublicTicket` llama al renderer SVG solo si el estado es Pagado. `qrcode::QrCode` renderiza con quiet zone y dimensiones mínimas aptas para móvil. El secreto no aparece como campo JSON, logs, errores ni HTML textual.

### 6.2 Check-in condicional

`POST /api/entradas/checkin` recibe `evento_id` y `codigo`. El caso de uso recorta y normaliza a lowercase únicamente un valor de 32 hex; URLs, `st_*` y otros formatos producen `Invalida`.

En una transacción `IMMEDIATE` se ejecuta primero:

```sql
UPDATE entradas
SET estado_checkin = 'Ingresó', hora_ingreso = ?1, fecha_actualizacion = ?1
WHERE evento_id = ?2
  AND codigo_qr = ?3
  AND estado_pago = 'Pagado'
  AND estado_checkin = 'No ingresó';
```

- Una fila afectada: `Valida`; se lee comprador, tipo y hora registrada.
- Cero filas: se relee por `codigo_qr` en la misma conexión y se clasifica como `Ya Usada` o `Invalida` por desconocido, otro evento o pago pendiente.
- Dos solicitudes concurrentes no pueden afectar la misma fila: una obtiene `Valida`; la siguiente observa el estado confirmado y obtiene `Ya Usada` con la hora original.

Los tres resultados de negocio responden HTTP 200. Solo payload malformado usa 400 y fallos de infraestructura usan 500.

## 7. Afiches

`LocalPosterStorage` es propietario exclusivo de `data/posters/` y su subdirectorio interno `.trash/`. `main.rs` crea ambos desde `Config.data_dir`; las pruebas usan `TempDir`.

Flujo de reemplazo:

1. Axum aplica un límite de cuerpo ligeramente superior a 10 MiB para permitir overhead multipart; el handler cuenta chunks del campo `imagen` y rechaza el archivo al superar exactamente `10 * 1024 * 1024` bytes.
2. Se escribe a un `.part` generado por servidor, nunca al nombre recibido.
3. Se inspeccionan magic bytes: JPEG `FF D8 FF`, PNG firma de 8 bytes, o WebP `RIFF....WEBP`. El tipo declarado debe ser permitido y compatible; otro contenido devuelve 415.
4. El archivo se promueve por rename a `{evento_id}_{uuid}.{jpg|png|webp}` y se actualiza `eventos.imagen`.
5. Si falla SQLite, se elimina el nuevo asset. Tras commit, el anterior se mueve a `.trash/` y se elimina.

El filesystem y SQLite no ofrecen una transacción distribuida. Para hacer robusta esa brecha, reemplazo/eliminación usan renames atómicos dentro del mismo filesystem y una reconciliación de arranque elimina `.part`, `.trash` y basenames no referenciados (`reconcile_orphans`). La DB nunca guarda paths aportados por cliente. El borrado de evento ejecuta primero la eliminación del afiche en el filesystem (si falla FS, aborta inmediatamente con 500 y la DB queda intacta); una vez asegurado el retiro del afiche, ejecuta la cascada en SQLite. Cualquier archivo huérfano eventual queda saneado por la reconciliación existente.

`GET /api/eventos/{id}/imagen` resuelve solo la clave validada, detecta el tipo persistido, devuelve bytes con `Content-Type`, `ETag` y `Cache-Control: public, max-age=300, must-revalidate`. Ticket público usa `Cache-Control: no-store` para no conservar estados de pago o check-in obsoletos.

## 8. API Axum y DTOs

### 8.1 Rutas

| Método y ruta | Caso de uso | Éxito |
|---|---|---|
| `GET /api/eventos` | `ListEvents` (`incluir_finalizados`) | 200 |
| `POST /api/eventos` | `CreateEvent` | 201 |
| `GET /api/eventos/{id}` | `GetEvent` | 200 |
| `PATCH /api/eventos/{id}` | `UpdateEvent` | 200 |
| `DELETE /api/eventos/{id}` | `DeleteEvent` | 200 con `cleanup_pending` |
| `POST /api/eventos/{id}/finalizar` | `FinalizeEvent` idempotente | 200 |
| `POST /api/eventos/{id}/reabrir` | `ReopenEvent` idempotente | 200 |
| `POST /api/eventos/{id}/imagen` | `ReplaceEventPoster` | 200 |
| `GET /api/eventos/{id}/imagen` | `GetEventPoster` | 200 bytes |
| `GET /api/eventos/{id}/resumen` | `GetEventSummary` | 200 |
| `GET /api/eventos/{id}/entradas` | `ListTickets` | 200 |
| `POST /api/eventos/{id}/entradas` | `SellTickets` | 201 |
| `GET /api/entradas/{id}` | `GetTicket` | 200 |
| `PATCH /api/entradas/{id}` | `UpdateTicketContact` | 200 |
| `DELETE /api/entradas/{id}` | `DeleteTicket` | 204 |
| `POST /api/entradas/{id}/confirmar-pago` | `ConfirmTicketPayment` idempotente | 200 |
| `POST /api/entradas/checkin` | `CheckInTicket` | 200 con resultado |
| `GET /api/public/ticket/{scan_token}` | `GetPublicTicket` | 200/no-store |
| `GET /api/eventos/{id}/interesados` | `ListInterests` | 200 |
| `POST /api/eventos/{id}/interesados` | `CreateInterest` | 201 |
| `PATCH /api/interesados/{id}` | `UpdateInterest` | 200 |
| `DELETE /api/interesados/{id}` | `DeleteInterest` | 204 |
| `POST /api/interesados/{id}/vender` | `ConvertInterestToSale` | 201 |
| `GET /api/eventos/{id}/gastos` | `ListExpenses` | 200 |
| `POST /api/eventos/{id}/gastos` | `CreateExpense` | 201 |
| `PATCH /api/gastos/{id}` | `UpdateExpense` | 200 |
| `DELETE /api/gastos/{id}` | `DeleteExpense` | 204 |

El router agrupa estas rutas en `handlers::ticketing::router()` y aplica el límite multipart solo a upload de afiche, no al router completo.

### 8.2 DTOs clave

`CreateEventRequest`/`UpdateEventRequest` usan `tipos_de_entrada: [{nombre, precio, capacidad}]`. `SellTicketsRequest` contiene comprador, contacto, tipo, cantidad, medio y estado; no acepta precio libre: el caso de uso toma el precio configurado. `ConvertInterestRequest` contiene tipo, cantidad, medio y estado; nombre/contacto/evento proceden del interesado.

La respuesta administrativa de entrada contiene `id`, `evento_id`, comprador, contacto, tipo, precio, pago, check-in, `scan_token` y `ticket_path`. No contiene `codigo_qr`. Listados no incluyen SVG; detalle/venta pueden incluir `qr_svg` solo para pagadas si el consumidor lo solicita.

Respuesta pública exacta:

```json
{
  "nombre_evento": "Cena Show",
  "fecha_hora": "2026-10-15T21:00:00Z",
  "lugar": "Barranco",
  "nombre_comprador": "Valeria",
  "tipo_entrada": "General",
  "precio": 30.0,
  "estado_pago": "Pagado",
  "estado_checkin": "No ingresó",
  "qr_svg": "<svg ...>...</svg>"
}
```

Para pendiente, `qr_svg` es `null`; no se incluyen `id`, `evento_id`, `scan_token`, `codigo_qr`, contacto ni agregados financieros.

Resumen:

```json
{
  "cobrado": 300.0,
  "pendiente": 120.0,
  "total_proyectado": 420.0,
  "gastos": 180.0,
  "ganancia_neta": 120.0,
  "capacidad_total": 50,
  "entradas_emitidas": 16,
  "aforo_proyectado": 19,
  "por_medio_pago": {
    "Yape": { "cobrado": 300.0, "pendiente": 0.0, "cantidad": 10 },
    "Efectivo": { "cobrado": 0.0, "pendiente": 120.0, "cantidad": 4 },
    "Cortesía": { "cobrado": 0.0, "pendiente": 0.0, "cantidad": 2 }
  },
  "gastos_por_categoria": {
    "Local/Alquiler": 0.0,
    "Sonido/Luces": 180.0,
    "Músicos/Personal": 0.0,
    "Publicidad/Marketing": 0.0,
    "Bebidas/Catering": 0.0,
    "Transporte": 0.0,
    "Otros": 0.0
  }
}
```

Todos los totales se calculan en centavos mediante consultas `SUM(CASE ...)`; se inicializan con `COALESCE(..., 0)` y los desgloses siempre contienen sus claves completas.

### 8.3 Errores

Se conserva `{ "error": { "code", "message", "field" } }`. Sin cambiar el 422 histórico de repertorios, los casos nuevos usan:

- 400: JSON inválido, fecha/importe/enums/cantidad inválidos, evento finalizado, tipo inexistente o capacidad insuficiente (`available`, `requested` se expresan en el mensaje).
- 404: evento, entrada, interesado, gasto, afiche o `scan_token` inexistente.
- 409: entrada ya ingresada al eliminar, interesado ya convertido, o edición de tipos incompatible con ventas históricas.
- 413: archivo mayor de 10 MiB.
- 415: bytes no JPEG/PNG/WebP o desacuerdo de tipo.
- 500: SQLite/filesystem/QR inesperado, sin detalles internos.

Los resultados `Invalida` y `Ya Usada` de puerta son respuestas 200, no errores HTTP.

## 9. Ressy

### 9.1 Ocho tools nuevas

Se agregan exactamente estos nombres a las ocho tools actuales:

1. `crear_evento`
2. `listar_eventos`
3. `vender_entradas`
4. `confirmar_pago_entrada`
5. `registrar_gasto`
6. `resumen_evento`
7. `registrar_interesado`
8. `checkin_entrada`

Los JSON Schema usan `additionalProperties: false`, enums y mínimos de la spec. `vender_entradas` no recibe precio; el precio real viene del tipo configurado. `registrar_gasto` usa la fecha actual si la tool no la recibe. Las tools invocan los mismos objetos de casos de uso de `TicketingServices`; no importan repositorios SQLite.

`TurnToolContext` añade `known_event_ids`, `known_ticket_ids` y `active_event_id`. `listar_eventos` es lectura cacheable dentro del turno y llena IDs conocidos. `crear_evento` agrega el ID creado. Una venta, gasto, resumen, interesado o check-in rechaza un `evento_id` que no provenga del contexto activo o de `listar_eventos`. IDs de entradas emitidas se registran para permitir confirmación posterior sin inventarlos.

El contrato de chat acepta opcionalmente `event_id`; los controladores de `#/eventos/:id` lo fijan y las vistas de repertorio siguen enviando `presentation_id`. `AgentChatOutput` y `ToolAction` añaden campos opcionales `active_event_id`/`event_id` sin retirar los actuales.

Las tools mutantes conservan deduplicación por `call.id` y firma canónica. Además, las garantías de DB hacen idempotentes confirmación y resistentes a carrera check-in/conversión; una venta repetida en otro turno no se deduplica automáticamente y Ressy debe pedir confirmación ante ambigüedad.

Para enlaces completos de ticket en respuestas de Ressy se agrega `SHIRLEY_PUBLIC_BASE_URL` opcional, validado como origen HTTP(S). Cuando está configurado, la tool devuelve `{origin}/#/ticket/{scan_token}`; en desarrollo se devuelve el path relativo. `wa.me` sigue construyéndose en el navegador; no se añade daemon ni envío servidor-a-servidor.

### 9.2 Prompt

`SHIRLEY_SYSTEM_PROMPT` añade una sección corta “Eventos y entradas”:

- Nunca inventar IDs, tipos, precios o capacidades; listar/preguntar si falta evento.
- Antes de vender, exigir evento, comprador, contacto, tipo, cantidad, medio y estado de pago.
- Cortesía siempre equivale a precio cero y Pagado.
- Diferenciar Cobrado, Pendiente, Total proyectado y Ganancia neta; nunca llamar ingreso realizado a lo pendiente.
- Informar cada enlace de ticket devuelto tras una venta.
- Confirmar pago no equivale a check-in.
- Check-in solo usa el código de 32 hex, nunca una URL o `scan_token`.
- No afirmar éxito hasta recibir respuesta exitosa de la tool.

## 10. Frontend

### 10.1 Navegación y composición

`parse-route.js` reconocerá, en este orden específico:

- vacío, `/` y `#/presentations` → `presentations`.
- `#/presentations/:id` → `presentation-detail`.
- `#/eventos` → `events`.
- `#/eventos/:id` → `event-detail`.
- `#/ticket/:token` → `public-ticket`.
- `#/puerta/:id` → `door`.

Cada segmento se decodifica con manejo seguro de URI inválida; una ruta desconocida vuelve a `#/presentations`. `navigate.js` añade `toEvents`, `toEvent`, `toPublicTicket` y `toDoor`.

`createAppTopbar` incorpora un selector accesible `Repertorios | Eventos` mediante props `activeSection` y `onSectionChange`. Los controladores administrativos lo reutilizan; ticket público usa una cabecera mínima y puerta una cabecera compacta. El selector no se duplica en cada vista.

`main.js` continúa siendo el composition root: destruye el controlador anterior, monta header/main y conserva singleton de audio y Ressy. Al entrar a Eventos detiene audio y fija contexto de evento; al salir o entrar a ticket público limpia ese contexto.

### 10.2 Vistas administrativas

- `#/eventos`: lista mobile-first separada en activos/finalizados, estados de carga/vacío/error, crear/editar/eliminar y afiche opcional.
- `#/eventos/:id`: header de evento, finalizar/reabrir, afiche, acceso a puerta y pestañas locales `Resumen`, `Entradas`, `Interesados`, `Gastos`.
- Resumen muestra las siete métricas conceptuales, desglose Yape/Efectivo/Cortesía, gastos por categoría y alerta cuando `aforo_proyectado > capacidad_total`.
- Entradas tiene búsqueda, filtros de pago/tipo, confirmación, eliminación protegida, botón de WhatsApp y “Venta rápida”.
- Venta rápida solicita todos los datos, preselecciona solo cuando existe una única opción válida y muestra una tarjeta/enlace por entrada emitida.
- Interesados permite CRUD, estado y pase a venta; el formulario de conversión precarga cantidad con `personas` y reutiliza el endpoint transaccional.
- Gastos permite CRUD rápido con catálogo cerrado y recarga el resumen tras cada mutación.

Controladores poseen carga/errores y llaman `api.js`; vistas reciben datos/callbacks. Dialogs y scanner liberan listeners, stream de cámara y timers en `destroy()`.

### 10.3 Ticket, WhatsApp y puerta

`whatsapp.js` es una transformación pura: elimina no dígitos, antepone `51` a teléfonos peruanos de nueve dígitos y construye `https://wa.me/{telefono}?text=${encodeURIComponent(mensaje)}`. El mensaje pagado incluye nombre, evento y URL pública absoluta desde `window.location.origin`; el pendiente incluye monto, URL y la instrucción de coordinar/confirmar Yape o Efectivo antes de habilitar el QR.

`#/ticket/:token` consume exclusivamente el DTO público. Para Pagado muestra evento, datos de entrada y SVG prominente; para Pendiente muestra banner ámbar e instrucciones sin crear un contenedor QR. El SVG se inserta solo desde `qr_svg` generado por el backend; ningún valor del comprador se interpola como HTML.

`#/puerta/:id` prioriza móvil: campo manual siempre disponible, foco recuperado después de cada resultado, botones de al menos 44 px y pantalla completa con estados inequívocos verde `Válida`, ámbar `Ya usada`, rojo `Inválida`. Si `BarcodeDetector` soporta QR, `camera-qr-scanner.js` solicita cámara tras gesto explícito, entrega el texto al mismo submit y detiene tracks al pausar/salir. Sin soporte o permiso, la entrada manual sigue funcionando. El resultado exitoso muestra comprador y tipo; “ya usada” conserva la hora original; inválida muestra el motivo seguro.

## 11. Flujo de datos crítico

### Venta

```text
UI/API/Ressy → SellTickets DTO
→ validación de dominio + N credenciales independientes
→ BEGIN IMMEDIATE
→ evento activo + tipo/precio + count(tipo)
→ validar capacidad
→ INSERT N entradas
→ COMMIT
→ DTOs sin codigo_qr + paths públicos
```

### Conversión

```text
ConvertInterestToSale
→ BEGIN IMMEDIATE
→ interesado actual y convertido_at IS NULL
→ emisión compartida de N entradas
→ UPDATE interesado Confirmado + convertido_at
→ COMMIT
```

### Ticket público

```text
scan_token → entrada + evento
→ Pendiente: DTO qr_svg=null
→ Pagado: render qrcode(codigo_qr) → SVG → DTO mínimo/no-store
```

### Resumen

```text
evento + JSON capacidades
├─ agregados entradas por pago/medio/tipo
├─ agregados gastos por categoría
└─ personas de interesados Confirmados
→ EventSummary en centavos y conteos
→ DTO en soles
```

## 12. Estrategia de pruebas

Se sigue TDD estricto. Cada slice empieza con la prueba de aceptación más estrecha y termina con `cargo test`; los tests Node del frontend se invocan desde wrappers Rust, como ya hace `agent_chat_ui_tests.rs`.

### Automatizadas

- **Migración:** versión 2 aplicada una vez, cuatro tablas/índices/checks/FKs, idempotencia y compatibilidad con 0001.
- **Dominio:** trims, RFC3339/fecha, nombres únicos de tipos, centavos, enums, cortesía, categorías, cantidades y edición segura de capacidad.
- **Repositorios SQLite con `TempDir`:** CRUD y cascada; venta N exacta; rollback por capacidad o colisión; precio snapshot; finalizado; búsqueda/filtros acotados; confirmación idempotente; eliminación 409; conversión exitosa/fallida/duplicada.
- **Concurrencia:** dos threads/conexiones contra el mismo archivo SQLite ejecutan check-in simultáneo y producen exactamente una `Valida` y una `Ya Usada`; prueba análoga de capacidad evita sobreventa concurrente.
- **QR/tokens:** formato y unicidad, dos fuentes independientes mediante fake, pending sin invocar renderer, paid con SVG y secreto ausente como clave/campo de respuesta.
- **Resumen:** escenario completo de spec, vacío, cortesías, reconciliación de medios, siete categorías a cero y ganancia negativa posible.
- **Afiches:** 10 MiB aceptado, >10 MiB 413, MIME/magic inválido 415, UUID basename, reemplazo, eliminación, reconciliación de `.part/.trash`, path traversal imposible.
- **API Axum `oneshot`:** status/DTO/error de cada familia; public 404/no-store/superficie mínima; multipart; tres resultados de check-in.
- **Ressy:** 16 tools totales y conjunto exacto de ocho de ticketing; schemas; unknown tool; IDs no conocidos; venta/summary/check-in por los casos de uso; deduplicación; prompt contable y aclaración.
- **Frontend Node:** seis rutas y fallback, limpieza/codificación `wa.me`, render pendiente sin QR, colores/texto de puerta y cleanup del scanner con fakes.
- **Regresión:** suite existente de repertorios, audio y agente permanece verde.

Comandos de implementación/verificación:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

No se iniciarán procesos en `:3100` ni se tocará `.run/`.

### E2E manual

1. Desde `#/presentations`, abrir Eventos y volver sin perder repertorios.
2. Crear evento con dos tipos, subir/reemplazar JPEG/PNG/WebP y verificar móvil/escritorio.
3. Venta rápida de varias entradas pagadas: filas únicas, links `wa.me` y ticket público con QR.
4. Reserva pendiente: ticket y WhatsApp muestran pago pendiente sin QR; confirmar pago y recargar para verlo.
5. Escanear en dos dispositivos casi simultáneamente: uno verde y otro ámbar con la misma hora inicial.
6. Probar código de otro evento, `scan_token`, código desconocido y pago pendiente: todos rojos con motivo correcto.
7. Crear/confirmar/cancelar/convertir interesado y comprobar aforo; reintentar conversión sin duplicar entradas.
8. Registrar/editar/eliminar gastos y reconciliar las métricas/desgloses.
9. Finalizar evento, verificar venta/conversión bloqueadas, reabrir y volver a vender.
10. Probar puerta en teléfono con cámara permitida, denegada y sin `BarcodeDetector`; verificar cleanup al salir.

## 13. Entrega y rollout

Se mantienen los seis slices de la propuesta:

1. Migración, dominio, eventos/gastos/resumen.
2. Entradas, doble token, QR, pago y puerta.
3. Interesados, conversión, afiches y lifecycle restante.
4. Ocho tools Ressy, contexto y prompt.
5. Topbar, router, catálogo/detalle administrativo.
6. Venta rápida, WhatsApp, ticket público y puerta móvil.

Antes de implementar cada slice se estiman líneas reales. Con `ask-on-risk`, si el forecast supera 400 líneas o mezcla superficies difíciles de revisar, se pausa para elegir encadenamiento; no se infiere `size:exception`. Una migración aplicada no se revierte destructivamente. Si QR/check-in falla en rollout, se deshabilita la ruta de puerta antes que aceptar `scan_token` o relajar pago.

## 14. Trazabilidad de las ocho specs

| Spec | Superficie principal |
|---|---|
| `eventos` | dominio Event, migración, lifecycle, capacidad y afiches |
| `entradas` | venta transaccional, pagos, búsqueda, borrado y tokens |
| `interesados` | CRUD, aforo proyectado y conversión atómica |
| `gastos` | enum cerrado, CRUD y centavos |
| `resumen` | query model con siete métricas y dos desgloses |
| `checkin` | update condicional y tres resultados |
| `entrada-publica` | DTO mínimo, QR condicional, ticket móvil y `wa.me` |
| `ressy-ticketing` | ocho schemas, dispatcher compartido, contexto y prompt |

## 15. Límites conocidos

- No hay autenticación ni autorización; la advertencia operativa existente sigue vigente y los endpoints administrativos no deben exponerse directamente a Internet.
- No hay transacción distribuida SQLite/filesystem; renames, compensación y reconciliación hacen recuperable la limpieza de afiches.
- `BarcodeDetector` no está disponible en todos los navegadores; la entrada manual es parte obligatoria del modo puerta.
- No se incorporan pasarela de pago, Whatsmeow, asientos, reembolsos ni rediseño visual.
