# Diseño: paridad operativa del agente Ressy

**Change:** `shirley-show-agent-parity`  
**Estado:** listo para planificación, con gate de presupuesto antes de implementar el slice destructivo  
**Fuente:** `proposal.md`, `explore.md`, `specs/agent-tools/spec.md`, `specs/agent-chat/spec.md`

## Resumen de la solución

Ressy ampliará su catálogo de 16 a 32 herramientas sin crear reglas de negocio nuevas: el dispatcher validará argumentos y contexto, y delegará las mutaciones a los casos de uso existentes. Las ocho herramientas seguras se ejecutarán normalmente; las siete destructivas pasarán por un registro efímero y server-side ligado a una conversación, con autorización de un solo uso válida únicamente para el siguiente turno; `subir_afiche` consumirá una imagen JPEG, PNG o WebP validada por magic bytes y acotada a 10 MiB dentro del request actual.

El frontend seguirá siendo presentacional/headless: validará tempranamente el adjunto, conservará un identificador opaco de conversación solo en memoria, mostrará un label de subida apropiado y emitirá señales de refresco después de acciones exitosas. El backend seguirá siendo la autoridad para formato, tamaño, confirmación y mutación.

## Objetivos y límites

### Objetivos

- Registrar exactamente 32 tools con schemas cerrados (`additionalProperties: false`).
- Despachar las 16 tools nuevas a los casos de uso existentes.
- Impedir determinísticamente cualquier destructiva sin autorización server-side del turno anterior.
- Aceptar un único adjunto de audio o imagen por request, clasificado por contenido.
- Consumir una imagen una sola vez mediante `ReplaceEventPoster`.
- Refrescar catálogo, presentación o evento tras mutaciones exitosas.
- Mantener el system prompt renderizado por debajo de 2.500 tokens.

### Fuera de alcance

No se agregan tools para reproducción, Stage Mode, cámara, filesystem, HTTP arbitrario, SQL ni documentos generales. No cambian los repositorios, invariantes de dominio ni cascadas existentes. No se persiste conversación ni confirmación en SQLite.

## Arquitectura resultante

```text
Composer/UI
  -> agent-chat-api (validación temprana + conversation_id)
  -> POST /api/agent/chat (JSON o multipart)
      -> parser HTTP
          -> audio: StagingFileGuard, <= 100 MiB
          -> imagen: Vec<u8> + ImageFormat, <= 10 MiB
      -> PendingDestructiveRegistry.take(conversation_id)
      -> AgentChat / TurnToolContext
          -> AgentToolDispatcher
              -> casos de uso existentes
              -> PendingDestructiveRegistry.arm(...) si falta confirmación
      -> AgentChatOutput (acciones + contexto + conversation_id)
  -> chat-state / señales de refresco
  -> vistas de catálogo, presentación o evento
```

La dirección se conserva: transporte y agente dependen de casos de uso; los casos de uso continúan siendo propietarios de reglas y persistencia. La UI no conoce `ReplaceEventPoster`, el registro server-side ni los repositorios.

## Contratos de turno

### Imagen staged

En `src/agent/turn_context.rs` se añade:

```rust
pub enum StagedImageHandle {
    Available {
        data: Vec<u8>,
        format: ImageFormat,
        byte_size: usize,
    },
    Consumed,
    Failed,
}
```

`TurnToolContext` obtiene `staged_image: Option<StagedImageHandle>`. El handle existe solo durante el request y se mueve con `take()` antes de llamar a `ReplaceEventPoster`:

- éxito: queda `Consumed`;
- fallo del caso de uso: queda `Failed` y los bytes se liberan;
- ausencia: `no_image_staged`;
- segundo uso: `image_already_consumed`.

No se crea un registro durable de imágenes ni un archivo staging adicional. Diez MiB es un límite aceptable para memoria request-scoped y evita que un afiche sobreviva por accidente a un turno.

### Acción destructiva pendiente

Se añade `src/agent/destructive_confirmation.rs` como módulo cohesivo de seguridad del agente:

```rust
pub struct PendingDestructiveAction {
    pub tool: DestructiveTool,
    pub target: DestructiveTarget,
    pub target_label: String,
    pub created_at: Instant,
}

pub enum DestructiveTarget {
    Presentation { id: String },
    ShowItem { presentation_id: String, item_id: String },
    Event { id: String },
    Ticket { id: String },
    Expense { id: String },
    Interest { id: String },
}
```

El tipo conoce la frase requerida, los argumentos canónicos y la consecuencia que debe comunicarse. El target tipado evita colisiones y conserva ambos IDs para `eliminar_elemento`.

`TurnToolContext` incorpora:

- `conversation_id: String`;
- `pending_destructive_action: Option<PendingDestructiveAction>`, obtenido del servidor al iniciar el turno;
- `confirmation_reply: ConfirmationReply` (`ExactDelete`, `ExactFinalize`, `Cancel`, `Ambiguous`, `Other`);
- `confirmation_requested: Option<PendingDestructiveAction>` para que la respuesta final incluya siempre el aviso determinístico.

### Correlación server-side

Como el endpoint actual es stateless y no hay login propio, el API añade `conversation_id` opcional a JSON y multipart, y lo devuelve siempre que el request haya sido parseado. El servidor genera un UUID v4 cuando falta. `chat-state.js` lo conserva solo en memoria y el cliente lo reenvía en turnos posteriores.

`PendingDestructiveRegistry`, compartido mediante `Arc` en `AppState`, guarda `conversation_id -> PendingDestructiveAction`. El UUID es un identificador opaco de correlación, no reemplaza la autenticación externa asumida por el producto.

Semántica del registro:

1. `arm` reemplaza cualquier pendiente previa de esa conversación.
2. `take` remueve atómicamente la pendiente al comenzar el siguiente request; dos requests concurrentes no pueden recibir la misma autorización.
3. El valor tomado se instala en `TurnToolContext` y nunca se reinserta después de ejecutar o cambiar de tema.
4. Una respuesta ambigua de la lista cerrada (`dale`, `ok`, `hazlo`, `bueno`, `procede` o una casi-confirmación incorrecta) no autoriza; crea una nueva pendiente para el mismo target y vuelve a pedir la frase exacta. Cada registro individual sigue siendo válido para un solo turno.
5. Cancelación o tema distinto descartan el valor sin rearmarlo.
6. Las entradas no reclamadas se purgan perezosamente tras 15 minutos para acotar memoria. Un reinicio del servidor cancela todas las pendientes, que es el comportamiento seguro.

La normalización de confirmación solo acepta mayúsculas/minúsculas, espacios exteriores y la coma opcional prevista por la spec. Las únicas frases autorizantes son `sí, elimínalo` / `sí elimínalo` y `sí, finalízalo` / `sí finalízalo`; texto adicional no autoriza.

## Flujo de confirmación destructiva

### Turno inicial

1. El modelo emite una tool destructiva para el target resuelto.
2. El dispatcher parsea, verifica pertenencia al contexto y consulta el recurso para obtener un label confiable.
3. Al no existir una pendiente tomada del turno anterior con frase válida, no invoca el caso de uso.
4. Arma `PendingDestructiveRegistry` y devuelve `confirmation_required` con tool, target, label, consecuencia y frase.
5. `AgentChat` incorpora ese texto determinístico aun si la redacción del modelo omite parte del aviso.

Una tool repetida en el mismo turno reutiliza el resultado `confirmation_required`; no rearma ni muta.

### Siguiente turno

- **Confirmación exacta y tipo correcto:** `AgentChat` construye una `ToolCall` interna desde los argumentos canónicos de la pendiente tomada y la pasa por el dispatcher. El modelo no puede sustituir el target. El dispatcher vuelve a comparar tool y target, consume `pending_destructive_action` antes de invocar el caso de uso y registra el resultado.
- **Confirmación exacta sin pendiente:** no hay ejecución; historia enviada por el cliente no aporta autorización.
- **Target/tool distintos:** `destructive_confirmation_mismatch`, sin mutación.
- **Ambigua:** no ejecuta; informa que no se realizó y rearma el mismo target para pedir la frase correcta.
- **Cancelación:** respuesta cálida y determinística, sin provider ni tool.
- **Petición no relacionada:** la pendiente expira y el nuevo pedido continúa normalmente.
- **Tercer turno repetido:** como `take` ya vació el registro y la pendiente fue consumida, no puede reejecutar.

La ejecución exacta se resuelve antes de solicitar una nueva completion. El LLM ayuda a descubrir el target en el primer turno, pero no participa en la decisión de autorización del segundo.

## Dispatcher y casos de uso

### Tools seguras

| Tool | Validación adicional | Caso de uso | Refresh |
| --- | --- | --- | --- |
| `renombrar_presentacion` | presentación conocida; nombre no vacío | `RenamePresentation` | catálogo + presentación |
| `reordenar_elemento` | presentación conocida; exactamente posición o dirección | `ReorderItem` | presentación |
| `editar_elemento` | exactamente `texto` o `titulo`, compatible con kind | `UpdateShowItem` | presentación |
| `editar_evento` | evento autorizado; lista no vacía; tipos/precios/capacidades válidos | `UpdateEvent` | evento + lista de eventos |
| `reabrir_evento` | evento autorizado | `ReopenEvent` | evento + lista de eventos |
| `editar_gasto` | recurso pertenece a evento autorizado; enum, Money y fecha válidos | `UpdateExpense` | evento |
| `editar_interesado` | recurso pertenece a evento autorizado; enum y cantidad válidos | `UpdateInterest` | evento |
| `confirmar_interesado` | recurso pertenece a evento autorizado; pago/cantidad válidos | `ConvertInterestToSale` | evento |

`reordenar_elemento` rechaza ambos selectores a la vez para evitar precedencia implícita. `editar_elemento` refleja la invariante ya existente de `UpdateShowItem`: exactamente un campo según el tipo del elemento.

### Tools destructivas

| Tool | Target confiable | Caso de uso tras confirmar | Consecuencia comunicada |
| --- | --- | --- | --- |
| `eliminar_presentacion` | `GetPresentation` | `DeletePresentation` | presentación y todos sus elementos |
| `eliminar_elemento` | `ShowItemReader::get` bajo presentación | `DeleteItem` | elemento y audio asociado si aplica |
| `eliminar_evento` | `GetEvent` | `DeleteEvent` | entradas, gastos, interesados y afiche |
| `finalizar_evento` | `GetEvent` | `FinalizeEvent` | cierre de ventas y mutaciones operativas |
| `eliminar_entrada` | `GetTicket` | `DeleteTicket` | anulación/eliminación de entrada emitida |
| `eliminar_gasto` | `GetExpense` | `DeleteExpense` | retiro del gasto y cambio del resumen |
| `eliminar_interesado` | `GetInterest` | `DeleteInterest` | retiro definitivo del lead |

Para ticket, gasto e interesado, la consulta previa deriva `evento_id` y comprueba que pertenezca al evento activo/conocido. No se agregan sets de IDs confiados a la conversación para esas entidades.

### Afiche

`subir_afiche({ evento_id })` exige evento autorizado y `StagedImageHandle::Available`. Llama exclusivamente a `ticketing.replace_poster.execute(evento_id, &data)`. Devuelve `evento_id`, filename, formato y `byte_size`; registra `ToolAction { tool: "subir_afiche", event_id, status: "success" }`.

`ReplaceEventPoster` conserva la responsabilidad de validar existencia, almacenar el archivo, actualizar el evento, retirar el afiche anterior y revertir ante fallo. El formato se vuelve a validar en storage como defensa en profundidad.

### Errores, receipts y deduplicación

- Todos los structs de argumentos usan `#[serde(deny_unknown_fields)]` además del schema JSON.
- Los errores de dominio mantienen `error` y `message`; `subir_afiche` traduce específicamente evento ausente a `evento_not_found` para cumplir su contrato.
- Una llamada que alcanzó un caso de uso registra `ToolAction` con `status: "success"` o `"error"`. `confirmation_required` no es una acción ejecutada.
- `compose_grounded_message` distingue receipts exitosos de fallidos; no presenta un error como mutación confirmada.
- El frontend refresca únicamente acciones con `status === "success"`.
- La deduplicación existente por call ID y firma canónica se extiende automáticamente a las 16 tools. La confirmación sintética consume la pendiente antes del caso de uso, por lo que un fallo tampoco permite un segundo intento con la misma autorización.

## Schemas y allowlist

`tool_schemas.rs` se organiza en bloques cohesivos (presentaciones, items, eventos/afiche, entradas, gastos e interesados) sin cambiar el contrato `Vec<ToolDefinition>`. Se agregan las 16 definiciones cerradas de la spec y se conserva el listado exacto de 32 nombres.

Las restricciones representables se expresan en JSON Schema (`minimum`, enums, required y `additionalProperties: false`). Las invariantes cruzadas —exactamente un selector de reordenamiento y exactamente un campo editable— también se validan en Rust, porque el dispatcher es la frontera autoritativa aunque el provider ignore combinadores de schema.

## Multipart y magic bytes

`multipart_request.rs` cambia de un pipeline audio-only a uno de clasificación temprana:

1. Rechazar un segundo campo `file` con `duplicate_file_field`.
2. Acumular como máximo 32 bytes iniciales antes de crear staging definitivo.
3. Usar extensión/MIME solo como familia declarada; nunca como prueba de formato:
   - familia imagen declarada: exige `ImageFormat::detect`;
   - familia audio declarada: exige magic bytes MP3/WAV/M4A;
   - metadata genérica: intenta imagen y luego audio por magic bytes.
4. Si es imagen, copiar a `Vec<u8>` y abortar en cuanto supera `MAX_POSTER_SIZE_BYTES` (10.485.760 bytes).
5. Si es audio, crear `StagingFileGuard`, escribir header y stream restante, y abortar al superar 100 MiB.
6. Si ninguna firma coincide, devolver 400 sin invocar Groq.

Se extrae una variante content-only del detector de audio para el chat; el upload manual existente puede conservar la comprobación extensión+firma. El límite de afiche se centraliza como constante de dominio compartida por handler, caso de uso y storage, evitando tres valores divergentes.

Si el mensaje multipart está vacío, el default depende del contenido: `Añadir esta pista de audio` o `Usar esta imagen como afiche del evento activo`. Al salir del request, `Vec<u8>` se libera; el guard de audio conserva su limpieza RAII actual.

## System prompt

No se anexan 16 párrafos al prompt actual. `system_prompt.rs` se reescribe en secciones compactas:

1. identidad, idioma y veracidad;
2. resolución de IDs/contexto;
3. requisitos mínimos por familia de tools;
4. audio/YouTube/separación vocal;
5. ticketing y contabilidad;
6. bloque único de destructivas con frases exactas;
7. imagen staged y `subir_afiche`;
8. exclusiones locales y cancelación.

El prompt indica que la primera tool destructiva solo solicita confirmación y que el servidor decide la autorización. `build_context_system_prompt` añade indicadores breves para imagen, audio, evento y pendiente server-side, sin incluir historia como evidencia.

Se aplican dos gates:

- test offline conservador de tamaño de caracteres/palabras para impedir crecimiento accidental;
- verificación con el tokenizer publicado del modelo Groq configurado, registrada durante verify, exigiendo `< 2.500` tokens para el contexto máximo.

No se añade una dependencia de tokenizer al binario de producción solo para esta comprobación.

## Frontend

### Validación y transporte

`agent-chat-api.js` expone `validateAgentAttachment(file)` con resultado `{ valid, kind }`:

- audio `.mp3/.wav/.m4a` o MIME exacto compatible: hasta 100 MiB;
- imagen `.jpg/.jpeg/.png/.webp` o MIME JPEG/PNG/WebP: hasta 10 MiB;
- otro tipo: error local claro.

`validateAudioAttachment` queda como wrapper compatible para consumidores/tests existentes. La validación cliente mejora feedback, pero no autoriza formato.

`sendAgentChatMessage` incluye `conversation_id` en JSON y multipart. La respuesta incorpora el ID generado/confirmado por servidor.

### Estado, composer y typing

- `chat-state.js` agrega `conversationId: null`, lo actualiza desde cada respuesta y nunca usa `localStorage`/`sessionStorage`.
- `resolveTurn` elimina el adjunto tras éxito de `add_track_from_staged` o `subir_afiche`; lo conserva ante error para un reintento explícito.
- Una imagen en vuelo usa `pendingLabel = "Ressy está subiendo el afiche…"`; el indicador existente mantiene `role=status` y `aria-live=polite`.
- `agent-chat-composer.js` usa `validateAgentAttachment`, acepta audio y `image/*` (con extensiones/MIME soportados explícitos), y cambia aria-label/title a “audio o imagen”.
- `agent-attachment-chip.js` continúa recibiendo solo `{name,size}`; se generaliza su copy de audio a archivo, sin acoplarlo al backend.
- `agent-chat-controller.js` transporta `conversationId` y conserva una única instancia de chat por sesión de página.

### Refresh

`agent-refresh.js` amplía su contrato con scopes `catalog`, `presentation`, `events` y `event`, más `removed?: boolean`.

- Presentación creada/renombrada/eliminada: refresca catálogo.
- Items y rename: refresca detalle de presentación.
- Presentación eliminada: el detalle navega al catálogo con `removed: true`.
- Evento creado/editado/reabierto/finalizado/eliminado: refresca lista de eventos.
- Tickets, gastos, interesados y afiche: refrescan el detalle del evento.
- Evento eliminado: el detalle navega a la lista con `removed: true`.

Los controladores existentes se suscriben y limpian la suscripción en `destroy`; no se introduce un bus global nuevo.

## Cambios por archivo

| Área | Archivos |
| --- | --- |
| Seguridad conversacional | nuevo `src/agent/destructive_confirmation.rs`; `src/agent/mod.rs`, `src/agent/turn_context.rs` |
| Composición | `src/state/app_state.rs` y, si se exporta el registro desde state, `src/state/mod.rs` |
| Catálogo/dispatch | `src/agent/tool_schemas.rs`, `src/agent/tool_dispatcher.rs` |
| Orquestación/prompt | `src/use_cases/agent_chat.rs`, `src/agent/system_prompt.rs` |
| HTTP | `src/handlers/agent/json_request.rs`, `multipart_request.rs`, `chat.rs` |
| Media | `src/infra/filesystem/detect_audio_format.rs`, constante de tamaño en `src/domain/ticketing/*`, `use_cases/ticketing/posters.rs`, `infra/filesystem/local_poster_storage.rs` |
| Frontend | `agent-chat-api.js`, `chat-state.js`, `agent-chat-controller.js`, `agent-chat-composer.js`, `agent-attachment-chip.js`, `agent-refresh.js`, controladores de presentaciones/eventos |
| Tests | `agent_tools_tests.rs`, `ticketing_ressy_tests.rs`, `agent_chat_tests.rs`, `agent_chat_api_tests.rs`, los tres harnesses `agent-chat-*.test.mjs` y `agent_chat_ui_tests.rs` como runner |

## Estrategia de pruebas (TDD estricto)

### Slice 1 — ocho tools seguras

**RED:** conteo/nombres/schema 24 tools intermedias; validación de enums, montos, campos mutuamente exclusivos, pertenencia de recursos y errores estructurados.  
**GREEN:** wiring de casos de uso y `ToolAction`.  
**TRIANGULATE:** éxito y fallo por cada familia, dedupe y refresh frontend.  
**REFACTOR:** agrupar parsing/receipts cohesivos sin convertir el dispatcher en una capa de negocio.

### Slice 2 — siete destructivas

**RED:** primera llamada no muta; conversación A no autoriza B; historia forjada; target/tool mismatch; confirmación exacta; confirmación incorrecta; ambigua; cancelación; tema distinto; tercero repetido; dos requests concurrentes; TTL/reinicio seguro; error not-found.  
**GREEN:** registro server-side, `conversation_id`, preflight exacto y dispatch a los siete casos de uso.  
**TRIANGULATE:** flujo HTTP completo de dos turnos, cascada de evento y cleanup de archivos.  
**REFACTOR:** helpers únicos para parsear target, advertencia y consumo; ninguna comprobación duplicada por tool.

### Slice 3 — afiche e imágenes

**RED:** JPEG/PNG/WebP; exactamente 10 MiB y 10 MiB+1; MIME/extensión spoofed; metadata genérica; bytes no soportados; audio MP3/WAV/M4A regresión; archivo duplicado; imagen ausente/fallida/consumida; evento inexistente; imagen sin tool.  
**GREEN:** clasificador streaming, `StagedImageHandle`, `subir_afiche`, API/composer/state.  
**TRIANGULATE:** multipart -> dispatcher -> poster storage -> refresh de evento; error no invoca provider cuando corresponde.  
**REFACTOR:** un único validador frontend y una única constante backend de 10 MiB.

### Verificación

- Dirigida: `cargo test --test agent_tools_tests --test ticketing_ressy_tests --test agent_chat_tests --test agent_chat_api_tests --test agent_chat_ui_tests`.
- Frontend aislado durante RED/GREEN: `node tests/agent-chat-state.test.mjs`, `node tests/agent-chat-ui-components.test.mjs`, `node tests/agent-chat-integration.test.mjs`.
- Integral: `cargo fmt --check` y `cargo test`.
- Evidencia adicional: conteo exacto de 32 tools y tokenización del prompt máximo `< 2.500`.

## Rollout, presupuesto y rollback

Se conserva el orden del proposal: safe tools, destructivas, afiche. Cada slice se habilita al registrar sus schemas; quitar una definición funciona como kill switch sin tocar datos ni casos de uso.

El diseño server-side de destructivas añade más superficie que la estimación inicial (registro, correlación HTTP, orquestación y pruebas de concurrencia). Existe riesgo real de superar 400 líneas en el slice 2. Bajo `ask-on-risk`, antes de aplicar ese slice se debe pausar y pedir una decisión de entrega; este diseño no infiere chain strategy ni excepción de tamaño.

Rollback:

1. retirar schemas nuevos para detener llamadas;
2. en destructivas, retirar primero las siete definiciones y luego el registro/correlación;
3. en imágenes, volver el composer y multipart a audio-only; los afiches ya guardados siguen siendo válidos;
4. no hay migraciones ni reversión automática de operaciones ya confirmadas.

## Decisiones y rationale

| Decisión | Rationale | Alternativa descartada |
| --- | --- | --- |
| Estado pendiente en memoria server-side y por conversación | Historia y prompt no son evidencia; aislamiento y consumo atómico evitan replay | Confiar en `history` o guardar pending solo en JS |
| `conversation_id` UUID devuelto por servidor y efímero en JS | El endpoint necesita correlación sin introducir login o persistencia | Estado global único, IP o cookie compartida entre tabs |
| `take` al comienzo del siguiente request | Materializa “un turno”, single-use y seguridad ante concurrencia | Timestamp sin consumo o autorización reutilizable |
| Ejecución sintética desde pending exacto | El target confirmado no puede ser sustituido por el modelo | Pedir al LLM que reconstruya tool/args en el segundo turno |
| Target destructivo tipado | Un item necesita dos IDs y todos los matches son explícitos | Concatenar strings o comparar solo el ID visible |
| Imagen en memoria, audio en staging RAII | 10 MiB es acotado y el afiche es request-scoped; audio puede llegar a 100 MiB | Registro durable de imágenes o cargar audio completo en RAM |
| Magic bytes autoritativos con metadata como intención | Rechaza spoofing y mantiene feedback coherente con el tipo elegido | Confiar en extensión/MIME o ignorar completamente una declaración contradictoria |
| Reutilizar casos de uso existentes | Conserva invariantes, transacciones, cascadas y cleanup en su dueño | Mutar repositorios desde el dispatcher |
| Prompt reescrito y centralizado | Añadir texto al prompt actual arriesga el límite y duplica políticas | Una advertencia larga por tool |
| Refresh basado en `ToolAction` exitoso | La UI reacciona a receipts confirmados sin conocer use cases | Refrescar por texto del asistente o en todo response |

## Riesgos residuales

- El registro en memoria se pierde al reiniciar; esto cancela confirmaciones y es seguro, aunque obliga a repetir la intención.
- Sin auth propia, `conversation_id` es un bearer efímero dentro del perímetro de autenticación externa asumido; no debe registrarse en logs de aplicación.
- La detección por magic bytes valida contenedor, no decodifica una imagen completa. `LocalPosterStorage` mantiene su validación y errores; una futura decodificación profunda requeriría una decisión separada.
- Los casos de uso de finalización y reapertura delegan en repositorio; cualquier endurecimiento de transición pertenece a ticketing, no a este dispatcher.
- El conteo exacto de tokens depende del tokenizer del modelo configurado y debe ser evidencia obligatoria de verify, además del guard offline.
