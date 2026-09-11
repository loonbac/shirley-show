# Progreso de aplicación: `shirley-show-yt-download`

**Fecha:** 2026-09-11  
**Slice activo:** Slice 4 (`yt-s4-timeout-frontend`) — ¡Último slice completado!  
**Estrategia de entrega:** 4 PRs encadenados stacked-to-main (PR 1 = Slice 1, PR 2 = Slice 2, PR 3 = Slice 3, PR 4 = Slice 4)  
**Attempt authority:**
- Token: `sha256:40914e4af5ccd4e1e55dcf2cb87e731a51d76ba70bc7a401417594dda489803e`
- Work-unit: `"yt-s4-timeout-frontend"`
- Request: `y4-acquire-001`
- Max budget: 3000 líneas (Reportado para settle, NO settle ejecutado)

---

## Tareas completadas en Slice 1

- [x] 1.1 RED: Escribir pruebas unitarias para `YouTubeUrl` en `tests/youtube_url_tests.rs` validando formatos compatibles (`watch?v=`, `youtu.be/`, `/shorts/`), normalización ignorando `&list=` y otros query params, y rechazo estricto de hosts externos, IDs inválidos y metacaracteres de shell (`;`, `|`, `$`, backtick, comillas). <!-- sdd-owner: implementation -->
- [x] 1.2 GREEN: Implementar el tipo de dominio `YouTubeUrl` y sus errores en `src/domain/youtube_url.rs`, registrar el módulo en `src/domain/mod.rs` y añadir la dependencia `url` en `Cargo.toml`. <!-- sdd-owner: implementation -->
- [x] 1.3 RED: Escribir pruebas herméticas de integración para `ProcessYoutubeDownloader` en `tests/youtube_downloader_tests.rs` utilizando scripts stub de prueba para `yt-dlp`, `ffmpeg` y `ffprobe` en directorio temporal aislado; asertar el orden y contenido exacto de los argumentos estructurados (`--no-config`, `--no-playlist`, `--extract-audio`, `--audio-format`, `mp3`, `--audio-quality`, `320K`, `--max-filesize`, `100M`, `--match-filter`, `!is_live & duration <= 1200`, `--no-simulate`, `--no-progress`, `--print-json`, `--output`, `{stage_id}.%(ext)s`, URL canónica), la presencia innegociable de MP3 a 320 kbps y la validación de duración <= 1200 s y tamaño <= 100 MiB. <!-- sdd-owner: implementation -->
- [x] 1.4 GREEN: Implementar el trait del puerto `YoutubeDownloader` en `src/ports/youtube_downloader.rs`, `src/ports/mod.rs`, y el adaptador `ProcessYoutubeDownloader` junto con sus submódulos atómicos en `src/infra/youtube/resolve_youtube_binary.rs`, `src/infra/youtube/youtube_process_guard.rs`, `src/infra/youtube/purge_youtube_attempt.rs`, `src/infra/youtube/process_youtube_downloader.rs`, `src/infra/youtube/mod.rs` y `src/infra/mod.rs`, asegurando preflight de `ffmpeg`/`ffprobe`, timeout de 90 s con kill-and-wait, verificación física de archivo MP3 y purga completa de parciales `{stage_id}.*` ante cualquier fallo. <!-- sdd-owner: implementation -->
- [x] 1.5 TRIANGULATE: Añadir casos de prueba para terminación de proceso por timeout de 90 s, reapeo de procesos hijos ante cancelación de future, error controlado cuando `YT_DLP_BIN` no existe sin fallback silencioso, y rechazo de streams en vivo en `tests/youtube_downloader_tests.rs`. <!-- sdd-owner: implementation -->
- [x] 1.6 REFACTOR: Limpiar visibilidad de módulos en `src/infra/youtube/`, asegurar saneamiento estricto de diagnósticos (cero paths o stderr crudo en salidas públicas) y validar que `cargo test --test youtube_url_tests --test youtube_downloader_tests` pase limpiamente. <!-- sdd-owner: implementation -->

---

## Tareas completadas en Slice 2

- [x] 2.1 RED: Escribir pruebas unitarias de concurrencia y ciclo de vida para `StagedAudioRegistry` en `tests/staged_audio_registry_tests.rs`, verificando asignación de `staged_audio_id` opaco, estados `Available -> Reserved(lease_id) -> Consumed`, rechazo de segundo consumo con `audio_already_consumed`, liberación de reserva ante fallo de show item y purga automática de disco tras vencimiento del TTL de 15 minutos (probado con TTL corto inyectado). <!-- sdd-owner: implementation -->
- [x] 2.2 GREEN: Implementar `StagedAudioRegistry` en `src/state/staged_audio_registry.rs` con mutex granular sobre HashMap, tarea de expiración programada por entrada y exportar en `src/state/mod.rs`. <!-- sdd-owner: implementation -->
- [x] 2.3 RED: Escribir pruebas unitarias para `DownloadYouTubeAudio` y la guardia de promoción en `tests/download_youtube_audio_tests.rs`, validando fallback de título (solicitado > metadata > ID de 11 caracteres), registro en staging y reversión segura del archivo a `audio/staging/{stage_id}.part` si la inserción en base de datos falla durante la promoción. <!-- sdd-owner: implementation -->
- [x] 2.4 GREEN: Implementar `DownloadYouTubeAudio` en `src/use_cases/download_youtube_audio.rs`, la guardia de reversión `StagedPromotionGuard` en `src/infra/filesystem/staged_promotion_guard.rs`, integrar la compensación en `src/infra/filesystem/local_audio_storage.rs` y `src/use_cases/upload_track.rs`, exportar en `src/use_cases/mod.rs`, integrar `StagedAudioRegistry` y `YoutubeDownloader` en `src/state/app_state.rs`, y actualizar la reconciliación inicial en `src/infra/filesystem/reconcile_audio_files.rs` y `src/main.rs` para purgar temporales huérfanos al arrancar. <!-- sdd-owner: implementation -->
- [x] 2.5 TRIANGULATE: Añadir pruebas de estrés para reservas concurrentes del mismo `staged_audio_id`, fallos durante la compensación de filesystem y verificación de que el arranque nunca borra archivos bajo `audio/files` en `tests/staged_audio_registry_tests.rs`. <!-- sdd-owner: implementation -->
- [x] 2.6 REFACTOR: Verificar la separación estricta entre el `stage_id` interno del filesystem y el `staged_audio_id` público entregado a las capas superiores, ejecutando `cargo test --test staged_audio_registry_tests --test download_youtube_audio_tests`. <!-- sdd-owner: implementation -->

---

## Tareas completadas en Slice 3

- [x] 3.1 RED: Escribir pruebas en `tests/agent_tools_tests.rs` validando la allowlist estricta de 7 herramientas, el schema exacto de `download_youtube_track` (`url` obligatoria, `title` opcional, `additionalProperties: false`), la propiedad opcional `staged_audio_id` en `add_track_from_staged`, la prevención de llamadas duplicadas a `download_youtube_track` en el mismo turno y el despacho asíncrono. <!-- sdd-owner: implementation -->
- [x] 3.2 GREEN: Actualizar definiciones en `src/agent/tool_schemas.rs`, extender `TurnToolContext` en `src/agent/turn_context.rs` con la capability `staged_audio_id`, hacer asíncrono `AgentToolDispatcher::dispatch` en `src/agent/tool_dispatcher.rs`, y registrar la ejecución de `download_youtube_track` y el consumo de `add_track_from_staged`. <!-- sdd-owner: implementation -->
- [x] 3.3 RED: Escribir pruebas de loop y grounding en `tests/agent_chat_tests.rs` con stub de proveedor de IA (`StubChatProvider`), validando encadenamiento en el mismo turno cuando `presentation_id` está activo (descarga + alta directa de pista), detención con pregunta de destino cuando no hay presentación activa (emitiendo `staged_audio_id` en el output), segundo turno que consume el ID estructurado, y solicitud de aclaración ante nombres ambiguos de presentación. <!-- sdd-owner: implementation -->
- [x] 3.4 GREEN: Actualizar `src/agent/system_prompt.rs` con las directrices conversacionales de descarga de YouTube y encadenamiento, ajustar `compose_grounded_message` y el bucle principal en `src/use_cases/agent_chat.rs`, hacer opcional `presentation_id` en `ToolAction` de `src/domain/agent_tool.rs` para acciones de staging sin show, y añadir `staged_audio_id: Option<String>` a `AgentChatOutput`. <!-- sdd-owner: implementation -->
- [x] 3.5 TRIANGULATE: Añadir pruebas de robustez en `tests/agent_chat_tests.rs` para asegurar que afirmaciones falsas del modelo sin ejecución de herramienta son suprimidas por el grounding, y que errores de descarga (`downloader_not_configured`, timeout) devuelven mensajes amables en español sin exponer trazas técnicas. <!-- sdd-owner: implementation -->
- [x] 3.6 REFACTOR: Verificar que ninguna ruta exponga comandos de proceso ni paths absolutos en outputs o metadata del agente, confirmando que `cargo test --test agent_tools_tests --test agent_chat_tests` pase al 100%. <!-- sdd-owner: implementation -->

---

## Tareas completadas en Slice 4

- [x] 4.1 RED: Escribir pruebas de API HTTP en `tests/agent_chat_api_tests.rs` validando la lectura de `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS` en `Config`, la aceptación y round-trip de `staged_audio_id` en peticiones JSON y multipart, y la respuesta HTTP 200 con deadline unificado de 120 s ante timeout global reportando acciones confirmadas. <!-- sdd-owner: implementation -->
- [x] 4.2 GREEN: Actualizar `src/config.rs` y `.env.example` con las nuevas variables, ajustar `src/main.rs`, actualizar los parsers de solicitud en `src/handlers/agent/json_request.rs` y `src/handlers/agent/multipart_request.rs`, e implementar el deadline absoluto unificado en `src/handlers/agent/chat.rs`. <!-- sdd-owner: implementation -->
- [x] 4.3 RED: Escribir pruebas unitarias de frontend en `tests/agent-chat-state.test.mjs` y `tests/agent-chat-ui-components.test.mjs` para validar la detección de URLs de YouTube, la visualización del label "descargando el audio…" con `aria-live="polite"`, la retención de `staged_audio_id` en `chat-state.js` solo en memoria efímera (cero `localStorage`), su inclusión en la siguiente llamada API, y su limpieza tras agregar pista o ante comandos no relacionados. <!-- sdd-owner: implementation -->
- [x] 4.4 GREEN: Implementar las micro-utilidades puras `frontend/js/utils/validation/contains-youtube-url.js` e `is-staged-audio-unrelated.js`, actualizar la molécula `frontend/js/components/molecules/agent-typing-indicator.js` para recibir `label` dinámico, actualizar el panel en `frontend/js/components/organisms/agent-chat-panel.js`, extender `frontend/js/core/state/chat-state.js` con el ciclo de vida de `stagedAudioId`, y cablear el transporte en `frontend/js/data/agent-chat-api.js` y `frontend/js/controllers/agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [x] 4.5 TRIANGULATE: Añadir pruebas de integración en frontend verificando que errores de red preservan el `stagedAudioId` para reintentos, que desmontar el panel restaura el composer y elimina el indicador de actividad, y que mensajes con hosts parecidos (`notyoutube.com`) conservan el label general "Shirley está pensando...". <!-- sdd-owner: implementation -->
- [x] 4.6 REFACTOR: Ejecutar la suite completa de pruebas con `cargo test` (incluyendo los harnesses Node de chat UI) para verificar que no existen regresiones en subida manual de audio, streaming con Range HTTP, CRUD de presentaciones ni Stage Mode. <!-- sdd-owner: implementation -->

---

## Archivos modificados y creados en Slice 4

### Nuevos
- `frontend/js/utils/validation/contains-youtube-url.js`: Micro-utilidad pura (1 función = 1 archivo) que analiza cadenas en busca de URLs con protocolo `http:` o `https:` y host exacto en `['youtube.com', 'www.youtube.com', 'm.youtube.com', 'youtu.be']`. Rechaza dominios falsificados como `notyoutube.com` o texto plano que solo mencione la palabra youtube.
- `frontend/js/utils/validation/is-staged-audio-unrelated.js`: Micro-utilidad pura que clasifica conservadoramente comandos explícitamente no relacionados con el audio preparado (como creación/listado de presentaciones, agregado de notas/letras sin referencia a audio/pista, o nueva URL/adjunto), preservando la capability ante mensajes ambiguos o de seguimiento.

### Modificados
- `src/config.rs`: Incorporadas variables `YT_DLP_BIN` (opcional, trim y sin fallback silencioso) y `SHIRLEY_AGENT_TIMEOUT_SECS` (entero positivo con default de 120 segundos).
- `.env.example`: Documentadas las variables opcionales `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS`.
- `src/main.rs`: Inyectada la configuración de `YT_DLP_BIN` en `ProcessYoutubeDownloaderConfig` y `agent_timeout_secs` en `AgentLimits` de `AppState`.
- `src/handlers/agent/json_request.rs`: Añadido el parsing y validación estricta de `staged_audio_id` (trim, longitud máxima de 128 caracteres) en el payload JSON.
- `src/handlers/agent/multipart_request.rs`: Añadido el parsing y validación estricta de `staged_audio_id` (campo multipart, máximo 128 caracteres) en solicitudes con o sin archivo adjunto.
- `src/use_cases/agent_chat.rs`: Actualizado `AgentLimits::default().request_timeout` a 120 segundos, implementado el método `execute_until(input, deadline)` cubriendo bucle y tools sin competir con timeouts externos, y actualizado el copy del mensaje de timeout de "30 s" a formato dinámico basado en la configuración real (120 s).
- `src/handlers/agent/chat.rs`: Reemplazado el doble timeout exterior por el patrón de deadline unificado absoluto: `tokio::time::timeout_at(deadline, parse_request)` cubre lectura de body/multipart, y `AgentChat::execute_until(input, deadline)` ejecuta el agente garantizando que si se alcanza el deadline, todas las mutaciones comprometidas en `actions_taken` son reportadas en HTTP 200 con `agent_timeout`.
- `frontend/js/components/molecules/agent-typing-indicator.js`: Actualizada la molécula para aceptar opción `label` (por defecto `'Ressy está pensando...'`) y exponer el método `setLabel(newLabel)` para actualizar texto y `aria-label` dinámicamente manteniendo `aria-live="polite"`.
- `frontend/js/components/organisms/agent-chat-panel.js`: Acepta `pendingLabel` dinámico, pasa el label a la molécula de indicador, actualiza el mensaje de notice de timeout a "120 s", y sincroniza actualizaciones reactivas de label.
- `frontend/js/core/state/chat-state.js`: Incorporado `stagedAudioId: null` y `pendingLabel` en el estado efímero en memoria (cero `localStorage`/`sessionStorage`). Limpieza automática ante consumo exitoso (`add_track_from_staged`) o comandos no relacionados (`isStagedAudioUnrelated`). Preservación en fallos de red/proveedor para reintentos.
- `frontend/js/data/agent-chat-api.js`: Cableado `stagedAudioId` tanto en solicitudes JSON (`jsonBody.staged_audio_id`) como multipart (`formData.append('staged_audio_id', ...)`).
- `frontend/js/controllers/agent-chat-controller.js`: Sincronización de `pendingLabel` con el panel, propagación de `stagedAudioId` en llamadas API, temporizador de 3 segundos para retroalimentación cualitativa ante esperas prolongadas, y retorno de promesas en `open` y `sendAndOpen`.
- `tests/agent_chat_api_tests.rs`: Pruebas de configuración de `YT_DLP_BIN` y timeout de 120 s, round-trip y validación de `staged_audio_id` en JSON y multipart, y prueba de deadline unificado que preserva acciones comprometidas ante timeout devolviendo HTTP 200.
- `tests/agent-chat-state.test.mjs`: Pruebas para `containsYouTubeUrl`, `isStagedAudioUnrelated`, ciclo de vida de `stagedAudioId` y propagación de API.
- `tests/agent-chat-ui-components.test.mjs`: Pruebas para el indicador de escritura dinámico y `setLabel`.
- `tests/agent-chat-integration.test.mjs`: Pruebas de triangulación para preservación de `stagedAudioId` ante errores de red, desmontaje de panel y hosts parecidos.
- `openspec/changes/shirley-show-yt-download/tasks.md`: Marcadas tareas 4.1 a 4.6 completadas `[x]`.

---

## Evidencia de Ciclo TDD Estricto (Slice 4)

| Tarea | Fase | Acción realizada | Resultado / Comando |
|---|---|---|---|
| 4.1 | RED | Añadir pruebas de API en `tests/agent_chat_api_tests.rs` (lectura de config `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS`, round-trip y validación de 128 chars de `staged_audio_id` en JSON y multipart, y preservación de mutaciones ante deadline absoluto unificado en HTTP 200) | `cargo test --test agent_chat_api_tests` falló con 5 errores de compilación (`no field yt_dlp_bin on type Config`, etc.) |
| 4.2 | GREEN | Campos en `Config`, defaults y validación en `config.rs`, variables en `.env.example`, inyección en `main.rs`, parsers en `json_request.rs` y `multipart_request.rs`, `AgentLimits` a 120 s con `execute_until` en `agent_chat.rs`, y deadline unificado absoluto en `chat.rs` | `cargo test --test agent_chat_api_tests` pasó limpiamente (17 tests ok) |
| 4.3 | RED | Añadir pruebas de frontend en `tests/agent-chat-state.test.mjs` y `tests/agent-chat-ui-components.test.mjs` (`containsYouTubeUrl`, `isStagedAudioUnrelated`, ciclo de vida de `stagedAudioId` en memoria efímera sin `localStorage`, inclusión en API JSON/multipart, `label` dinámico y `setLabel` en indicador de actividad) | `node tests/agent-chat-state.test.mjs` falló con `ERR_MODULE_NOT_FOUND`, `node tests/agent-chat-ui-components.test.mjs` falló por discrepancia de label |
| 4.4 | GREEN | Implementación de `contains-youtube-url.js` e `is-staged-audio-unrelated.js`, actualización de `agent-typing-indicator.js` con `setLabel`, panel con `pendingLabel`, `chat-state.js` con ciclo de vida de `stagedAudioId`, y transporte en `agent-chat-api.js` y `agent-chat-controller.js` | `node tests/agent-chat-state.test.mjs` y `node tests/agent-chat-ui-components.test.mjs` pasaron limpiamente (100% ok) |
| 4.5 | TRIANGULATE | Pruebas de integración en `tests/agent-chat-integration.test.mjs` verificando que errores de red preservan `stagedAudioId` para reintentos seguros, que desmontar el panel elimina el indicador y restaura el composer, y que hosts parecidos (`notyoutube.com`) conservan el label general | `node tests/agent-chat-integration.test.mjs` pasó limpiamente (6 tests ok) |
| 4.6 | REFACTOR | Ejecución de suite completa de pruebas herméticas de Rust y harnesses Node. Verificación de cero regresiones en subidas manuales, Range HTTP, streaming, Stage Mode y CRUD | `cargo test` suite completa: 24 test suites ok, 0 fallos |

---

## Validación Manual E2E en Vivo (`:3100`)

Se ejecutó la validación manual de extremo a extremo con el servidor real en ejecución en `http://127.0.0.1:3100`:
1. **Creación de presentación de prueba:** `POST /api/presentations` -> Creado show `Test Presentacion E2E` con ID `3f85054d-bfd0-4a63-95e9-e7d9592ea015`.
2. **Descarga y encadenamiento en el mismo turno:** Solicitud conversacional con URL de YouTube real (`https://www.youtube.com/watch?v=jNQXAC9IVRw` - "Me at the zoo", 19 s). Ressy ejecutó `download_youtube_track` y `add_track_from_staged` en el mismo turno.
3. **Verificación de audio físico:**
   - Ubicación: `.run/data/audio/files/36b6de94-4199-4727-a445-03d5ec19f139.mp3`
   - Tamaño: `762.285 bytes`
   - Verificación con `ffprobe`: `codec_name: "mp3"`, `bit_rate: "320000"` (MP3 a 320 kbps innegociable, confirmado).
4. **Flujo de 2 turnos sin presentación activa:** Solicitud con `https://www.youtube.com/watch?v=jNQXAC9IVRw` sin show activo -> Ressy devolvió `staged_audio_id: "staged-a5391de0-4981-4b09-8b8a-8443bb878766"`, estado de staging activo con TTL de 15 minutos, y preguntó deterministamente a qué presentación agregarlo.
5. **Limpieza integral:** Se eliminó la presentación y pista de prueba vía `DELETE /api/presentations/{id}` (cascada confirmada) y se purgó el staging temporal. Se verificó que la lista de presentaciones y los directorios quedaron en estado inicial limpio.

---

## Comandos de verificación ejecutados

- `cargo check --tests`: 0 errores, 0 warnings.
- `cargo test --test agent_chat_api_tests`: 17 tests ok.
- `cargo test --test agent_chat_ui_tests`: 3 tests ok (cubriendo harnesses Node de state, UI components y route integration).
- `node tests/agent-chat-state.test.mjs`: 13 suites ok.
- `node tests/agent-chat-ui-components.test.mjs`: 14 suites ok.
- `node tests/agent-chat-integration.test.mjs`: 6 suites ok.
- `cargo test`: 24 test suites ok al 100%, 0 fallos.

---

## Desviaciones de diseño

Ninguna. La implementación de Slice 4 sigue al 100% las especificaciones de `design.md` y `specs/agent-chat/spec.md`:
- Deadline HTTP absoluto y unificado (`tokio::time::timeout_at`) que protege la lectura del cuerpo y el bucle del agente sin condiciones de carrera.
- Configuración transparente de `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS` con valor por defecto de 120 segundos.
- Micro-modularidad estricta en frontend con micro-utilidades puras para validación de URL y clasificación de comandos ajenos.
- Indicador de actividad cualitativo con `aria-live="polite"` y método `setLabel` dinámico.
- Memoria efímera estricta para `stagedAudioId` (cero persistencia en `localStorage`).

---

## Tareas restantes

**Ninguna.** Las 24 tareas de los 4 slices han sido completadas, verificadas y testeadas rigurosamente.

- Slice 1 (Dominio e Infraestructura): 6/6 completadas.
- Slice 2 (Caso de uso y Registro TTL): 6/6 completadas.
- Slice 3 (Herramientas y Agente): 6/6 completadas.
- Slice 4 (Deadline HTTP y Frontend): 6/6 completadas.
- **Total:** 24/24 completadas (100%).
