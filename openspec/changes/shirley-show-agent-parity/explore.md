# SDD Exploration: `shirley-show-agent-parity`

**Change**: `shirley-show-agent-parity`  
**Date**: 2026-09-11  
**Status**: Completed  
**Author**: Gentle AI (SDD Explore Phase)  
**Artifact Store**: `openspec` (`openspec/changes/shirley-show-agent-parity/explore.md`)  
**Delivery Strategy**: `ask-on-risk` (Budget: 400 líneas / slice)  

---

## 1. Visión y Propósito del Cambio

El asistente conversacional de Shirley Show (**Ressy**) cuenta actualmente con 16 herramientas registradas en `src/agent/tool_schemas.rs`: 8 orientadas a presentaciones musicales (creación, lectura, adición de letras, notas, audio y separación de voces) y 8 orientadas a ticketing básico (creación de evento, listado, venta, confirmación de pago, registro de gasto, resumen contable, registro de interesados y check-in en puerta).

Sin embargo, existe una brecha operativa entre lo que el usuario puede hacer manualmente desde la interfaz web y lo que puede solicitarle a Ressy por chat. El objetivo fundamental de este cambio es alcanzar la **paridad funcional total**:
> **Cualquier acción administrativa o de gestión que un usuario pueda ejecutar manualmente mediante los botones y formularios de la aplicación debe poder realizarse conversacionalmente a través de Ressy.**

### Acciones Excluidas con Justificación Arquitectónica (No-Objetivos)
1. **Play / Pausa de audio:** Es estado efímero y local del reproductor de medios en el navegador (`HTMLAudioElement`). No tiene persistencia ni sentido como comando asíncrono de chat.
2. **Modo Escenario (Stage Mode):** Es una alternancia visual de interfaz para pantalla completa del carrusel de canciones (`stage-mode-state.js`). No altera datos del backend.
3. **Cámara de Puerta para Check-in:** Es un flujo de hardware local del dispositivo (captura de video continuo con WebRTC/`getUserMedia` para escaneo de QR en tiempo real). El check-in conversacional ya está cubierto mediante `checkin_entrada` para ingresar o pegar el código secreto.

---

## 2. Auditoría del Gap: Acciones Manuales vs Herramientas Existentes

| Dominio | Acción Manual en la UI | Estado en Ressy | Gap / Herramienta a Crear |
| :--- | :--- | :--- | :--- |
| **Presentaciones** | Crear presentación | ✅ `create_presentation` | — |
| **Presentaciones** | Listar presentaciones | ✅ `list_presentations` | — |
| **Presentaciones** | Ver detalle de show | ✅ `get_presentation` | — |
| **Presentaciones** | Renombrar presentación | ❌ Faltante | **`renombrar_presentacion`** |
| **Presentaciones** | Eliminar presentación | ❌ Faltante | **`eliminar_presentacion`** (Destructiva) |
| **Elementos Show** | Añadir letra | ✅ `add_lyrics` | — |
| **Elementos Show** | Añadir nota | ✅ `add_note` | — |
| **Elementos Show** | Subir pista de audio | ✅ `add_track_from_staged` | — |
| **Elementos Show** | Descargar pista de YouTube | ✅ `download_youtube_track` | — |
| **Elementos Show** | Separación vocal | ✅ `separate_vocals` | — |
| **Elementos Show** | Reordenar canción/nota | ❌ Faltante | **`reordenar_elemento`** |
| **Elementos Show** | Editar contenido de letra/nota/título | ❌ Faltante | **`editar_elemento`** |
| **Elementos Show** | Eliminar canción/nota | ❌ Faltante | **`eliminar_elemento`** (Destructiva) |
| **Eventos** | Crear evento | ✅ `crear_evento` | — |
| **Eventos** | Listar eventos | ✅ `listar_eventos` | — |
| **Eventos** | Editar evento (nombre/fecha/lugar/aforo) | ❌ Faltante | **`editar_evento`** |
| **Eventos** | Finalizar evento (cerrar ventas/edición) | ❌ Faltante | **`finalizar_evento`** (Destructiva/Estado) |
| **Eventos** | Reabrir evento finalizado | ❌ Faltante | **`reabrir_evento`** |
| **Eventos** | Eliminar evento (en cascada) | ❌ Faltante | **`eliminar_evento`** (Destructiva) |
| **Eventos** | Subir/Reemplazar afiche del evento | ❌ Faltante | **`subir_afiche`** (Requiere imagen en chat) |
| **Entradas** | Vender / emitir entradas | ✅ `vender_entradas` | — |
| **Entradas** | Confirmar pago pendiente | ✅ `confirmar_pago_entrada` | — |
| **Entradas** | Check-in en puerta con código QR | ✅ `checkin_entrada` | — |
| **Entradas** | Eliminar entrada emitida | ❌ Faltante | **`eliminar_entrada`** (Destructiva) |
| **Gastos** | Registrar gasto | ✅ `registrar_gasto` | — |
| **Gastos** | Editar gasto existente | ❌ Faltante | **`editar_gasto`** |
| **Gastos** | Eliminar gasto | ❌ Faltante | **`eliminar_gasto`** (Destructiva) |
| **Finanzas** | Resumen financiero del evento | ✅ `resumen_evento` | — |
| **Interesados** | Registrar interesado / promotora | ✅ `registrar_interesado` | — |
| **Interesados** | Editar interesado | ❌ Faltante | **`editar_interesado`** |
| **Interesados** | Eliminar interesado | ❌ Faltante | **`eliminar_interesado`** (Destructiva) |
| **Interesados** | Confirmar / Convertir en venta | ❌ Faltante | **`confirmar_interesado`** |

**Resultado del inventario:**
- Herramientas actuales: 16.
- Herramientas nuevas necesarias: **16**.
- Total final de herramientas: **32**.

---

## 3. Reutilización de Casos de Uso Existentes (Backend)

La arquitectura actual de Shirley Show destaca por su modularidad limpia (`AGENTS.md`). La totalidad de las operaciones de negocio requeridas **ya está implementada y probada** en la capa de casos de uso y en el repositorio SQLite. No se requiere escribir nueva lógica de persistencia ni mutaciones de base de datos; solo se debe conectar el dispatcher a las instancias existentes en `AppState` y `TicketingServices`.

| Herramienta Nueva | Caso de Uso Existente | Ubicación | Firma del Método |
| :--- | :--- | :--- | :--- |
| `renombrar_presentacion` | `RenamePresentation` | `src/use_cases/rename_presentation.rs` | `execute(id, new_title)` |
| `eliminar_presentacion` | `DeletePresentation` | `src/use_cases/delete_presentation.rs` | `execute(id)` |
| `reordenar_elemento` | `ReorderItem` | `src/use_cases/reorder_item.rs` | `execute_target_position(pres_id, item_id, target_pos)` o `execute(pres_id, item_id, dir)` |
| `editar_elemento` | `UpdateShowItem` | `src/use_cases/update_show_item.rs` | `execute(pres_id, item_id, title, body)` |
| `eliminar_elemento` | `DeleteItem` | `src/use_cases/delete_item.rs` | `execute(pres_id, item_id)` |
| `editar_evento` | `UpdateEvent` | `src/use_cases/ticketing/events.rs` | `execute(UpdateEventInput)` |
| `finalizar_evento` | `FinalizeEvent` | `src/use_cases/ticketing/events.rs` | `execute(id)` |
| `reabrir_evento` | `ReopenEvent` | `src/use_cases/ticketing/events.rs` | `execute(id)` |
| `eliminar_evento` | `DeleteEvent` | `src/use_cases/ticketing/events.rs` | `execute(id)` (elimina afiche en disco + cascada DB) |
| `subir_afiche` | `ReplaceEventPoster` | `src/use_cases/ticketing/posters.rs` | `execute(evento_id, &[u8])` |
| `eliminar_entrada` | `DeleteTicket` | `src/use_cases/ticketing/tickets.rs` | `execute(id)` |
| `editar_gasto` | `UpdateExpense` | `src/use_cases/ticketing/expenses.rs` | `execute(UpdateExpenseInput)` |
| `eliminar_gasto` | `DeleteExpense` | `src/use_cases/ticketing/expenses.rs` | `execute(id)` |
| `editar_interesado` | `UpdateInterest` | `src/use_cases/ticketing/interests.rs` | `execute(UpdateInterestInput)` |
| `eliminar_interesado` | `DeleteInterest` | `src/use_cases/ticketing/interests.rs` | `execute(id)` |
| `confirmar_interesado` | `ConvertInterestToSale` | `src/use_cases/ticketing/interests.rs` | `execute(ConvertInterestInput)` |

---

## 4. Lista Cerrada de Nuevas Herramientas con Schemas JSON

Todas las nuevas herramientas mantienen nombres en español consistentes con las herramientas existentes de ticketing, validación estricta (`additionalProperties: false`) y documentación clara de tipos.

### 4.1 Presentaciones y Elementos (5 herramientas)

```json
[
  {
    "name": "renombrar_presentacion",
    "description": "Cambia el nombre o título de una presentación musical existente.",
    "parameters": {
      "type": "object",
      "properties": {
        "presentacion_id": { "type": "string", "description": "Identificador único de la presentación." },
        "nuevo_nombre": { "type": "string", "description": "Nuevo nombre o título para la presentación." }
      },
      "required": ["presentacion_id", "nuevo_nombre"],
      "additionalProperties": false
    }
  },
  {
    "name": "eliminar_presentacion",
    "description": "Elimina permanentemente una presentación musical y todos sus elementos (letras, notas y pistas). Requiere confirmación previa.",
    "parameters": {
      "type": "object",
      "properties": {
        "presentacion_id": { "type": "string", "description": "Identificador único de la presentación a eliminar." }
      },
      "required": ["presentacion_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "reordenar_elemento",
    "description": "Cambia el orden o posición de un elemento (canción, letra o nota) dentro de una presentación musical.",
    "parameters": {
      "type": "object",
      "properties": {
        "presentacion_id": { "type": "string", "description": "Identificador de la presentación." },
        "elemento_id": { "type": "string", "description": "Identificador del elemento a mover." },
        "posicion_destino": { "type": "integer", "minimum": 1, "description": "Nueva posición numérica del elemento (1 para la primera posición)." },
        "direccion": { "type": "string", "enum": ["arriba", "abajo"], "description": "Opcional: mover una posición hacia 'arriba' o 'abajo'." }
      },
      "required": ["presentacion_id", "elemento_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "editar_elemento",
    "description": "Actualiza el texto de una letra o nota, o el título de una pista de audio existente.",
    "parameters": {
      "type": "object",
      "properties": {
        "presentacion_id": { "type": "string", "description": "Identificador de la presentación." },
        "elemento_id": { "type": "string", "description": "Identificador único del elemento a editar." },
        "texto": { "type": "string", "description": "Nuevo texto para elementos de tipo letra o nota." },
        "titulo": { "type": "string", "description": "Nuevo título para elementos de tipo pista de audio." }
      },
      "required": ["presentacion_id", "elemento_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "eliminar_elemento",
    "description": "Elimina permanentemente un elemento (canción, letra o nota) de una presentación. Requiere confirmación previa.",
    "parameters": {
      "type": "object",
      "properties": {
        "presentacion_id": { "type": "string", "description": "Identificador de la presentación." },
        "elemento_id": { "type": "string", "description": "Identificador del elemento a eliminar." }
      },
      "required": ["presentacion_id", "elemento_id"],
      "additionalProperties": false
    }
  }
]
```

### 4.2 Eventos y Afiche (5 herramientas)

```json
[
  {
    "name": "editar_evento",
    "description": "Actualiza la información básica de un evento existente (nombre, fecha/hora, lugar y configuración de tipos de entrada).",
    "parameters": {
      "type": "object",
      "properties": {
        "evento_id": { "type": "string", "description": "Identificador único del evento a actualizar." },
        "nombre": { "type": "string", "description": "Nombre o título actualizado del evento." },
        "fecha_hora": { "type": "string", "description": "Fecha y hora en formato ISO 8601 (YYYY-MM-DDTHH:MM)." },
        "lugar": { "type": "string", "description": "Lugar o recinto del evento." },
        "tipos_de_entrada": {
          "type": "array",
          "description": "Lista completa de tipos de entrada configurados para el evento.",
          "items": {
            "type": "object",
            "properties": {
              "nombre": { "type": "string" },
              "precio": { "type": "number", "minimum": 0.0 },
              "capacidad": { "type": "integer", "minimum": 1 }
            },
            "required": ["nombre", "precio", "capacidad"],
            "additionalProperties": false
          }
        }
      },
      "required": ["evento_id", "nombre", "fecha_hora", "lugar", "tipos_de_entrada"],
      "additionalProperties": false
    }
  },
  {
    "name": "finalizar_evento",
    "description": "Finaliza un evento activo, cerrando la venta de entradas y bloqueando nuevas modificaciones operativas. Requiere confirmación previa.",
    "parameters": {
      "type": "object",
      "properties": {
        "evento_id": { "type": "string", "description": "Identificador único del evento a finalizar." }
      },
      "required": ["evento_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "reabrir_evento",
    "description": "Reabre un evento previamente finalizado para permitir continuar su gestión y ventas.",
    "parameters": {
      "type": "object",
      "properties": {
        "evento_id": { "type": "string", "description": "Identificador único del evento a reabrir." }
      },
      "required": ["evento_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "eliminar_evento",
    "description": "Elimina permanentemente un evento junto con todas sus entradas, gastos, interesados y afiche. Requiere confirmación previa estricta.",
    "parameters": {
      "type": "object",
      "properties": {
        "evento_id": { "type": "string", "description": "Identificador único del evento a eliminar." }
      },
      "required": ["evento_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "subir_afiche",
    "description": "Asigna o reemplaza la imagen del afiche promocional del evento utilizando la imagen adjunta en la conversación.",
    "parameters": {
      "type": "object",
      "properties": {
        "evento_id": { "type": "string", "description": "Identificador único del evento al que pertenece el afiche." }
      },
      "required": ["evento_id"],
      "additionalProperties": false
    }
  }
]
```

### 4.3 Entradas, Gastos e Interesados (6 herramientas)

```json
[
  {
    "name": "eliminar_entrada",
    "description": "Anula y elimina permanentemente una entrada emitida. Requiere confirmación previa.",
    "parameters": {
      "type": "object",
      "properties": {
        "entrada_id": { "type": "string", "description": "Identificador único de la entrada a eliminar." }
      },
      "required": ["entrada_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "editar_gasto",
    "description": "Modifica un gasto registrado para corregir su categoría, monto, descripción o fecha.",
    "parameters": {
      "type": "object",
      "properties": {
        "gasto_id": { "type": "string", "description": "Identificador único del gasto a modificar." },
        "categoria": {
          "type": "string",
          "enum": [
            "Local/Alquiler",
            "Sonido/Luces",
            "Músicos/Personal",
            "Publicidad/Marketing",
            "Bebidas/Catering",
            "Transporte",
            "Otros"
          ],
          "description": "Categoría contable del gasto."
        },
        "monto": { "type": "number", "minimum": 0.0, "description": "Monto en soles (S/)." },
        "descripcion": { "type": "string", "description": "Detalle del gasto." },
        "fecha": { "type": "string", "description": "Fecha del gasto (YYYY-MM-DD)." }
      },
      "required": ["gasto_id", "categoria", "monto", "descripcion", "fecha"],
      "additionalProperties": false
    }
  },
  {
    "name": "eliminar_gasto",
    "description": "Elimina permanentemente un registro de gasto. Requiere confirmación previa.",
    "parameters": {
      "type": "object",
      "properties": {
        "gasto_id": { "type": "string", "description": "Identificador único del gasto a eliminar." }
      },
      "required": ["gasto_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "editar_interesado",
    "description": "Modifica los datos o estado de una persona interesada o promotora registrada.",
    "parameters": {
      "type": "object",
      "properties": {
        "interesado_id": { "type": "string", "description": "Identificador único del interesado." },
        "nombre": { "type": "string", "description": "Nombre de la persona interesada." },
        "contacto": { "type": "string", "description": "Teléfono o celular de contacto." },
        "canal": { "type": "string", "description": "Canal de contacto (ej: WhatsApp, Instagram)." },
        "personas": { "type": "integer", "minimum": 1, "description": "Número estimado de personas." },
        "promotora": { "type": "string", "description": "Nombre de la promotora asignada." },
        "estado": { "type": "string", "enum": ["Pendiente", "Confirmado", "Cancelado"], "description": "Estado de la reserva." }
      },
      "required": ["interesado_id", "nombre", "contacto", "canal", "personas", "promotora", "estado"],
      "additionalProperties": false
    }
  },
  {
    "name": "eliminar_interesado",
    "description": "Elimina permanentemente a una persona de la lista de interesados. Requiere confirmación previa.",
    "parameters": {
      "type": "object",
      "properties": {
        "interesado_id": { "type": "string", "description": "Identificador único del interesado a eliminar." }
      },
      "required": ["interesado_id"],
      "additionalProperties": false
    }
  },
  {
    "name": "confirmar_interesado",
    "description": "Convierte a un interesado en una venta de entradas formal, emitiendo los tickets correspondientes.",
    "parameters": {
      "type": "object",
      "properties": {
        "interesado_id": { "type": "string", "description": "Identificador único del interesado." },
        "tipo_entrada": { "type": "string", "description": "Tipo de entrada asignado a la venta." },
        "cantidad": { "type": "integer", "minimum": 1, "description": "Cantidad de entradas a emitir." },
        "medio_pago": { "type": "string", "enum": ["Yape", "Efectivo", "Cortesía"], "description": "Medio de pago." },
        "estado_pago": { "type": "string", "enum": ["Pagado", "Pendiente"], "description": "Estado de pago inicial." }
      },
      "required": ["interesado_id", "tipo_entrada", "cantidad", "medio_pago", "estado_pago"],
      "additionalProperties": false
    }
  }
]
```

---

## 5. Reglas de Confirmación para Operaciones Destructivas

### 5.1 Catálogo de Operaciones Destructivas
Son acciones irreversibles que eliminan datos o alteran el ciclo de vida crítico:
1. `eliminar_presentacion`
2. `eliminar_elemento`
3. `eliminar_evento`
4. `finalizar_evento`
5. `eliminar_entrada`
6. `eliminar_gasto`
7. `eliminar_interesado`

### 5.2 Protocolo de Confirmación Obligatorio (Prompt & Dispatcher)
Para evitar eliminaciones accidentales por alucinación o solicitud impulsiva:
1. **NUNCA ejecutar en el primer turno:** Cuando el usuario solicite por primera vez eliminar o finalizar cualquiera de estas entidades, Ressy **NO** debe invocar la herramienta destructiva.
2. **Advertencia y contexto explícito:** Ressy identifica el elemento a eliminar con su nombre/descripción visible e informa las consecuencias (por ejemplo: *"Si eliminas el evento 'Concierto Acústico', también se borrarán todas sus entradas vendidas, gastos y el afiche"*).
3. **Solicitud de frase textual de confirmación:** Ressy solicita una confirmación afirmativa explícita formulando la pregunta exacta:
   - Para eliminaciones: `¿Estás segura? Por favor confirma diciendo: sí, elimínalo`
   - Para finalizaciones: `¿Estás segura? Por favor confirma diciendo: sí, finalízalo`
4. **Ejecución en segundo turno:** Únicamente cuando el usuario responde en el siguiente turno con una confirmación explícita (e.g., *"sí, elimínalo"*, *"sí elimínalo"*, *"confirmo"*, *"sí, finalízalo"*), Ressy ejecuta la herramienta.
5. **Cancelación amigable:** Si el usuario desiste o expresa duda (*"mejor no"*, *"espera"*, *"cancela"*), Ressy cancela la operación sin llamar a ninguna herramienta y responde con calidez.

---

## 6. Soporte de Imágenes en Chat y Subida de Afiches

### 6.1 Limitación Actual
Actualmente, el chat del agente solo admite subida de audio:
- `frontend/js/components/organisms/agent-chat-composer.js` tiene `accept: '.mp3,.wav,.m4a,audio/...'`.
- `frontend/js/data/agent-chat-api.js` rechaza cualquier archivo que no sea audio y tenga más de 100 MiB.
- `src/handlers/agent/multipart_request.rs` asume que el campo `"file"` es un archivo de audio y lo procesa hacia `detect_audio_format`.

### 6.2 Diseño de Soporte de Imagen en Chat
Para habilitar `subir_afiche`:
1. **Composer en Frontend (`agent-chat-composer.js`):**
   - Extender el input de archivo: `accept: '.mp3,.wav,.m4a,.jpg,.jpeg,.png,.webp,audio/*,image/*'`.
   - Generalizar la validación de adjunto en `agent-chat-api.js`:
     - Si es audio: límite de 100 MiB, extensiones `.mp3, .wav, .m4a`.
     - Si es imagen: límite de 10 MiB (coherente con `MAX_POSTER_SIZE_BYTES`), extensiones `.jpg, .jpeg, .png, .webp`.
     - Otros tipos: rechazo con mensaje claro.
   - Chip de adjunto (`agent-attachment-chip.js`): visualización clara de nombre y tamaño tanto para audio como para imagen.
2. **Backend Multipart (`multipart_request.rs`):**
   - Al recibir el stream de `"file"`, inspeccionar los primeros bytes mediante `ImageFormat::detect`:
     - Si es imagen válida (`Jpeg`, `Png` o `Webp`) y su tamaño es $\le 10 \text{ MiB}$, empaquetar como `StagedImageHandle::Available { data: Vec<u8>, format: ImageFormat, byte_size }`.
     - Si es audio, mantener el pipeline existente de `StagingFileGuard` y `detect_audio_format`.
     - Si no coincide con ninguno, responder `400 Bad Request`.
3. **Turn Context & Dispatcher (`turn_context.rs`, `tool_dispatcher.rs`):**
   - `TurnToolContext` almacena `staged_image: Option<StagedImageHandle>`.
   - Al invocarse `subir_afiche(evento_id)`:
     - El dispatcher verifica `context.staged_image`. Si existe, llama directamente a `self.ticketing.replace_poster.execute(evento_id, &data)`.
     - Marca el handle como consumido.
     - Registra la acción `ToolAction` y emite evento reactivo para refrescar la vista de evento en el cliente.
     - Si no hay imagen en el contexto del turno, devuelve error guiado: *"No se encontró ninguna imagen adjunta. Por favor sube la imagen del afiche junto con tu mensaje."*

---

## 7. Análisis de Riesgos y Mitigaciones

| Riesgo Identificado | Severidad | Mitigación Arquitectónica |
| :--- | :--- | :--- |
| **Acciones destructivas ejecutadas por error** | **Alta** | Regla estricta de 2 turnos con frase explícita obligatoria (`sí, elimínalo`). La herramienta nunca se ejecuta en el turno de la primera intención. |
| **Crecimiento del catálogo de tools (16 → 32)** | **Media** | Nombres descriptivos y ortogonales en español. Schemas con `additionalProperties: false` y tipos estrictos. Claridad contextual inyectando la presentación o evento activo en el prompt. |
| **Sobrecarga de tokens en System Prompt** | **Media** | Redacción compacta y unificada de las reglas de confirmación en la sección de directrices generales, evitando duplicar texto por cada herramienta individual. Mantener el prompt del sistema dentro del presupuesto óptimo de Groq (< 2.500 tokens). |
| **Confusión entre audio e imagen en chat** | **Baja** | Detección robusta por magic bytes en backend (`ImageFormat::detect` vs `detect_audio_format`) sin confiar únicamente en la extensión declarada por el cliente. |

---

## 8. Estrategia de Slices (Límite: 400 Líneas por Slice)

Siguiendo el delivery budget acordado de 400 líneas, el cambio se divide en 3 slices independientes y verificables:

```
shirley-show-agent-parity
├── Slice 1: Safe Tools (Lectura y Edición Segura) [~320 líneas]
├── Slice 2: Destructive Tools & Confirmation Flow [~350 líneas]
└── Slice 3: Poster Upload & Chat Image Attachment [~380 líneas]
```

### Slice 1: Safe Tools (Lectura y Edición Segura)
- **Alcance:** Registro y despacho de las 8 herramientas no-destructivas de edición y reordenamiento.
- **Herramientas:** `renombrar_presentacion`, `reordenar_elemento`, `editar_elemento`, `editar_evento`, `reabrir_evento`, `editar_gasto`, `editar_interesado`, `confirmar_interesado`.
- **Archivos:**
  - `src/agent/tool_schemas.rs`: Definición de los 8 nuevos schemas.
  - `src/agent/tool_dispatcher.rs`: Argument structs y despacho hacia use cases existentes.
  - `src/agent/system_prompt.rs`: Instrucciones operativas para edición y reordenamiento.
  - `tests/agent_tools_tests.rs` / `tests/ticketing_ressy_tests.rs`: Actualización de assertions de conteo de tools y tests unitarios.

### Slice 2: Destructive Tools & Confirmation Flow
- **Alcance:** Registro, despacho y política de confirmación obligatoria para las 7 herramientas destructivas.
- **Herramientas:** `eliminar_presentacion`, `eliminar_elemento`, `eliminar_evento`, `finalizar_evento`, `eliminar_entrada`, `eliminar_gasto`, `eliminar_interesado`.
- **Archivos:**
  - `src/agent/tool_schemas.rs`: Definición de los 7 schemas destructivos.
  - `src/agent/tool_dispatcher.rs`: Despacho de eliminaciones hacia use cases existentes.
  - `src/agent/system_prompt.rs`: Actualización de la directriz 5 e incorporación de la regla obligatoria de confirmación previa de 2 turnos (*"sí, elimínalo"* / *"sí, finalízalo"*).
  - Tests unitarios de schemas, dispatcher y flujo de confirmación.

### Slice 3: Poster Upload & Chat Image Attachment
- **Alcance:** Soporte de adjuntos de imagen en la interfaz del chat y herramienta `subir_afiche`.
- **Herramienta:** `subir_afiche`.
- **Archivos:**
  - `frontend/js/data/agent-chat-api.js`: Soporte de validación de imágenes (`MAX_IMAGE_BYTES = 10 MiB`, extensiones de imagen).
  - `frontend/js/components/organisms/agent-chat-composer.js`: Input de archivos aceptando `image/*`.
  - `src/handlers/agent/multipart_request.rs`: Detección de imagen vía `ImageFormat::detect` y creación de `StagedImageHandle`.
  - `src/agent/turn_context.rs`: Almacenamiento de `staged_image` en `TurnToolContext`.
  - `src/agent/tool_schemas.rs`: Schema de `subir_afiche`.
  - `src/agent/tool_dispatcher.rs`: Despacho de `subir_afiche` consumiendo `staged_image` y llamando a `ReplaceEventPoster`.
  - Tests de integración de multipart, subida de afiche y validaciones.

---

## 9. Conclusión del Explore

El objetivo de paridad total para Ressy es 100% viable y se apoya directamente en la arquitectura preexistente del proyecto. La base de datos, los repositorios y los casos de uso ya implementan cada una de las 16 operaciones faltantes. Al implementar estos 3 slices ordenados, Ressy obtendrá la capacidad completa de operar el sistema Shirley Show por voz o texto, garantizando máxima seguridad contra eliminaciones accidentales y extendiendo el chat multimodal para recibir afiches fotográficos de eventos.

---

## Key Learnings
1. Toda la funcionalidad operativa requerida para la paridad de Ressy ya existe en los use cases de backend en `src/use_cases/` y `src/use_cases/ticketing/`.
2. Las operaciones destructivas en asistentes conversacionales requieren un protocolo de confirmación en dos turnos con frases afirmativas específicas para prevenir pérdidas accidentales de datos.
3. El soporte multipart de chat puede bifurcar transparentemente entre audio e imágenes mediante inspección de magic bytes en los primeros bytes del stream.
4. El crecimiento de la allowlist de 16 a 32 herramientas requiere esquemas JSON estrictos y nombres ortogonales en español para asegurar la selección precisa por parte del LLM.
