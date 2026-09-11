# Tasks: Descarga de audio de YouTube con Ressy

**Change:** `shirley-show-yt-download`  
**Status:** Ready for apply  
**Delivery strategy:** ask-on-risk  
**Review budget:** 400 changed lines per autonomous review unit  

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 980 - 1180 lines (S1: ~290, S2: ~270, S3: ~280, S4: ~240) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Slice 1: Dominio e Infra) → PR 2 (Slice 2: Caso de uso y Registro) → PR 3 (Slice 3: Tools y Agente) → PR 4 (Slice 4: Timeout, API y Frontend) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

---

## Slice 1: Dominio, puerto e infraestructura de descarga (S1)

**Objetivo:** Validar URLs de YouTube (`YouTubeUrl`), definir el puerto `YoutubeDownloader` e implementar `ProcessYoutubeDownloader` con argumentos estructurados seguros, timeout de 90 s, cancelación segura y pruebas herméticas con stub ejecutable que verifiquen `--audio-quality 320K` y extracción MP3.

- [x] 1.1 RED: Escribir pruebas unitarias para `YouTubeUrl` en `tests/youtube_url_tests.rs` validando formatos compatibles (`watch?v=`, `youtu.be/`, `/shorts/`), normalización ignorando `&list=` y otros query params, y rechazo estricto de hosts externos, IDs inválidos y metacaracteres de shell (`;`, `|`, `$`, backtick, comillas). <!-- sdd-owner: implementation -->
- [x] 1.2 GREEN: Implementar el tipo de dominio `YouTubeUrl` y sus errores en `src/domain/youtube_url.rs`, registrar el módulo en `src/domain/mod.rs` y añadir la dependencia `url` en `Cargo.toml`. <!-- sdd-owner: implementation -->
- [x] 1.3 RED: Escribir pruebas herméticas de integración para `ProcessYoutubeDownloader` en `tests/youtube_downloader_tests.rs` utilizando scripts stub de prueba para `yt-dlp`, `ffmpeg` y `ffprobe` en directorio temporal aislado; asertar el orden y contenido exacto de los argumentos estructurados (`--no-config`, `--no-playlist`, `--extract-audio`, `--audio-format`, `mp3`, `--audio-quality`, `320K`, `--max-filesize`, `100M`, `--match-filter`, `!is_live & duration <= 1200`, `--no-simulate`, `--no-progress`, `--print-json`, `--output`, `{stage_id}.%(ext)s`, URL canónica), la presencia innegociable de MP3 a 320 kbps y la validación de duración <= 1200 s y tamaño <= 100 MiB. <!-- sdd-owner: implementation -->
- [x] 1.4 GREEN: Implementar el trait del puerto `YoutubeDownloader` en `src/ports/youtube_downloader.rs`, `src/ports/mod.rs`, y el adaptador `ProcessYoutubeDownloader` junto con sus submódulos atómicos en `src/infra/youtube/resolve_youtube_binary.rs`, `src/infra/youtube/youtube_process_guard.rs`, `src/infra/youtube/purge_youtube_attempt.rs`, `src/infra/youtube/process_youtube_downloader.rs`, `src/infra/youtube/mod.rs` y `src/infra/mod.rs`, asegurando preflight de `ffmpeg`/`ffprobe`, timeout de 90 s con kill-and-wait, verificación física de archivo MP3 y purga completa de parciales `{stage_id}.*` ante cualquier fallo. <!-- sdd-owner: implementation -->
- [x] 1.5 TRIANGULATE: Añadir casos de prueba para terminación de proceso por timeout de 90 s, reapeo de procesos hijos ante cancelación de future, error controlado cuando `YT_DLP_BIN` no existe sin fallback silencioso, y rechazo de streams en vivo en `tests/youtube_downloader_tests.rs`. <!-- sdd-owner: implementation -->
- [x] 1.6 REFACTOR: Limpiar visibilidad de módulos en `src/infra/youtube/`, asegurar saneamiento estricto de diagnósticos (cero paths o stderr crudo en salidas públicas) y validar que `cargo test --test youtube_url_tests --test youtube_downloader_tests` pase limpiamente. <!-- sdd-owner: implementation -->

---

## Slice 2: Caso de uso, registro temporal TTL y AppState (S2)

**Objetivo:** Crear el registro en memoria `StagedAudioRegistry` con TTL de 15 minutos, consumo único y reserva atómica, el caso de uso `DownloadYouTubeAudio`, la compensación transaccional en `LocalAudioStorage` / `UploadTrack` y la inyección en `AppState` y purga de arranque.

- [ ] 2.1 RED: Escribir pruebas unitarias de concurrencia y ciclo de vida para `StagedAudioRegistry` en `tests/staged_audio_registry_tests.rs`, verificando asignación de `staged_audio_id` opaco, estados `Available -> Reserved(lease_id) -> Consumed`, rechazo de segundo consumo con `audio_already_consumed`, liberación de reserva ante fallo de show item y purga automática de disco tras vencimiento del TTL de 15 minutos (probado con TTL corto inyectado). <!-- sdd-owner: implementation -->
- [ ] 2.2 GREEN: Implementar `StagedAudioRegistry` en `src/state/staged_audio_registry.rs` con mutex granular sobre HashMap, tarea de expiración programada por entrada y exportar en `src/state/mod.rs`. <!-- sdd-owner: implementation -->
- [ ] 2.3 RED: Escribir pruebas unitarias para `DownloadYouTubeAudio` y la guardia de promoción en `tests/download_youtube_audio_tests.rs`, validando fallback de título (solicitado > metadata > ID de 11 caracteres), registro en staging y reversión segura del archivo a `audio/staging/{stage_id}.part` si la inserción en base de datos falla durante la promoción. <!-- sdd-owner: implementation -->
- [ ] 2.4 GREEN: Implementar `DownloadYouTubeAudio` en `src/use_cases/download_youtube_audio.rs`, la guardia de reversión `StagedPromotionGuard` en `src/infra/filesystem/staged_promotion_guard.rs`, integrar la compensación en `src/infra/filesystem/local_audio_storage.rs` y `src/use_cases/upload_track.rs`, exportar en `src/use_cases/mod.rs`, integrar `StagedAudioRegistry` y `YoutubeDownloader` en `src/state/app_state.rs`, y actualizar la reconciliación inicial en `src/infra/filesystem/reconcile_audio_files.rs` y `src/main.rs` para purgar temporales huérfanos al arrancar. <!-- sdd-owner: implementation -->
- [ ] 2.5 TRIANGULATE: Añadir pruebas de estrés para reservas concurrentes del mismo `staged_audio_id`, fallos durante la compensación de filesystem y verificación de que el arranque nunca borra archivos bajo `audio/files` en `tests/staged_audio_registry_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] 2.6 REFACTOR: Verificar la separación estricta entre el `stage_id` interno del filesystem y el `staged_audio_id` público entregado a las capas superiores, ejecutando `cargo test --test staged_audio_registry_tests --test download_youtube_audio_tests`. <!-- sdd-owner: implementation -->

---

## Slice 3: Herramientas del agente, dispatcher y orquestación conversacional (S3)

**Objetivo:** Añadir `download_youtube_track` a la allowlist (exactamente 7 herramientas), actualizar `add_track_from_staged` para admitir `staged_audio_id`, volver asíncrono el dispatcher con binding de capabilities, y ajustar prompt y grounding determinista diferenciando "audio preparado" de "pista agregada".

- [ ] 3.1 RED: Escribir pruebas en `tests/agent_tools_tests.rs` validando la allowlist estricta de 7 herramientas, el schema exacto de `download_youtube_track` (`url` obligatoria, `title` opcional, `additionalProperties: false`), la propiedad opcional `staged_audio_id` en `add_track_from_staged`, la prevención de llamadas duplicadas a `download_youtube_track` en el mismo turno y el despacho asíncrono. <!-- sdd-owner: implementation -->
- [ ] 3.2 GREEN: Actualizar definiciones en `src/agent/tool_schemas.rs`, extender `TurnToolContext` en `src/agent/turn_context.rs` con la capability `staged_audio_id`, hacer asíncrono `AgentToolDispatcher::dispatch` en `src/agent/tool_dispatcher.rs`, y registrar la ejecución de `download_youtube_track` y el consumo de `add_track_from_staged`. <!-- sdd-owner: implementation -->
- [ ] 3.3 RED: Escribir pruebas de loop y grounding en `tests/agent_chat_tests.rs` con stub de proveedor de IA (`StubChatProvider`), validando encadenamiento en el mismo turno cuando `presentation_id` está activo (descarga + alta directa de pista), detención con pregunta de destino cuando no hay presentación activa (emitiendo `staged_audio_id` en el output), segundo turno que consume el ID estructurado, y solicitud de aclaración ante nombres ambiguos de presentación. <!-- sdd-owner: implementation -->
- [ ] 3.4 GREEN: Actualizar `src/agent/system_prompt.rs` con las directrices conversacionales de descarga de YouTube y encadenamiento, ajustar `compose_grounded_message` y el bucle principal en `src/use_cases/agent_chat.rs`, hacer opcional `presentation_id` en `ToolAction` de `src/domain/agent_tool.rs` para acciones de staging sin show, y añadir `staged_audio_id: Option<String>` a `AgentChatOutput`. <!-- sdd-owner: implementation -->
- [ ] 3.5 TRIANGULATE: Añadir pruebas de robustez en `tests/agent_chat_tests.rs` para asegurar que afirmaciones falsas del modelo sin ejecución de herramienta son suprimidas por el grounding, y que errores de descarga (`downloader_not_configured`, timeout) devuelven mensajes amables en español sin exponer trazas técnicas. <!-- sdd-owner: implementation -->
- [ ] 3.6 REFACTOR: Verificar que ninguna ruta exponga comandos de proceso ni paths absolutos en outputs o metadata del agente, confirmando que `cargo test --test agent_tools_tests --test agent_chat_tests` pase al 100%. <!-- sdd-owner: implementation -->

---

## Slice 4: Deadline HTTP, continuidad de estado y feedback en frontend (S4)

**Objetivo:** Configurar el timeout global de 120 s con `SHIRLEY_AGENT_TIMEOUT_SECS` y variable `YT_DLP_BIN`, transportar `staged_audio_id` en el endpoint `/api/agent/chat` (JSON y multipart), e implementar en frontend el indicador cualitativo "descargando el audio…" y la continuidad efímera entre turnos sin persistencia local.

- [ ] 4.1 RED: Escribir pruebas de API HTTP en `tests/agent_chat_api_tests.rs` validando la lectura de `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS` en `Config`, la aceptación y round-trip de `staged_audio_id` en peticiones JSON y multipart, y la respuesta HTTP 200 con deadline unificado de 120 s ante timeout global reportando acciones confirmadas. <!-- sdd-owner: implementation -->
- [ ] 4.2 GREEN: Actualizar `src/config.rs` y `.env.example` con las nuevas variables, ajustar `src/main.rs`, actualizar los parsers de solicitud en `src/handlers/agent/json_request.rs` y `src/handlers/agent/multipart_request.rs`, e implementar el deadline absoluto unificado en `src/handlers/agent/chat.rs`. <!-- sdd-owner: implementation -->
- [ ] 4.3 RED: Escribir pruebas unitarias de frontend en `tests/agent-chat-state.test.mjs` y `tests/agent-chat-ui-components.test.mjs` para validar la detección de URLs de YouTube, la visualización del label "descargando el audio…" con `aria-live="polite"`, la retención de `staged_audio_id` en `chat-state.js` solo en memoria efímera (cero `localStorage`), su inclusión en la siguiente llamada API, y su limpieza tras agregar pista o ante comandos no relacionados. <!-- sdd-owner: implementation -->
- [ ] 4.4 GREEN: Implementar las micro-utilidades puras `frontend/js/utils/validation/contains-youtube-url.js` e `is-staged-audio-unrelated.js`, actualizar la molécula `frontend/js/components/molecules/agent-typing-indicator.js` para recibir `label` dinámico, actualizar el panel en `frontend/js/components/organisms/agent-chat-panel.js`, extender `frontend/js/core/state/chat-state.js` con el ciclo de vida de `stagedAudioId`, y cablear el transporte en `frontend/js/data/agent-chat-api.js` y `frontend/js/controllers/agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [ ] 4.5 TRIANGULATE: Añadir pruebas de integración en frontend verificando que errores de red preservan el `stagedAudioId` para reintentos, que desmontar el panel restaura el composer y elimina el indicador de actividad, y que mensajes con hosts parecidos (`notyoutube.com`) conservan el label general "Shirley está pensando...". <!-- sdd-owner: implementation -->
- [ ] 4.6 REFACTOR: Ejecutar la suite completa de pruebas con `cargo test` (incluyendo los harnesses Node de chat UI) para verificar que no existen regresiones en subida manual de audio, streaming con Range HTTP, CRUD de presentaciones ni Stage Mode. <!-- sdd-owner: implementation -->
