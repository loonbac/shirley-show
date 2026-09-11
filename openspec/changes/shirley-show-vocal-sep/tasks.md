# Tasks: Separación vocal conversacional con Ressy (`shirley-show-vocal-sep`)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~930 lines across 4 slices (S1: ~170, S2: ~290, S3: ~230, S4: ~240) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (S1: Dominio y AVG_WAVE) → PR 2 (S2: Infraestructura, Guard, Mutex y Adapter) → PR 3 (S3: Caso de uso y Staging) → PR 4 (S4: Tool, Prompt, Timeout y Frontend) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

---

## Slice 1: Dominio `VocalSeparationMode` y Utilidad DSP `avg_wave` con `hound`

### Contexto y alcance
Crear el tipo cerrado de dominio `VocalSeparationMode` (`Coros`, `Instrumental`), serialización pública JSON (`"chorus"`, `"instrumental"`), y la función pura de procesamiento de audio `avg_wave` basada en `hound`. Esta utilidad combina 3 archivos WAV en lockstep `O(1)` float32 con pesos fijos `1:1:1`, valida paridad estricta de formato/muestras, hace clamp en `[-1.0, 1.0]`, escribe WAV PCM de 24 bits determinista y purga el archivo de salida ante cualquier error.

### Archivos involucrados
- `Cargo.toml`, `Cargo.lock`
- `src/domain/vocal_separation_mode.rs` (nuevo)
- `src/domain/mod.rs`
- `src/audio/avg_wave.rs` (nuevo)
- `src/audio/mod.rs` (nuevo)
- `src/lib.rs`
- `tests/avg_wave_tests.rs` (nuevo)

### Tareas TDD

- [x] RED: Escribir pruebas unitarias en `tests/avg_wave_tests.rs` para `VocalSeparationMode` (parsing JSON `"chorus"`/`"instrumental"`, rechazo de alias/desconocidos) y `avg_wave` con fixtures sintéticos generados en memoria (promedio float de 3 WAVs mono/estéreo a 16-bit/24-bit/32-bit float, clamp ante picos +1.0/-1.0, rechazo de disparidad en sample rate/canales/longitud y purga del archivo de salida ante fallos). <!-- sdd-owner: implementation -->
- [x] GREEN: Agregar dependencia runtime `hound = "3.5"` a `Cargo.toml`, implementar `VocalSeparationMode` en `src/domain/vocal_separation_mode.rs`, y la función de promedio streaming `avg_wave` con errores tipados en `src/audio/avg_wave.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Incorporar pruebas de casos de borde en `tests/avg_wave_tests.rs` (silencio absoluto, WAVs vacíos de cero samples, valores extremos de cuantización a 24 bits, lectura en disco de archivos no existentes y fallo durante escritura con eliminación determinista de residuos). <!-- sdd-owner: implementation -->
- [x] REFACTOR: Pulir el bucle de iteración lockstep para garantizar consumo `O(1)` de memoria sin buffering en RAM de streams de 10 minutos, documentar tipos de error y registrar módulos en `src/lib.rs`. <!-- sdd-owner: implementation -->

### Criterios de aceptación
- `VocalSeparationMode` acepta exclusivamente `"chorus"` e `"instrumental"` en JSON.
- `avg_wave` combina exactamente 3 WAVs con fórmula `(a + b + c) / 3.0` y clamp `[-1.0, 1.0]`.
- Falla de inmediato y limpia la salida si los WAVs difieren en canales, sample rate o cantidad de muestras.
- No hay fugas de rutas ni nombres de temporales en los errores devueltos.
- `cargo test --test avg_wave_tests` pasa en verde.

---

## Slice 2: Infraestructura CLI, `VocalProcessGuard`, Mutex GPU y Adapter `ProcessVocalSeparator`

### Contexto y alcance
Definir el puerto abstracto `VocalSeparator`, implementar el adapter `ProcessVocalSeparator` alrededor de `karaoke-separator`, `ffprobe` y `ffmpeg`. Controlar la exclusión mutua global de GPU mediante `tokio::sync::Semaphore(1)`. Implementar `VocalProcessGuard` para kill-and-wait y limpieza determinista de directorios de trabajo. Implementar resolución de stems (`resolve_stem`) según plantilla `NAME_MODEL_STEM` excluyendo `(?i)vocal`. Garantizar preflight con `ffprobe` para rechazar audios `> 600` segundos antes de pedir el mutex GPU, y preflight de PolarFormer devolviendo degradación amable si el checkpoint no existe. Asegurar codificación final a MP3 CBR 320 kbps.

### Archivos involucrados
- `src/ports/vocal_separator.rs` (nuevo)
- `src/ports/mod.rs`
- `src/infra/vocal/vocal_process_guard.rs` (nuevo)
- `src/infra/vocal/resolve_stem.rs` (nuevo)
- `src/infra/vocal/process_vocal_separator_config.rs` (nuevo)
- `src/infra/vocal/process_vocal_separator.rs` (nuevo)
- `src/infra/vocal/mod.rs` (nuevo)
- `src/infra/mod.rs`
- `src/infra/filesystem/local_audio_storage.rs` (directorio `vocal_work_dir`)
- `src/infra/filesystem/reconcile_audio_files.rs` (purga de directorios huérfanos de trabajo vocal)
- `src/state/app_state.rs` (`gpu_semaphore` e inyección de `VocalSeparator`)
- `tests/process_vocal_separator_tests.rs` (nuevo, pruebas herméticas con stubs ejecutables)

### Tareas TDD

- [ ] RED: Escribir pruebas herméticas en `tests/process_vocal_separator_tests.rs` utilizando binarios stub en directorios temporales (verificación de preflight `ffprobe` con rechazo >600s antes de GPU; ejecución secuencial de `gabox-invertido`, `frazer-becruily` y `anvuew` en Coros; resolución de stems excluyendo `(?i)vocal`; preflight de PolarFormer ausente devolviendo `instrumental_unavailable` sin pedir GPU; transcodificación a MP3 CBR 320 kbps; serialización con Semaphore(1); y cancelación que activa kill-and-wait y purga de workdir reteniendo el permiso hasta su terminación). <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar el trait `VocalSeparator` en `src/ports/vocal_separator.rs`, `VocalProcessGuard` con RAII y drop background en `src/infra/vocal/vocal_process_guard.rs`, `resolve_stem` en `src/infra/vocal/resolve_stem.rs`, `ProcessVocalSeparatorConfig` con resolución `VOCAL_SEP_BIN` en `src/infra/vocal/process_vocal_separator_config.rs`, `ProcessVocalSeparator` en `src/infra/vocal/process_vocal_separator.rs`, y el método `vocal_work_dir` en `LocalAudioStorage` junto con la purga de inicio en `reconcile_audio_files.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Agregar casos de prueba de fallo y contención en `tests/process_vocal_separator_tests.rs` (falla del segundo modelo en la cadena Coros con limpieza completa de salidas previas, error cuando `VOCAL_SEP_BIN` no existe o no es ejecutable, salida con bitrate MP3 distinto a 320 kbps o codec incorrecto, y dos tareas simultáneas donde la segunda espera sin solapar VRAM). <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Estructurar submódulos atómicos bajo `src/infra/vocal/`, asegurar que ningún comando invoque shells (`sh -c`) sino argumentos estructurados, y cablear el `Arc<Semaphore>::new(1)` en `AppState`. <!-- sdd-owner: implementation -->

### Criterios de aceptación
- `ffprobe` valida duración `<= 600` s y stream de audio antes de solicitar el semáforo GPU.
- En `Coros`, se ejecutan únicamente los 3 modelos aprobados, se excluye el stem vocal de cada uno y se transcodifica a MP3 CBR 320 kbps.
- En `Instrumental`, si `model_bs_polarformer_float16.ckpt` o su YAML están ausentes, devuelve `instrumental_unavailable` amablemente sin ejecutar nada en GPU ni caer en fallbacks como Demucs o Coros.
- La salida final es verificada como MP3 CBR 320 kbps válido antes de completar.
- Cancelación, desconexión o timeout matan al subproceso, limpian temporales y no liberan el mutex GPU hasta que el subproceso haya finalizado (`wait`).
- `cargo test --test process_vocal_separator_tests` pasa en verde sin requerir GPU física ni red.

---

## Slice 3: Caso de Uso `SeparateVocals`, Staging y Extensión de `StagedAudioRegistry`

### Contexto y alcance
Implementar el caso de uso `SeparateVocals` que acepta fuentes heterogéneas (`staged_audio_id`, `track_id` o `youtube_url`) con prioridad estricta `staged` → `track` → `youtube`. Integrar con `StagedAudioRegistry` mediante leases RAII (`reserve_guarded`, `StagedAudioLeaseGuard`) y consumo destructivo de fuentes temporales (`commit_processed`), preservando la fuente temporal si la separación falla. El resultado se registra en staging con TTL de 15 minutos (900 segundos) y título `<canción> (Coros)` o `<canción> (Instrumental)` para ser agregado posteriormente como pista canónica mediante `add_track_from_staged`.

### Archivos involucrados
- `src/use_cases/separate_vocals.rs` (nuevo)
- `src/use_cases/mod.rs`
- `src/state/staged_audio_lease_guard.rs` (nuevo)
- `src/state/staged_audio_registry.rs` (métodos `reserve_guarded` y `commit_processed`)
- `src/state/app_state.rs` (composición de `SeparateVocals` y fakes para tests)
- `tests/separate_vocals_tests.rs` (nuevo)
- `tests/staged_audio_registry_tests.rs` (extensiones para lease guards y commit processed)

### Tareas TDD

- [ ] RED: Escribir pruebas unitarias e integración en `tests/separate_vocals_tests.rs` y `tests/staged_audio_registry_tests.rs` con un fake `VocalSeparator` (separación exitosa desde `staged_audio_id`, desde `track_id` administrado sin modificar el archivo original, y desde `youtube_url` reutilizando `DownloadYouTubeAudio`; prioridad de fuentes; verificación de `commit_processed` que elimina el archivo staged fuente y retiene tombstone single-use; verificación de que ante fallo del separator la fuente staged permanece disponible para reintento; registro del MP3 resultante con TTL de 15 minutos; y degradación amable ante PolarFormer ausente). <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `StagedAudioLeaseGuard` en `src/state/staged_audio_lease_guard.rs`, añadir `reserve_guarded` y `commit_processed` a `src/state/staged_audio_registry.rs`, implementar el caso de uso `SeparateVocals` en `src/use_cases/separate_vocals.rs` coordinando staging, resolución de audio y almacenamiento, y registrarlo en `src/state/app_state.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Añadir pruebas de robustez en `tests/separate_vocals_tests.rs` (intento de separar con fuente inexistente o ya consumida, fallo al descargar de YouTube antes de llegar a la separación, fuente que excede el límite de duración, y validación de que el audio resultante puede ser consumido por `UploadTrack`/`add_track_from_staged` para crear una pista canónica). <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Desacoplar la resolución de fuente en funciones privadas puras, validar que `SeparateVocals` siga el principio de 1 struct por caso de uso, y asegurar el aislamiento de las capas de dominio frente a detalles de I/O. <!-- sdd-owner: implementation -->

### Criterios de aceptación
- Resuelve fuentes en orden de prioridad: `staged_audio_id` → `track_id` → `youtube_url`.
- Fuente de tipo `track` se lee en modo sólo lectura y nunca se borra ni se consume.
- Fuente de tipo `staged` se elimina del disco mediante `commit_processed` únicamente tras registrar el resultado exitoso.
- Si el proceso falla, la fuente staged original queda intacta y utilizable dentro de su TTL.
- El resultado se registra en `StagedAudioRegistry` como MP3 CBR 320 kbps válido con TTL de 15 minutos.
- `cargo test --test separate_vocals_tests` y `cargo test --test staged_audio_registry_tests` pasan en verde.

---

## Slice 4: Tool `separate_vocals`, Prompt Ressy, Timeout 600s y Feedback Frontend

### Contexto y alcance
Integrar la capacidad en el agente conversacional: exponer `separate_vocals` en `tool_schemas.rs` (allowlist de 8 herramientas), conectarlo en `tool_dispatcher.rs`, y registrar la política estricta en `system_prompt.rs` y `TurnToolContext`: NUNCA ejecutar `separate_vocals` ni adquirir la GPU sin haber confirmado tanto el modo (`chorus` | `instrumental`) como la presentación de destino. Ampliar la política de deadlines en `agent_chat.rs` y `config.rs` permitiendo hasta 600 segundos para turnos que despachan separación (`SHIRLEY_SEPARATION_TIMEOUT_SECS` / `SHIRLEY_AGENT_TIMEOUT_SECS`). En el frontend, añadir micro-utilidad `is-vocal-separation-intent.js`, actualizar `chat-state.js` para mostrar `separando voces…` en el indicador de escritura durante la espera, preservar `stagedAudioId` de forma efímera en memoria entre turnos (sin `localStorage`), y restaurar el composer y labels en cualquier finalización o cancelación sin streaming ni SSE.

### Archivos involucrados
- `src/agent/tool_schemas.rs` (esquema `separate_vocals`)
- `src/agent/tool_dispatcher.rs` (allowlist de 8 tools, despacho y deduplicación)
- `src/agent/turn_context.rs` (control de `confirmed_destination_id` y `known_track_ids`)
- `src/agent/system_prompt.rs` (reglas conversacionales, pregunto-antes-de-GPU y distinción de 3 etapas)
- `src/config.rs` (timeouts `SHIRLEY_SEPARATION_TIMEOUT_SECS` con ceiling de 600s)
- `src/use_cases/agent_chat.rs` (política de deadline dinámico por tool despachado)
- `src/handlers/agent/chat.rs` (transporte de `staged_audio_id` y deadline ampliado)
- `frontend/js/utils/validation/is-vocal-separation-intent.js` (nuevo)
- `frontend/js/core/state/chat-state.js` (estado `vocalSeparationPending`, pendingLabel `separando voces…`)
- `frontend/js/controllers/agent-chat-controller.js` (indicador cualitativo y restauración del composer)
- `tests/agent_tools_tests.rs`
- `tests/agent_chat_tests.rs`
- `tests/agent_chat_api_tests.rs`
- `tests/agent-chat-state.test.mjs`
- `tests/agent-chat-integration.test.mjs`
- `tests/agent-chat-ui-components.test.mjs`

### Tareas TDD

- [ ] RED: Escribir pruebas unitarias y de integración para backend y frontend (en `tests/agent_tools_tests.rs`, `tests/agent_chat_tests.rs` y `tests/agent-chat-state.test.mjs`) que verifiquen: esquema estricto de `separate_vocals`; rechazo en dispatcher/context si falta destino confirmado; rechazo si el modo es inválido; flujo conversacional en dos turnos donde Shirley pregunta modo y destino antes de ejecutar GPU; encadenamiento en el mismo turno cuando la presentación activa ya existe (`separate_vocals` seguido de `add_track_from_staged`); deduplicación de llamadas idénticas; deadline dinámico de hasta 600 segundos para separación; y en el frontend, activación del label "separando voces…" en el estado del chat al detectar intención vocal, preservación de `stagedAudioId` en memoria y restauración limpia del composer en éxito, error o timeout. <!-- sdd-owner: implementation -->
- [ ] GREEN: Declarar `separate_vocals` en `tool_schemas.rs`, despacharlo en `tool_dispatcher.rs`, proteger la precondición de destino confirmado en `turn_context.rs`, actualizar instrucciones en `system_prompt.rs`, implementar la política de deadline dinámico en `agent_chat.rs` y `config.rs`, e implementar `is-vocal-separation-intent.js`, la gestión de estado en `chat-state.js` y el controlador en `agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Añadir pruebas de integración completas (en `tests/agent_chat_api_tests.rs` e integración frontend) que verifiquen: turno con timeout a los 600s reportando acciones previas y liberando GPU; respuesta amable ante PolarFormer ausente sin agregar pista fantasma; llamada repetida ignorada sin segunda ejecución en GPU; desconexión de cliente abortando la petición; y persistencia en un solo paso hacia el show activo cuando todos los parámetros están presentes. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Revisar que no queden temporizadores genéricos en el controlador pisando el label "separando voces…", asegurar la micro-modularidad de las funciones frontend, verificar accesibilidad con `aria-live="polite"` y comprobar que la suite completa de tests de Rust y Node pasa sin regresiones. <!-- sdd-owner: implementation -->

### Criterios de aceptación
- La allowlist contiene exactamente 8 herramientas locales.
- Ressy NUNCA ejecuta `separate_vocals` ni ocupa la GPU sin confirmación previa de modo y presentación de destino (pregunto-antes-de-GPU).
- Con modo y destino confirmados, encadena `separate_vocals` y `add_track_from_staged` para agregar la pista a la presentación.
- Las confirmaciones distinguen con precisión: "fuente preparada", "separación terminada" y "pista agregada".
- El deadline HTTP escala hasta 600 segundos únicamente para el turno que despacha separación.
- El chat frontend muestra `separando voces…` durante la espera, sin SSE ni porcentajes falsos, y restaura el composer al finalizar.
- `stagedAudioId` viaja entre turnos en memoria efímera y se limpia tras agregar la pista al repertorio.
- `cargo test` y los tests de Node pasan al 100%.
