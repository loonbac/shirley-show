# Proposal: Descarga de audio de YouTube con Ressy

**Change:** `shirley-show-yt-download`  
**Status:** Proposed  
**Product owner:** Shirley  
**Execution mode:** Auto  
**Artifact store:** OpenSpec  
**Delivery strategy:** Ask on risk  
**Review budget:** 400 changed lines per review unit  
**Planned slices:** S1 dominio/infra; S2 caso de uso/registro; S3 herramientas/agente; S4 chat/progreso  
**Discovery status:** Pre-proposal confirmado por el orquestador; no quedan decisiones de producto que requieran una nueva entrevista en esta fase

## Intent

Permitir que Shirley, una usuaria no técnica, le pida a Ressy en lenguaje natural que convierta un único video público de YouTube en una pista de audio MP3 lista para su repertorio. Una petición como “descarga este video `youtube.com/...` en pista” debe iniciar una descarga acotada, mostrar un estado de trabajo comprensible, confirmar únicamente un resultado real y preguntar a qué presentación agregarlo cuando ese destino no sea conocido.

Cuando la conversación ya tenga una presentación inequívoca, Ressy podrá descargar y agregar la pista en el mismo turno. Cuando no la tenga, conservará temporalmente el audio durante 15 minutos para completar el flujo en un segundo turno sin pedir a la usuaria que descargue y vuelva a subir el archivo manualmente.

## Product problem

Hoy Ressy puede agregar un archivo de audio que la usuaria adjunta, pero no puede resolver una URL de YouTube. Shirley debe salir de la aplicación, usar herramientas externas, entender formatos, localizar el archivo convertido y subirlo de nuevo. Ese recorrido técnico interrumpe la preparación del show, resulta especialmente incómodo desde el teléfono y expone a errores de formato, tamaño y asociación con la presentación equivocada.

La oportunidad es convertir esa tarea multiherramienta en una intención conversacional sencilla sin convertir al agente en un descargador general, darle acceso arbitrario a red o shell, ni debilitar las reglas canónicas de creación de pistas.

## Target user and situations

La usuaria principal es Shirley, artista y operadora no técnica de una instancia local o de red confiable de Shirley Show.

Los momentos de uso previstos son:

1. Preparando un repertorio desde teléfono o PC y recibiendo una URL de YouTube que debe convertirse en pista.
2. Conversando dentro de una presentación ya abierta, donde el destino está disponible como `presentation_id` activo.
3. Conversando sin presentación activa, donde primero conviene preparar el audio y después elegir o aclarar la presentación.
4. Recuperándose de una URL inválida, un video demasiado largo, un archivo demasiado grande, una descarga lenta o una herramienta del servidor ausente sin necesitar interpretar errores técnicos.

No se diseña para operación autónoma, descargas masivas ni control durante Stage Mode.

## Product outcome

Después de este cambio:

- Shirley puede pegar una URL compatible y pedir una pista sin manejar `yt-dlp`, `ffmpeg`, formatos ni archivos temporales.
- Ressy cambia su indicador cualitativo a “descargando el audio…” mientras procesa una petición con URL de YouTube; no muestra un porcentaje ficticio.
- La confirmación “lista” aparece sólo cuando existe un MP3 validado y registrado temporalmente, o cuando además se creó correctamente la pista canónica.
- Si hay una presentación activa o inequívocamente resuelta, Ressy encadena descarga y alta de pista en el mismo turno.
- Si falta el destino, Ressy pregunta a qué presentación agregarla y mantiene una referencia temporal opaca durante 15 minutos.
- La pista finalmente agregada se comporta como cualquier pista existente: ocupa la siguiente posición contigua, se reproduce mediante el streaming canónico y se administra desde las vistas actuales.
- Los fallos dejan mensajes accionables, no crean elementos parciales y no exponen rutas, comandos ni detalles internos.

## Current-state gap

El sistema canónico ya dispone de chat, ejecución de herramientas locales acotadas, staging de archivos adjuntos y `add_track_from_staged`, pero ese staging está ligado a un archivo subido y al ciclo de vida de una única solicitud.

Faltan:

- un tipo de dominio que acepte sólo URLs compatibles de YouTube y obtenga un identificador de video válido;
- un puerto de descarga y una implementación segura basada en procesos;
- un caso de uso que descargue, convierta, valide y registre audio temporal;
- continuidad explícita entre turnos mediante un `staged_audio_id` opaco y con expiración;
- una herramienta permitida para iniciar la descarga;
- capacidad de `add_track_from_staged` para consumir la referencia temporal descargada además del adjunto actual;
- un plazo global compatible con una descarga acotada;
- feedback de chat específico para que la usuaria sepa que el audio se está preparando.

La especificación canónica `agent-tools` fija actualmente seis herramientas y un deadline global de 30 segundos, por lo que esta propuesta requiere deltas explícitos: una séptima herramienta permitida y un deadline global de 120 segundos. La semántica request-scoped de adjuntos existente se conserva; el registro TTL se añade como una segunda fuente de staging, no como relajación de la limpieza de uploads.

## Product rules and invariants

### Solicitud y URL

1. Sólo se acepta una URL cuyo host sea `youtube.com` o `youtu.be` en una forma soportada y de la que se pueda obtener un identificador de video de exactamente 11 caracteres.
2. La URL se normaliza y valida antes de iniciar cualquier proceso o conexión de descarga.
3. Cada solicitud procesa un solo video y fuerza `--no-playlist`, incluso si la URL contiene contexto de lista.
4. No se aceptan directos, contenido que requiera cookies o autenticación, ni URLs de otras plataformas.
5. Un título opcional proporcionado por la usuaria se recorta y valida. Si falta, el resultado puede proponer un título no vacío derivado de metadatos seguros, con el identificador de video como respaldo.

### Descarga y conversión

1. El resultado permitido es únicamente audio MP3.
2. La duración máxima del contenido es 20 minutos (`1200` segundos).
3. El tamaño máximo permitido es 100 MiB (`104,857,600` bytes); un resultado que exceda el límite no puede registrarse ni convertirse en pista.
4. La implementación invoca `yt-dlp` mediante argumentos estructurados de `tokio::process`; nunca construye ni ejecuta una cadena de shell.
5. La invocación fuerza extracción MP3, `--no-playlist` y `--max-filesize 100M`, y depende de `ffmpeg` para la conversión.
6. El subproceso dispone de hasta 90 segundos. Al vencer ese plazo se termina, se espera su cierre y se purgan los archivos parciales.
7. El ciclo completo del agente dispone de hasta 120 segundos, incluyendo proveedor, herramientas y respuesta controlada.
8. Una descarga sólo se considera exitosa después de comprobar el archivo final, su formato esperado y sus límites; la salida textual del proceso por sí sola no prueba éxito.

### Staging temporal y consumo

1. `download_youtube_track(url, title?)` devuelve una referencia `staged_audio_id` opaca, metadatos seguros para la conversación y ningún path del sistema.
2. `StagedAudioRegistry` conserva en memoria la asociación entre esa referencia y el archivo temporal durante 15 minutos.
3. La referencia es de un solo consumo. Tras un `add_track_from_staged` exitoso no puede crear una segunda pista.
4. Una referencia inexistente, expirada o ya consumida produce un error estructurado y ninguna mutación.
5. Un archivo descargado que falla validación, cuyo proceso falla o cuyo subproceso vence se elimina inmediatamente.
6. Las entradas expiradas se invalidan y sus archivos se purgan; reiniciar el servidor invalida referencias en memoria y debe limpiar temporales huérfanos de este flujo sin tocar audio administrado.
7. Los adjuntos multipart existentes mantienen su guard request-scoped y su limpieza actual. No se prolonga automáticamente la vida de un adjunto cargado por la usuaria.
8. El cliente conserva y reenvía sólo el identificador temporal necesario para el siguiente turno; no interpreta ni fabrica paths o handles.

### Destino y persistencia

1. Si existe una presentación activa o una presentación resuelta inequívocamente durante el turno, el agente puede encadenar `download_youtube_track` con `add_track_from_staged`.
2. Si no existe un destino inequívoco, Ressy confirma que el audio está preparado y pregunta a qué presentación agregarlo.
3. Si varias presentaciones comparten el nombre indicado, Ressy debe listar o pedir aclaración; no elige arbitrariamente.
4. La integridad de `presentation_id` sigue las reglas canónicas del agente y del dominio.
5. Crear la pista usa el flujo canónico: nombre visible no vacío, filename administrado seguro, registro `track`, referencia relativa y posición contigua al final.
6. Un fallo al agregar no debe marcar como consumido el staging antes de que la operación canónica pueda completarse; la usuaria puede corregir el destino mientras la referencia siga vigente.
7. Una vez creada, la pista deja de depender del registro temporal y no se elimina cuando expira el TTL.

### Conversación y confianza

1. El mensaje final y `actions_taken` se basan únicamente en resultados reales de herramientas.
2. “Audio listo” significa descargado, convertido, validado y disponible en staging; “pista agregada” significa persistida en una presentación.
3. `staged_audio_id` debe viajar como estado estructurado del chat y no depender de que el modelo lo copie correctamente desde texto libre.
4. El estado de progreso es cualitativo. Esta entrega no promete bytes, porcentaje ni tiempo restante en tiempo real.
5. Los errores son cálidos y accionables, y no muestran variables, rutas absolutas, stderr sin filtrar, comandos ni detalles del proceso.
6. Los límites de iteraciones, prevención de llamadas duplicadas, degradación de Groq y grounding canónicos siguen vigentes salvo el cambio explícito del deadline global.

## Scope

### Slice 1 — Dominio, puerto e infraestructura de descarga

- Crear `YouTubeUrl` como valor de dominio validado y normalizado.
- Definir el puerto mínimo `YoutubeDownloader`.
- Implementar `ProcessYoutubeDownloader` con `tokio::process`, argumentos estructurados, resolución configurable de binario, timeout, terminación y purga.
- Cubrir la integración herméticamente con un script stub de `yt-dlp`; las pruebas no dependen de YouTube ni de la red.
- Forecast inicial: aproximadamente 250 líneas cambiadas.

### Slice 2 — Caso de uso y registro temporal

- Crear `DownloadYouTubeAudio` como caso de uso único que coordina validación, descarga y registro.
- Crear `StagedAudioRegistry` en memoria con TTL de 15 minutos, consumo único y limpieza.
- Inyectar las nuevas dependencias a través de `AppState` sin acoplar el dominio a Axum o al proveedor de IA.
- Cubrir éxito, expiración, consumo, limpieza y fallos parciales.
- Forecast inicial: aproximadamente 220 líneas cambiadas.

### Slice 3 — Herramientas y orquestación del agente

- Añadir el esquema y despacho de `download_youtube_track(url, title?)` a la allowlist.
- Ampliar la fuente aceptada por `add_track_from_staged` para consumir de forma segura un staging remoto vigente sin romper adjuntos request-scoped.
- Ajustar prompt, grounding, deduplicación y reporte de acciones para distinguir “preparado” de “agregado”.
- Encadenar descarga y alta cuando el destino sea conocido, o formular la pregunta de destino cuando no lo sea.
- Probar ambos recorridos y los errores estructurados sin proveedor ni red reales.
- Forecast inicial: aproximadamente 230 líneas cambiadas.

### Slice 4 — Deadline, continuidad y feedback de chat

- Elevar el deadline global configurable a 120 segundos mediante `SHIRLEY_AGENT_TIMEOUT_SECS`.
- Transportar `staged_audio_id` de forma estructurada entre respuesta, estado efímero del frontend y siguiente solicitud.
- Cambiar el indicador de escritura a “descargando el audio…” al detectar una URL de YouTube en la petición.
- Conservar el indicador genérico para otros mensajes y restaurarlo en éxito, error, timeout o desmontaje.
- Mantener la implementación sin SSE, WebSocket ni polling de progreso.
- Forecast inicial: aproximadamente 180 líneas cambiadas.

Cada slice es una unidad de revisión prevista por debajo de 400 líneas. Si el forecast actualizado de una unidad supera el presupuesto, la ejecución debe detenerse en el gate `ask-on-risk` antes de elegir encadenamiento o aceptar una excepción.

## Affected areas and implications

### Backend

- Dominio de URL y errores de descarga.
- Puerto de infraestructura e implementación de proceso externo.
- Gestión de directorios temporales y ciclo de vida de archivos.
- Caso de uso de descarga y `AppState`.
- Registro concurrente en memoria con expiración y consumo único.
- Allowlist, esquemas, dispatcher, prompt y reporte de acciones del agente.
- Contrato de chat para transportar staging temporal.
- Configuración del deadline global y resolución de `YT_DLP_BIN`.

### Frontend

- Cliente de chat y su estado efímero para conservar `staged_audio_id`.
- Indicador de actividad contextual basado en intención/URL.
- Mensajes de confirmación, expiración y recuperación.
- No se modifican los reproductores, Stage Mode ni los componentes canónicos de pista salvo la recarga normal tras una alta exitosa.

### Operations and support

- El servicio systemd debe poder resolver `yt-dlp` mediante `YT_DLP_BIN`, ya que el binario instalado en `~/.nix-profile/bin` puede no estar en su `PATH`.
- `ffmpeg` debe estar disponible para `yt-dlp`; una ausencia se comunica como configuración faltante, no como fallo genérico.
- Los logs deben permitir distinguir validación, timeout, salida no válida y binario ausente, siempre con URL sensible, paths y salida de proceso saneados cuando corresponda.
- Las descargas consumen red, CPU, disco temporal y hasta 120 segundos de capacidad del request. La instancia sigue orientada a uso local/confiable y de baja concurrencia.
- El soporte deberá explicar que el audio preparado caduca a los 15 minutos y que la usuaria puede volver a solicitar la descarga si expiró.

## Edge cases and failure behavior

| Caso | Comportamiento esperado |
|---|---|
| Mensaje sin URL compatible | No se activa la herramienta de YouTube ni el label de descarga. |
| URL malformada, host parecido o ID inválido | Rechazo previo al proceso, con solicitud de una URL válida de YouTube. |
| URL contiene playlist junto a un video | Se procesa como máximo el video individual; nunca la lista. |
| URL representa un live | Rechazo sin crear staging ni pista. |
| Video supera 20 minutos | Cancelación/rechazo y purga de temporales. |
| Descarga o MP3 supera 100 MiB | Cancelación/rechazo, sin registro temporal ni item. |
| `YT_DLP_BIN` no existe o no es ejecutable | Mensaje de configuración accionable; no se intenta shell ni fallback inseguro. |
| `ffmpeg` no está disponible | Error amable de conversión/configuración y limpieza completa. |
| `yt-dlp` devuelve fallo o salida inesperada | No se anuncia éxito; se sanea el diagnóstico y se eliminan parciales. |
| Subproceso supera 90 segundos | Se mata y espera el proceso, se purgan parciales y se devuelve timeout controlado. |
| Request global alcanza 120 segundos | Se cancelan operaciones pendientes, se conservan sólo acciones ya confirmadas y se limpia staging no válido. |
| Cliente se desconecta durante descarga | La cancelación no deja un proceso desatendido ni un parcial permanente. |
| Título opcional vacío | Se usa un título seguro derivado o el ID; nunca se crea una pista sin nombre visible. |
| No hay presentación activa | El audio queda preparado y Ressy pregunta el destino. |
| Presentación indicada no existe | No se consume la referencia; se informa y puede elegirse otra dentro del TTL. |
| Nombre de presentación ambiguo | Ressy pide aclaración y no agrega la pista. |
| `staged_audio_id` expiró | Se informa la caducidad y se ofrece repetir la descarga; no se crea item. |
| ID desconocido, manipulado o ya consumido | Error estructurado, sin acceso a paths y sin duplicar pistas. |
| Llamada duplicada del modelo | La deduplicación del turno evita una segunda descarga o una segunda alta idéntica. |
| Reinicio del servidor entre turnos | La referencia en memoria deja de ser válida; se purga el temporal huérfano y se pide repetir. |
| Alta canónica falla tras descarga | No queda item parcial; el staging sólo sigue disponible si permanece íntegro y dentro del TTL. |
| Descarga exitosa sin segundo turno | El archivo se elimina al expirar 15 minutos. |
| URL de YouTube aparece como texto informativo, no como orden | El label puede ser cualitativo, pero el agente sólo ejecuta la herramienta cuando la intención solicita descarga. |

## Scope boundaries

- El cambio amplía exclusivamente el flujo conversacional de creación de pistas.
- La descarga termina en el mismo modelo canónico de audio y show item; no se crea una biblioteca paralela de medios.
- El registro temporal es efímero, local y no sustituye base de datos, cola de trabajos ni historial persistente.
- La UI manual de subida continúa disponible y conserva su contrato actual.
- El contenido preparado no se reproduce automáticamente y no altera las reglas de reproducción exclusiva.
- Los cambios a las especificaciones canónicas deben ser deltas mínimos para `agent-chat`, `agent-tools` y, sólo si resulta necesario para reflejar la nueva procedencia de una pista, `show-item`.

## Non-goals

- Playlists o descarga por lotes.
- Descargar o almacenar video.
- Livestreams, premieres activas o captura en directo.
- Vimeo, TikTok, SoundCloud u otras plataformas.
- Cookies, login, contenido privado, age-gated que requiera sesión o evasión de restricciones.
- Selección avanzada de bitrate, codec, normalización, recorte, mezcla o edición de audio.
- Progreso porcentual, ETA, streaming de eventos, SSE, WebSockets o trabajos en background.
- Cola persistente, reanudación tras reinicio o historial de descargas.
- Búsqueda de YouTube por palabras; la usuaria debe aportar una URL.
- Reproducción automática, control de Stage Mode o cambios al controlador de audio.
- Nuevo modelo de permisos, autenticación, cuotas por usuaria o garantías sobre derechos de uso del contenido.
- Sustituir el flujo manual de upload o modificar los formatos que éste ya acepta.

## Constraints

### Product and safety

- La experiencia debe ser comprensible para una usuaria no técnica y diferenciar preparación temporal de persistencia definitiva.
- El sistema no debe afirmar que el contenido puede descargarse legalmente; la usuaria sigue siendo responsable de contar con los derechos y permisos aplicables.
- No se incorporan mecanismos para eludir autenticación, restricciones regionales, controles del proveedor o protecciones del contenido.
- La aplicación continúa sin login propio y asume protección de red externa; por ello el alcance de red y los recursos del proceso deben permanecer estrictamente acotados.

### Technical

- Rust + Axum y las operaciones canónicas actuales siguen siendo la base.
- Arquitectura micro-modular: valor de dominio, puerto, adapter, caso de uso, registro, herramientas y UI se separan por responsabilidad.
- `yt-dlp` y `ffmpeg` están disponibles en el entorno objetivo, pero la ruta de `yt-dlp` debe poder configurarse con `YT_DLP_BIN` y ofrecer un fallback explícito y verificable.
- No se ejecuta shell ni se aceptan flags suministrados por la usuaria.
- Timeout de proceso: 90 segundos. Deadline global del agente: 120 segundos, configurable con `SHIRLEY_AGENT_TIMEOUT_SECS`.
- Duración máxima: 20 minutos. Tamaño máximo: 100 MiB. Salida: MP3. TTL: 15 minutos.
- Pruebas estrictamente herméticas para el adapter de proceso mediante stub local; CI y desarrollo no deben descargar desde YouTube.
- No se espera migración de SQLite.

### Delivery

- Las cuatro slices se mantienen separadas para no mezclar infraestructura, estado temporal, decisiones del agente y feedback del frontend.
- El presupuesto canónico es 400 líneas cambiadas por unidad de revisión.
- `ask-on-risk` obliga a detenerse antes de implementar una unidad cuyo forecast actualizado exceda 400 líneas; no se infiere estrategia de chain ni excepción.

## Tradeoffs and decisions

1. **Proceso síncrono acotado en vez de job persistente.** Simplifica el primer flujo y permite una conversación directa, a cambio de ocupar un request durante más tiempo y perder continuidad tras reinicio.
2. **Registro en memoria con TTL en vez de persistencia.** Evita migraciones y una biblioteca temporal permanente, a cambio de que una referencia caduque o se pierda al reiniciar.
3. **Indicador cualitativo en vez de progreso porcentual.** Entrega feedback inmediato sin SSE ni parseo frágil de progreso, a cambio de no mostrar porcentaje o ETA.
4. **Allowlist ampliada en una herramienta específica en vez de HTTP arbitrario.** Limita el riesgo y mantiene auditable la intención, a cambio de soportar sólo YouTube y formas de URL explícitas.
5. **MP3 único en vez de conservar formatos originales.** Produce pistas reproducibles y predecibles, a cambio de recodificación y posible pérdida de calidad.
6. **Encadenamiento automático sólo con destino inequívoco.** Reduce pasos cuando hay contexto confiable, pero prioriza una pregunta adicional sobre el riesgo de agregar a la presentación incorrecta.
7. **Límite de 20 minutos y 100 MiB.** Protege tiempo y disco de una aplicación local, aunque excluye sets largos y archivos de alta calidad.
8. **`staged_audio_id` estructurado en vez de memoria textual del modelo.** Hace confiable el segundo turno, a cambio de ampliar de forma mínima el contrato del chat y el estado efímero del cliente.
9. **TTL como control temporal, no cuota de concurrencia.** Es suficiente para el uso local esperado en esta slice, pero no elimina el riesgo de presión de disco por muchas solicitudes simultáneas; una cuota explícita queda fuera hasta contar con una necesidad real.

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Descarga de contenido no autorizado | Riesgo legal o de términos de servicio | Alcance a URL aportada por la usuaria, sin bypass/cookies; comunicar responsabilidad de derechos y no prometer disponibilidad universal. |
| URL hostil o escape hacia red arbitraria | SSRF o uso del servidor como downloader | Valor de dominio estricto, host e ID permitidos, URL normalizada y ningún tool HTTP genérico. |
| Inyección de comandos por URL o título | Ejecución remota | `tokio::process` con argumentos estructurados, sin shell ni flags de usuario. |
| Proceso colgado o hijo huérfano | Agotamiento de recursos | Timeout de 90 segundos, kill-and-wait en timeout/cancelación y pruebas del lifecycle. |
| Temporales huérfanos | Llenado de disco | Directorio dedicado, guardas, purga en cada fallo, expiración de 15 minutos y limpieza de huérfanos al inicio. |
| Varias descargas concurrentes llenan disco | Interrupción operativa | Límite individual de 100 MiB, TTL corto, observabilidad y uso local; si aparece presión real, proponer aparte un límite concurrente/capacidad. |
| `staged_audio_id` filtrado o manipulado | Consumo de audio equivocado | Identificador aleatorio opaco, lookup server-side, no paths, single-use, TTL y contexto de chat estructurado. |
| Falsa confirmación del agente | Shirley cree tener una pista inexistente | Separar estados “preparado” y “agregado”; grounding obligatorio en resultados de herramientas. |
| Timeout global aumentado afecta capacidad | Requests más largos compiten por recursos | Subtimeout de proceso, límites de iteración existentes y descarga acotada; no background infinito. |
| `yt-dlp` cambia su salida o deja de funcionar | Fallos tras actualización del binario | Adapter aislado, validación del archivo final, errores saneados y pruebas con contrato de stub. |
| Diferencia entre PATH interactivo y systemd | Funciona en terminal pero no en producción | `YT_DLP_BIN`, fallback verificable y diagnóstico amigable de binario ausente. |
| Regresión en adjuntos existentes | Uploads dejan de limpiarse o consumirse bien | Mantener staging request-scoped como fuente separada y cubrir ambas fuentes en tests. |
| Slice supera 400 líneas | Revisión pierde foco | Reforecast antes de cada slice y gate `ask-on-risk` previo a exceder presupuesto. |

## Rollback plan

El rollback es por slice y preserva toda pista ya incorporada correctamente al almacenamiento canónico.

1. **Desactivar el flujo conversacional:** retirar de la allowlist y del prompt `download_youtube_track`; Ressy deja de ofrecer la descarga, mientras los uploads y herramientas canónicas siguen funcionando.
2. **Rollback de Slice 4:** restaurar el deadline previo y retirar el transporte frontend de `staged_audio_id` y el label especializado. Antes de hacerlo, invalidar y purgar staging remoto pendiente.
3. **Rollback de Slice 3:** retirar schema, dispatcher y encadenamiento de YouTube; devolver `add_track_from_staged` a su semántica exclusiva de adjunto request-scoped.
4. **Rollback de Slice 2:** detener aceptación de nuevas descargas, purgar entradas y archivos del registro TTL y retirar sus dependencias de `AppState`. Nunca borrar audio ya movido al directorio administrado.
5. **Rollback de Slice 1:** retirar adapter, puerto y dominio sólo después de que ninguna ruta o caso de uso los referencie.
6. **Configuración:** eliminar `YT_DLP_BIN` y, si ya no se necesita el plazo ampliado, `SHIRLEY_AGENT_TIMEOUT_SECS` de la unidad systemd. No modificar `~/.nix-profile` como parte del rollback de aplicación.
7. **Recuperación operativa:** detener el servicio, desplegar el binario y assets conocidos anteriores, conservar SQLite y el directorio de audio administrado, limpiar únicamente el directorio temporal dedicado y reiniciar.
8. **Verificación posterior:** abrir catálogo y presentación, subir manualmente una pista, reproducir una pista previamente creada por YouTube y comprobar que no quedan procesos `yt-dlp` ni archivos temporales huérfanos.
9. **Compatibilidad:** no se espera migración de base de datos. Si una implementación descubre que la necesita, debe detenerse y añadir un plan de migración/down separado antes de aplicar.

## Success criteria

1. Una usuaria puede pedir en lenguaje natural la descarga de una URL válida de YouTube y obtener un MP3 preparado sin usar herramientas externas manualmente.
2. Con una presentación activa válida, Ressy puede descargar y crear exactamente una pista canónica en el mismo turno, al final de la secuencia contigua.
3. Sin presentación activa, Ressy confirma el audio preparado, pregunta el destino y puede completar el alta en un segundo turno dentro de 15 minutos.
4. La respuesta diferencia de forma verificable entre audio temporal preparado y pista persistida; nunca informa éxito sin tool result correspondiente.
5. URLs no permitidas, playlists sin video individual, lives, contenido de más de 20 minutos, resultados de más de 100 MiB y contenido que requiera cookies no crean staging válido ni show item.
6. El resultado final aceptado es MP3, tiene nombre visible no vacío y usa un filename administrado seguro sin exponer paths al modelo o al cliente.
7. Timeout, cancelación, error de proceso, error de conversión y fallo de persistencia no dejan registros parciales, procesos hijos ni archivos temporales permanentes.
8. Una referencia expirada, desconocida o consumida no puede crear una pista; las expiradas se limpian y las consumidas no se reutilizan.
9. La ausencia o mala configuración de `yt-dlp`/`ffmpeg` produce un mensaje accionable y saneado, mientras el resto de Shirley Show sigue operativo.
10. El agente completa o cancela el request dentro del deadline global de 120 segundos, con un subtimeout de descarga de 90 segundos y sus límites de iteración/deduplicación intactos.
11. El chat muestra “descargando el audio…” durante la operación y restaura su estado en todos los finales, sin afirmar porcentaje ni requerir SSE.
12. Las pruebas del adapter y del caso de uso son herméticas, usan un stub de `yt-dlp` y no realizan descargas reales.
13. Los flujos canónicos de archivo adjunto, creación manual de pista, streaming, reproducción exclusiva, orden de items y Stage Mode no regresan.
14. Cada slice permanece dentro de 400 líneas cambiadas o la ejecución se detiene en el gate `ask-on-risk` antes de superar el presupuesto.
