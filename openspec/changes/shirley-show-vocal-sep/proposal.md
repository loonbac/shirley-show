# Proposal: Separación vocal conversacional con Ressy

**Change:** `shirley-show-vocal-sep`  
**Status:** Proposed  
**Product owner:** Shirley  
**Execution mode:** Auto  
**Artifact store:** OpenSpec  
**Delivery strategy:** Ask on risk  
**Review budget:** 400 changed lines per review unit  
**Planned slices:** S1 dominio + `AVG_WAVE`; S2 adapter de proceso + guard + mutex GPU; S3 caso de uso + staging; S4 tool + prompt + feedback frontend  
**Discovery status:** Pre-proposal confirmado por el orquestador; no quedan decisiones de producto que requieran una nueva entrevista en esta fase

## Intent

Permitir que Shirley le pida a Ressy en lenguaje natural “sepárame esta pista (link)” y reciba una pista MP3 lista para una presentación, sin instalar ni operar manualmente modelos de separación.

Ressy preparará el audio del enlace, preguntará qué resultado se necesita —`COROS` o `INSTRUMENTAL`— y en qué presentación guardarlo, y mantendrá el contexto temporal necesario para completar el flujo en un segundo turno. Durante el trabajo mostrará un estado cualitativo de espera, sin inventar porcentajes ni introducir SSE.

La separación se limita a dos pipelines cerrados. `COROS` ejecuta `gabox-invertido`, `frazer-becruily` y `anvuew` sobre el mismo original y mezcla sus stems karaoke con `AVG_WAVE` en proporción fija `1:1:1`. `INSTRUMENTAL` ejecuta únicamente PolarFormer. Como su checkpoint no está instalado actualmente, esa modalidad debe degradar con un mensaje amable y accionable, sin sustituirla silenciosamente por otro modelo.

## Product problem

Hoy Ressy puede preparar una pista desde un enlace, pero no puede producir una versión adecuada para cantar sobre ella. Shirley debe salir de la aplicación, conocer modelos y comandos especializados, coordinar varios resultados, mezclar archivos y volver a subir el audio. Ese recorrido es lento, propenso a errores y especialmente incómodo desde un teléfono durante la preparación de un show.

La oportunidad es convertir una operación técnica y pesada en una elección de producto comprensible: conservar coros o producir un instrumental. El valor depende de que el resultado sea determinista, que la única GPU no se sature, que un fallo no deje procesos o archivos huérfanos y que Ressy nunca anuncie una pista inexistente.

## Target user and situations

La usuaria principal es Shirley, artista y operadora no técnica de una instancia local o de red confiable de Shirley Show.

Los momentos previstos son:

1. Preparar desde móvil o PC una pista de ensayo o presentación a partir de un enlace compatible.
2. Elegir `COROS` cuando se quiere la mezcla fija de los tres stems karaoke aprobados.
3. Elegir `INSTRUMENTAL` cuando se requiere el pipeline único de PolarFormer.
4. Indicar una presentación activa o seleccionar otra presentación como destino.
5. Esperar una tarea GPU larga con feedback comprensible y recuperarse de configuración ausente, timeout o audio no compatible sin interpretar errores técnicos.

No se diseña para uso durante Stage Mode, procesamiento por lotes ni elección experta de modelos.

## Product outcome

Después de este cambio:

- Shirley puede iniciar la separación desde una petición con enlace y completar modo y destino en una conversación breve.
- El audio fuente se prepara en staging mientras Ressy aclara modo y presentación, evitando una descarga repetida en el segundo turno.
- `COROS` siempre significa la misma ejecución de tres modelos y el mismo promedio `AVG_WAVE 1:1:1`.
- `INSTRUMENTAL` siempre significa PolarFormer y no cae en Demucs, COROS u otro modelo cuando falte el checkpoint.
- Sólo una separación usa la GPU a la vez; el resto espera el mutex dentro del plazo acotado.
- El resultado exitoso es un único MP3 a 320 kbps agregado mediante el flujo canónico de pista al final de la presentación elegida.
- El chat muestra “separando voces…” durante la espera y vuelve a su estado normal en éxito, fallo, timeout o desmontaje, sin SSE ni progreso porcentual.
- Ressy diferencia “fuente preparada”, “separación terminada” y “pista agregada”, y confirma únicamente acciones respaldadas por resultados reales.

## Current-state gap

Las especificaciones canónicas `agent-chat`, `agent-tools` y `show-item` ya definen conversación, herramientas locales acotadas, staging de audio, creación canónica de pistas y feedback sin streaming. El cambio relacionado `shirley-show-yt-download` propone preparar desde YouTube un MP3 temporal identificado por un `staged_audio_id` opaco.

Faltan:

- un modo de dominio cerrado para `COROS` e `INSTRUMENTAL`;
- una operación pura y comprobable de promedio PCM `AVG_WAVE`;
- un puerto mínimo de separación y un adapter seguro para `karaoke-separator`;
- validación previa de duración mediante `ffprobe` con límite de 10 minutos;
- exclusión inequívoca del stem vocal generado por cada modelo;
- serialización global del acceso a la única GPU y liberación segura ante cualquier salida;
- un caso de uso que coordine staging, pipeline, transcodificación final y limpieza;
- una herramienta `separate_vocals` con esquema y errores acotados;
- continuidad estructurada entre los turnos de aclaración y ejecución;
- un deadline HTTP compatible con hasta 600 segundos de separación;
- feedback específico en el chat durante una operación larga.

La especificación canónica `agent-tools` fija actualmente seis herramientas y un deadline global de 30 segundos. Este cambio requiere deltas explícitos: incorporar `separate_vocals` a la allowlist y permitir hasta 600 segundos para el turno que ejecuta separación. Si `shirley-show-yt-download` se integra primero, la allowlist se amplía sobre su baseline y se reutiliza su staging; no se deben crear dos registros temporales incompatibles.

## Product rules and invariants

### Conversación, modo y destino

1. La intención soportada es separar una única pista proveniente de un enlace compatible con el flujo `shirley-show-yt-download`.
2. Ressy debe obtener de la usuaria dos datos explícitos antes de iniciar la GPU: modo (`COROS` o `INSTRUMENTAL`) y presentación de destino.
3. Mientras se aclaran esos datos, el audio fuente puede descargarse y quedar asociado a un identificador temporal opaco; el modelo y el cliente nunca reciben paths reales.
4. Si el destino está implícito en una presentación activa inequívoca, Ressy puede confirmarlo en la pregunta; no debe elegir entre nombres duplicados ni inventar un `presentation_id`.
5. La selección de modo es cerrada. Alias conversacionales pueden normalizarse, pero una intención ambigua debe aclararse antes de ejecutar.
6. La pista fuente preparada no se consume definitivamente hasta que el caso de uso pueda iniciar la separación de forma válida.
7. Las confirmaciones y `actions_taken` deben distinguir descarga preparada, separación producida y pista persistida.

### Pipeline `COROS`

1. El original se procesa exactamente una vez con cada modelo aprobado: `gabox-invertido`, `frazer-becruily` y `anvuew`.
2. Las tres ejecuciones pertenecen a una sola operación protegida por el mutex GPU; ningún otro trabajo GPU puede intercalarse.
3. De cada salida se selecciona el stem karaoke/no vocal y se excluye cualquier archivo cuyo stem coincida de forma case-insensitive con `vocal`.
4. La resolución de outputs ocurre sólo dentro de un directorio temporal dedicado a la operación y reconoce la plantilla `NAME_MODEL_STEM`; no busca resultados en directorios compartidos.
5. `AVG_WAVE` combina las tres señales PCM en coma flotante con pesos fijos `1:1:1`, valida formato compatible y limita la salida al rango representable antes de escribirla.
6. Si falta, sobra o es incompatible cualquiera de los tres stems requeridos, la operación falla completa; no se produce un promedio parcial ni se cambia de modelo.

### Pipeline `INSTRUMENTAL`

1. El único modelo permitido es `polarformer` con `model_bs_polarformer_float16.ckpt` y su YAML correspondiente.
2. La existencia y legibilidad del checkpoint y configuración se validan antes de ocupar la GPU o iniciar el proceso.
3. Mientras esos artefactos estén ausentes, Ressy informa que el modo instrumental aún no está disponible en el servidor y ofrece conservar la fuente dentro de su TTL para elegir `COROS` o reintentar tras la configuración.
4. El sistema no ejecuta Leap, Demucs, Gabox, Becruily, Anvuew, cascades ni otro fallback para simular `INSTRUMENTAL`.

### Límites, GPU y procesos

1. Antes de adquirir la GPU, `ffprobe` debe verificar que la fuente tenga duración válida y no exceda 10 minutos (`600` segundos).
2. Toda invocación externa usa argumentos estructurados; nunca interpola URL, título, paths o parámetros de la usuaria en una cadena de shell.
3. La ruta de `karaoke-separator` se resuelve mediante `VOCAL_SEP_BIN` y una configuración ausente o no ejecutable produce degradación accionable.
4. Una única primitiva compartida de exclusión mutua limita a uno los procesos de separación GPU en toda la aplicación.
5. El permiso GPU se libera de forma determinista en éxito, error, timeout, cancelación, desconexión o panic controlado.
6. El turno que ejecuta separación queda acotado a 600 segundos. Al vencer el plazo se cancelan los procesos pendientes, se espera su terminación y se limpian los temporales.
7. Los turnos que no ejecutan separación conservan su deadline canónico; el plazo ampliado no habilita loops de modelo abiertos ni más iteraciones.
8. La salida final se transcodifica a MP3 CBR 320 kbps mediante `ffmpeg` y se valida antes de registrarse como staging consumible.
9. Los subprocesos pueden liberar VRAM al finalizar, pero esa propiedad no sustituye el mutex global ni la limpieza explícita.

### Staging, persistencia y limpieza

1. Fuente y resultado viven en directorios temporales dedicados y se identifican mediante handles opacos con TTL; nunca se exponen paths al modelo o frontend.
2. El caso de uso `SeparateVocals` consume una fuente vigente, genera exactamente un resultado y registra ese resultado para consumo único por el flujo canónico de alta de pista.
3. El resultado sólo se marca consumido cuando `add_track_from_staged` completa la persistencia canónica en la presentación elegida.
4. La nueva pista conserva título visible no vacío, filename administrado seguro, referencia relativa y posición contigua `N`.
5. Un fallo de destino o persistencia no crea un item parcial; el resultado válido puede seguir disponible mientras no expire su TTL.
6. Fallos de validación, proceso, mezcla o transcodificación eliminan inmediatamente todos los outputs parciales de la operación.
7. La expiración o reinicio invalida los handles en memoria y purga temporales huérfanos sin tocar pistas ya incorporadas al almacenamiento administrado.
8. La fuente y el resultado no pueden consumirse dos veces, y las llamadas duplicadas del modelo mantienen las protecciones canónicas de deduplicación.

## Scope

### Slice 1 — Dominio y `AVG_WAVE`

- Crear `VocalSeparationMode` como tipo cerrado con `Coros` e `Instrumental`.
- Modelar errores de validación y resultados sin detalles de infraestructura.
- Implementar `AVG_WAVE` como módulo puro basado en `hound`, con promedio float `1:1:1`, validación de WAVs compatibles, clamp y escritura determinista.
- Cubrir promedio, silencio, extremos, incompatibilidad de canales/sample rate/longitud y archivos inválidos con fixtures pequeños.
- Forecast inicial: aproximadamente 180 líneas cambiadas.

### Slice 2 — Adapter de proceso, guard y mutex GPU

- Definir el puerto mínimo `VocalSeparator`.
- Implementar `ProcessVocalSeparator` alrededor de `karaoke-separator`, `ffprobe` y `ffmpeg`, con argumentos estructurados y resolución `VOCAL_SEP_BIN`.
- Crear `VocalProcessGuard` para kill-and-wait y limpieza de procesos/temporales en todas las salidas.
- Introducir un mutex/semaphore global de capacidad uno e inyectarlo sin acoplar dominio a Axum.
- Validar la salida por modelo y excluir stems vocales con matching case-insensitive.
- Probar el adapter herméticamente con binarios stub; las pruebas no requieren GPU ni modelos reales.
- Forecast inicial: aproximadamente 280 líneas cambiadas.

### Slice 3 — Caso de uso y staging

- Implementar `SeparateVocals` como caso de uso único que valida duración, disponibilidad de modo, fuente, GPU, pipeline, MP3 320K y cleanup.
- Reutilizar o extender el registro temporal de `shirley-show-yt-download` para fuente y resultado, con TTL y consumo único, sin crear un segundo sistema incompatible.
- Integrar dependencias en `AppState` mediante puertos y componentes concurrentes mínimos.
- Cubrir éxito de ambos pipelines mediante fakes, checkpoint ausente, espera de mutex, timeout, cancelación, expiración, duplicado y fallo de persistencia.
- Forecast inicial: aproximadamente 220 líneas cambiadas.

### Slice 4 — Tool, prompt y feedback frontend

- Añadir `separate_vocals(staged_audio_id, mode)` a la allowlist, schema, dispatcher, deduplicación y reporte de acciones.
- Ajustar el prompt para preguntar modo y destino, conservar handles como estado estructurado y no sustituir pipelines.
- Encadenar el resultado con `add_track_from_staged` cuando la presentación ya esté validada.
- Aplicar el deadline HTTP de 600 segundos sólo al turno de separación y mantener el máximo canónico de cinco iteraciones.
- Mostrar “separando voces…” en el indicador de actividad y restaurar el estado en todos los finales.
- Mantener una sola petición HTTP pendiente, sin SSE, WebSocket, polling ni progreso numérico.
- Forecast inicial: aproximadamente 190 líneas cambiadas.

Las cuatro slices son unidades de revisión previstas por debajo de 400 líneas. Si un forecast actualizado supera el presupuesto, la ejecución debe detenerse en el gate `ask-on-risk` antes de elegir una estrategia de chain o aceptar una excepción.

## Affected areas and implications

### Backend

- Dominio de modo y errores de separación.
- Utilidad DSP `AVG_WAVE` y dependencia `hound`.
- Puerto e infraestructura de `karaoke-separator`, `ffprobe` y `ffmpeg`.
- Ciclo de vida de procesos y directorios temporales.
- Estado concurrente para el mutex GPU.
- Caso de uso, staging TTL y consumo único.
- Allowlist, schemas, dispatcher, prompt, grounding y deadline del agente.
- Integración con la creación canónica de pistas y `AppState`.

### Frontend

- Estado efímero del chat para conservar los handles opacos entre turnos.
- Indicador cualitativo “separando voces…” y recuperación visual ante todos los resultados.
- Mensajes para selección de modo, destino, timeout y PolarFormer no disponible.
- No se modifican reproducción, Stage Mode ni edición de items, salvo la recarga normal tras agregar la pista.

### Operations and support

- El servicio debe resolver `karaoke-separator` mediante `VOCAL_SEP_BIN`, además de disponer de `ffprobe` y `ffmpeg`.
- `gabox-invertido`, `frazer-becruily` y `anvuew` deben estar instalados para `COROS`.
- PolarFormer requiere `model_bs_polarformer_float16.ckpt` y su YAML; la primera entrega debe asumirlo ausente y hacer visible esa disponibilidad sin alarmar.
- Una petición puede mantener conexión y esperar GPU hasta 600 segundos; proxies, servidor y cliente deben compartir un timeout compatible.
- La separación consume GPU, CPU, memoria y disco temporal. La instancia continúa orientada a una carga local/confiable y baja concurrencia.
- Logs y mensajes deben distinguir binario ausente, modelo ausente, fuente demasiado larga, espera/timeout, output inválido y transcodificación fallida sin exponer paths ni stderr crudo.

### Canonical specification deltas

- `agent-tools`: ampliar la allowlist con `separate_vocals`, definir su schema, consumo de staging, deduplicación y excepción de deadline de 600 segundos.
- `agent-chat`: transportar estado temporal opaco entre turnos y definir el feedback cualitativo durante separación sin streaming.
- `show-item`: confirmar que el MP3 separado entra por la creación canónica de `track`, mantiene posición contigua, no hace autoplay y conserva streaming/reproducción existentes.
- `shirley-show-yt-download`: dependencia de integración para obtener y conservar la fuente del enlace; la implementación debe reutilizar su registro o acordar un contrato común antes de aplicar S3.

## Edge cases and failure behavior

| Caso | Comportamiento esperado |
|---|---|
| Falta modo o destino | Ressy pregunta ambos y no ocupa la GPU todavía. |
| Modo desconocido o ambiguo | Pide elegir `COROS` o `INSTRUMENTAL`; no adivina. |
| Fuente temporal ausente, expirada o consumida | No ejecuta modelos; explica que debe preparar de nuevo el enlace. |
| Fuente dura más de 10 minutos | Rechazo previo a la GPU y limpieza de la operación. |
| `ffprobe` no obtiene duración válida | Error accionable; no se asume una duración segura. |
| `VOCAL_SEP_BIN` no existe o no es ejecutable | Mensaje de configuración; sin shell ni fallback. |
| Falta uno de los modelos de `COROS` | Falla el pipeline completo; no mezcla dos resultados ni cambia pesos. |
| Falta checkpoint/YAML de PolarFormer | Indica que `INSTRUMENTAL` no está disponible; no ejecuta otro modelo. |
| El separator produce stem vocal y karaoke | Se usa sólo el karaoke/no vocal y se excluye `(?i)vocal`. |
| Output ambiguo, faltante o extra | No se elige arbitrariamente; falla y limpia. |
| WAVs difieren en sample rate, canales o longitud | `AVG_WAVE` rechaza el conjunto; no trunca ni remuestrea implícitamente. |
| Un modelo falla después de otros éxitos | No queda resultado parcial ni MP3 final. |
| Dos solicitudes llegan juntas | Sólo una usa la GPU; la otra espera el mutex dentro de su deadline. |
| Request alcanza 600 segundos esperando o procesando | Cancela, mata y espera hijos, libera mutex, purga parciales y responde con timeout amable. |
| Cliente desconecta o navega | El backend cancela y limpia; el frontend ignora respuestas tardías y restaura el indicador. |
| `ffmpeg` falla al producir MP3 320K | No registra resultado ni item; conserva diagnóstico saneado. |
| Presentación no existe o es ambigua | No consume el resultado; Ressy pide un destino válido mientras el TTL siga vigente. |
| Alta canónica falla | No queda item parcial y no se anuncia “pista agregada”. |
| Llamada duplicada del modelo | No repite la separación ni agrega dos pistas. |
| Reinicio entre turnos | Los handles se invalidan y se pide repetir; pistas ya persistidas permanecen. |
| Stage Mode está abierto | La separación no controla Stage Mode ni inicia reproducción. |

## Scope boundaries

- El primer slice de producto separa una pista por solicitud y entrega un único MP3.
- El enlace se resuelve mediante el flujo acotado de `shirley-show-yt-download`; este cambio no añade otro descargador ni acceso HTTP genérico.
- Los nombres `COROS` e `INSTRUMENTAL` representan recetas fijas, no categorías configurables.
- El mutex protege toda la única GPU de la aplicación, no sólo un modelo individual.
- El estado temporal facilita dos turnos, pero no constituye una cola persistente ni historial de trabajos.
- Las pistas exitosas entran al modelo `track` existente; no se crea un nuevo tipo de show item.
- Los flujos manuales, archivos adjuntos, streaming, reproducción exclusiva, reordenamiento y Stage Mode deben permanecer sin cambios funcionales.

## Non-goals

- Leap, Demucs o cualquier modelo fuera de los cuatro aprobados.
- Cascades, min/max, medianas, votación, ponderaciones configurables o ensembles externos.
- Elección de modelos, pesos, stems, checkpoints, parámetros DSP o calidad por parte de la usuaria.
- Fallback automático de `INSTRUMENTAL` a `COROS` o viceversa.
- Procesar más de una pista o enlaces por lote.
- Fuentes de más de 10 minutos.
- WAV, FLAC u otra salida final; el producto entrega MP3 320K.
- Progreso porcentual, ETA, SSE, WebSockets, polling o streaming de logs.
- Jobs persistentes, reanudación después de reinicio, prioridad de cola o cancelación manual desde UI.
- Separación local de adjuntos o pistas ya almacenadas como recorrido de producto adicional en esta primera entrega.
- Edición, mastering, normalización, cambio de tono, tempo, corte o reducción de ruido.
- Autoplay, control de Stage Mode o cambios al singleton de reproducción.
- Instalación automática o descarga dinámica del checkpoint de PolarFormer.

## Constraints

### Product and safety

- La experiencia debe explicar las dos modalidades en lenguaje de resultado, no en jerga de modelos.
- Ressy nunca debe afirmar éxito por texto del proceso; necesita archivo validado y tool result correspondiente.
- La falta de PolarFormer es una disponibilidad esperada, no un error fatal de toda la aplicación.
- El usuario no puede suministrar flags, paths, nombres de modelos, pesos ni comandos.
- La aplicación sigue sin login propio y asume protección de red externa; la GPU debe permanecer estrictamente serializada.

### Technical

- Rust + Axum, SQLite, filesystem local y frontend vanilla JS siguen siendo la base.
- Arquitectura micro-modular: modo, promedio, puerto, adapter, guard, mutex, caso de uso, registro, tool y estado UI se separan por responsabilidad.
- `karaoke-separator` se invoca como proceso hijo mediante `VOCAL_SEP_BIN`; `ffprobe` valida duración y `ffmpeg` produce MP3 CBR 320K.
- `hound` implementa `AVG_WAVE` dentro del proyecto; no se añade un ensemble externo.
- La operación completa respeta 600 segundos y 10 minutos máximos de fuente.
- Todas las pruebas de proceso son herméticas mediante stubs; CI no requiere GPU, checkpoints ni inferencia real.
- No se espera migración de SQLite.

### Delivery

- Las cuatro slices se mantienen separadas para proteger el foco de revisión.
- El presupuesto canónico es 400 líneas cambiadas por unidad.
- Bajo `ask-on-risk`, cualquier forecast superior a 400 líneas pausa la ejecución para una decisión humana; no se infiere chain ni `size:exception`.
- S3 depende del contrato de staging de `shirley-show-yt-download`; si ese cambio no está integrado o su contrato difiere, debe resolverse la dependencia antes de implementar, no duplicarla silenciosamente.

## Tradeoffs and decisions

1. **Dos recetas fijas sobre flexibilidad de modelos.** La experiencia es explicable y repetible, a cambio de no permitir ajuste experto.
2. **Promedio `AVG_WAVE 1:1:1` sobre min/max o cascade.** La salida `COROS` aplica exactamente el criterio aprobado y auditable, aunque no se adapte por canción.
3. **PolarFormer sin fallback.** Evita prometer un instrumental con semántica distinta, a cambio de que esa modalidad permanezca no disponible hasta instalar el checkpoint.
4. **Mutex global sobre concurrencia GPU.** Previene OOM y competencia por VRAM, a cambio de espera serial para solicitudes simultáneas.
5. **HTTP síncrono hasta 600 segundos sobre job persistente.** Mantiene simple la primera entrega y el feedback del chat, a cambio de conexiones largas y pérdida de trabajo ante reinicio.
6. **Indicador cualitativo sobre progreso real.** Evita SSE y porcentajes falsos, a cambio de no ofrecer ETA ni detalle por modelo.
7. **Staging opaco entre turnos sobre repetir descarga.** Reduce tiempo y red, a cambio de expiración y estado efímero adicional.
8. **Fallo completo ante outputs incompatibles.** Protege calidad y determinismo, a cambio de no rescatar resultados parciales.
9. **MP3 320K único sobre formatos configurables.** Entrega una pista consistente con el producto, a cambio de transcodificación obligatoria.
10. **Validar duración antes del mutex.** Evita bloquear la GPU con entradas fuera de alcance, a cambio de una llamada previa a `ffprobe`.

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| OOM o contención en GPU | Caída del proceso o separación corrupta | Mutex global de capacidad uno, guard RAII y pruebas de serialización/cancelación. |
| Proceso hijo queda vivo | GPU bloqueada y recursos agotados | `VocalProcessGuard`, timeout, kill-and-wait y limpieza en desconexión. |
| Request largo falla en proxy/cliente | Usuario espera y pierde el resultado | Alinear timeout de 600 segundos en todas las capas y responder con recuperación amable; jobs quedan fuera de esta slice. |
| Checkpoint PolarFormer ausente | `INSTRUMENTAL` no funciona inicialmente | Preflight antes de GPU, disponibilidad explícita y ningún fallback engañoso. |
| Stems mal identificados | Se mezcla voz principal o archivo incorrecto | Temp dir aislado, parser de plantilla, exclusión `(?i)vocal` y cardinalidad exacta. |
| WAVs incompatibles o clipping | Audio inválido o degradado | Validación estricta, mezcla float, clamp y tests de extremos; no truncar/remuestrear implícitamente. |
| Confirmación falsa del agente | Shirley cree que la pista existe | Grounding en etapas y `actions_taken`; separar “preparada”, “separada” y “agregada”. |
| Temporales llenan disco | Degradación operativa | Directorios por operación, cleanup inmediato, TTL, single-use y purga de huérfanos al inicio. |
| Duplicación de registros de staging | Handles incompatibles y leaks | Reutilizar el registro del cambio YouTube mediante un contrato común antes de S3. |
| Deadline ampliado debilita otros turnos | Requests ordinarios retienen recursos | Aplicar 600 segundos sólo a separación y conservar iteraciones/deadlines canónicos fuera de ese flujo. |
| Binarios/modelos cambian outputs | Rotura silenciosa | Adapter aislado, validación final, errores saneados y pruebas contractuales con stubs. |
| Resultado se agrega al show equivocado | Error de repertorio | Resolver destino explícito, integridad de `presentation_id` y aclaración ante nombres duplicados. |
| Slice supera 400 líneas | Revisión pierde foco | Reforecast previo a cada slice y gate `ask-on-risk` antes de exceder el presupuesto. |

## Rollback plan

El rollback es por slice y preserva toda pista que ya se haya incorporado correctamente al almacenamiento canónico.

1. **Desactivar la capacidad:** retirar `separate_vocals` de allowlist y prompt para impedir nuevas operaciones, manteniendo descarga desde enlace, uploads y herramientas canónicas.
2. **Rollback de S4:** retirar schema, dispatcher, prompt, deadline especializado y label “separando voces…”. Invalidar y limpiar resultados de separación aún pendientes antes de retirar su transporte.
3. **Rollback de S3:** detener nuevas separaciones, purgar handles y archivos temporales de fuente/resultado propios del flujo y retirar `SeparateVocals` de `AppState`. No borrar audio ya administrado.
4. **Rollback de S2:** retirar adapter, guard y mutex sólo después de que ningún caso de uso los referencie; verificar que no quedan procesos `karaoke-separator` activos antes del despliegue anterior.
5. **Rollback de S1:** retirar el tipo de modo y `AVG_WAVE` cuando las capas superiores hayan sido desactivadas.
6. **Configuración:** retirar `VOCAL_SEP_BIN` y referencias a modelos del servicio. No borrar manualmente checkpoints compartidos ni modificar el entorno de usuario como parte del rollback de aplicación.
7. **Recuperación operativa:** detener el servicio, esperar o terminar de forma controlada cualquier hijo, desplegar binario/assets conocidos, conservar SQLite y audio administrado, limpiar sólo el directorio temporal dedicado y reiniciar.
8. **Verificación posterior:** abrir catálogo y presentación, reproducir una pista separada ya persistida, probar upload y descarga existentes y comprobar que no quedan handles, temporales ni procesos GPU huérfanos.
9. **Compatibilidad:** no se espera migración de base de datos. Si la implementación descubre que necesita una, debe detenerse y añadir un plan forward/down probado antes de aplicar.

## Success criteria

1. Una usuaria puede pedir “sepárame esta pista (link)”, elegir modo y destino, y completar el flujo sin operar herramientas externas.
2. La fuente se descarga una sola vez y permanece disponible mediante un handle opaco durante la aclaración de modo y destino.
3. `COROS` ejecuta exactamente `gabox-invertido`, `frazer-becruily` y `anvuew`, toma un stem karaoke válido de cada uno y aplica `AVG_WAVE 1:1:1`.
4. `INSTRUMENTAL` ejecuta únicamente PolarFormer cuando checkpoint y YAML están disponibles.
5. Con el checkpoint ausente, `INSTRUMENTAL` devuelve un mensaje amable y accionable, no ocupa la GPU y no ejecuta ningún fallback.
6. Una fuente de más de 10 minutos o con duración no verificable se rechaza antes de adquirir el mutex GPU.
7. Como máximo una separación usa la GPU; éxito, error, timeout y cancelación siempre liberan el permiso.
8. Toda operación termina o se cancela de forma controlada dentro de 600 segundos, sin hijos ni temporales huérfanos.
9. El resultado final válido es un único MP3 CBR 320K y se agrega exactamente una vez como pista canónica al final de la presentación elegida.
10. Outputs faltantes, ambiguos, vocales o incompatibles no producen promedio parcial, staging válido ni show item.
11. Ressy sólo confirma estados respaldados por tools y no expone paths, comandos, stderr crudo ni detalles sensibles de infraestructura.
12. El frontend muestra “separando voces…” durante la petición y restaura su estado en todos los finales, sin SSE, WebSocket, polling, porcentaje ni ETA.
13. Las pruebas de dominio, mezcla, procesos, mutex, timeout, staging y tool son herméticas y no necesitan GPU ni modelos reales.
14. Los flujos canónicos de chat, adjuntos, descarga desde enlace, creación manual de pistas, streaming, reproducción, orden y Stage Mode no regresan.
15. Cada slice permanece dentro de 400 líneas cambiadas o la ejecución se detiene en el gate `ask-on-risk` antes de superar el presupuesto.
