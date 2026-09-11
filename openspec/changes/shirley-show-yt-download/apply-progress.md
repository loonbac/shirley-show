# Progreso de aplicación: `shirley-show-yt-download`

**Fecha:** 2026-09-11  
**Slice activo:** Slice 1 (`yt-s1-domain-infra`)  
**Estrategia de entrega:** 4 PRs encadenados stacked-to-main (PR 1 = Slice 1)  
**Attempt authority:**
- Token: `sha256:e9dd8ea0ab6f61a6572571ad660417483f6d9ec41a1e61d0bc67bc6f57067f59`
- Work-unit: `"yt-s1-domain-infra"`
- Request: `y1-acquire-002`
- Max budget: 600 líneas (Reportado para settle, NO settle ejecutado)

---

## Tareas completadas en Slice 1

- [x] 1.1 RED: Escribir pruebas unitarias para `YouTubeUrl` en `tests/youtube_url_tests.rs` validando formatos compatibles (`watch?v=`, `youtu.be/`, `/shorts/`), normalización ignorando `&list=` y otros query params, y rechazo estricto de hosts externos, IDs inválidos y metacaracteres de shell (`;`, `|`, `$`, backtick, comillas). <!-- sdd-owner: implementation -->
- [x] 1.2 GREEN: Implementar el tipo de dominio `YouTubeUrl` y sus errores en `src/domain/youtube_url.rs`, registrar el módulo en `src/domain/mod.rs` y añadir la dependencia `url` en `Cargo.toml`. <!-- sdd-owner: implementation -->
- [x] 1.3 RED: Escribir pruebas herméticas de integración para `ProcessYoutubeDownloader` en `tests/youtube_downloader_tests.rs` utilizando scripts stub de prueba para `yt-dlp`, `ffmpeg` y `ffprobe` en directorio temporal aislado; asertar el orden y contenido exacto de los argumentos estructurados (`--no-config`, `--no-playlist`, `--extract-audio`, `--audio-format`, `mp3`, `--audio-quality`, `320K`, `--max-filesize`, `100M`, `--match-filter`, `!is_live & duration <= 1200`, `--no-simulate`, `--no-progress`, `--print-json`, `--output`, `{stage_id}.%(ext)s`, URL canónica), la presencia innegociable de MP3 a 320 kbps y la validación de duración <= 1200 s y tamaño <= 100 MiB. <!-- sdd-owner: implementation -->
- [x] 1.4 GREEN: Implementar el trait del puerto `YoutubeDownloader` en `src/ports/youtube_downloader.rs`, `src/ports/mod.rs`, y el adaptador `ProcessYoutubeDownloader` junto con sus submódulos atómicos en `src/infra/youtube/resolve_youtube_binary.rs`, `src/infra/youtube/youtube_process_guard.rs`, `src/infra/youtube/purge_youtube_attempt.rs`, `src/infra/youtube/process_youtube_downloader.rs`, `src/infra/youtube/mod.rs` y `src/infra/mod.rs`, asegurando preflight de `ffmpeg`/`ffprobe`, timeout de 90 s con kill-and-wait, verificación física de archivo MP3 y purga completa de parciales `{stage_id}.*` ante cualquier fallo. <!-- sdd-owner: implementation -->
- [x] 1.5 TRIANGULATE: Añadir casos de prueba para terminación de proceso por timeout de 90 s, reapeo de procesos hijos ante cancelación de future, error controlado cuando `YT_DLP_BIN` no existe sin fallback silencioso, y rechazo de streams en vivo en `tests/youtube_downloader_tests.rs`. <!-- sdd-owner: implementation -->
- [x] 1.6 REFACTOR: Limpiar visibilidad de módulos en `src/infra/youtube/`, asegurar saneamiento estricto de diagnósticos (cero paths o stderr crudo en salidas públicas) y validar que `cargo test --test youtube_url_tests --test youtube_downloader_tests` pase limpiamente. <!-- sdd-owner: implementation -->

---

## Archivos modificados y creados en Slice 1

### Creados
- `src/domain/youtube_url.rs`: Tipo de valor de dominio con validación estricta y normalización canónica de URLs de YouTube.
- `src/ports/youtube_downloader.rs`: Trait asíncrono object-safe `YoutubeDownloader`, target de descarga, resultado y enum de errores `YoutubeDownloadError`.
- `src/infra/youtube/resolve_youtube_binary.rs`: Resolución estricta de `YT_DLP_BIN` sin fallback si se especifica erróneamente, propagación de PATH para subprocesos y preflight de `ffmpeg`/`ffprobe`.
- `src/infra/youtube/purge_youtube_attempt.rs`: Purga atómica y segura de archivos `{stage_id}.*` y `{stage_id}.part` en directorio staging sin tocar `audio/files`.
- `src/infra/youtube/youtube_process_guard.rs`: RAII guard para terminación inmediata (kill-and-wait) y reapeo en drop ante cancelación de future.
- `src/infra/youtube/process_youtube_downloader.rs`: Adaptador concreto con ejecución estructurada de `yt-dlp` (MP3 320 kbps innegociable, límite 100M, max 1200s), validación de magic bytes y ffprobe duration.
- `src/infra/youtube/mod.rs`: Exportaciones del módulo de infraestructura.
- `tests/youtube_url_tests.rs`: Suite de pruebas unitarias para `YouTubeUrl`.
- `tests/youtube_downloader_tests.rs`: Suite de pruebas herméticas con stubs de `yt-dlp`, `ffmpeg` y `ffprobe`.

### Modificados
- `Cargo.toml`: Añadida dependencia `url = "2"`.
- `src/domain/mod.rs`: Exportado módulo y tipos de `youtube_url`.
- `src/ports/mod.rs`: Exportado puerto y tipos de `youtube_downloader`.
- `src/infra/mod.rs`: Exportado módulo `youtube`.
- `openspec/changes/shirley-show-yt-download/tasks.md`: Checkboxes de Slice 1 marcados `[x]`.

---

## Evidencia de Ciclo TDD Estricto

| Tarea | Fase | Acción realizada | Resultado / Comando |
|---|---|---|---|
| 1.1 | RED | Creación de pruebas unitarias para `YouTubeUrl` con formatos compatibles, normalización y rechazo estricto | `cargo test --test youtube_url_tests` falló por tipo no existente (E0432) |
| 1.2 | GREEN | Implementación de `YouTubeUrl` y `YouTubeUrlError` con crate `url` | `cargo test --test youtube_url_tests` pasó (9 tests ok) |
| 1.3 | RED | Pruebas herméticas de `ProcessYoutubeDownloader` con stubs verificando argumentos estructurados, MP3 320K, límites de tamaño y duración | `cargo test --test youtube_downloader_tests` falló por módulos no existentes (E0432) |
| 1.4 | GREEN | Implementación del puerto `YoutubeDownloader` y adaptador `ProcessYoutubeDownloader` con sub-módulos atómicos en `src/infra/youtube/` | `cargo test --test youtube_downloader_tests` pasó (3 tests ok) |
| 1.5 | TRIANGULATE | Pruebas adicionales para timeout 90s, reapeo de hijos al cancelar future, rechazo sin fallback de `YT_DLP_BIN` inexistente y rechazo de streams en vivo | `cargo test --test youtube_downloader_tests` pasó (7 tests ok) |
| 1.6 | REFACTOR | Saneamiento de visibilidad y salidas de error sin trazas crudas ni rutas absolutas, verificación integral limpia | `cargo test` suite completa (22 suites) ok sin errores |

---

## Comandos de verificación ejecutados

- `cargo check --tests`: 0 errores, 0 warnings.
- `cargo test --test youtube_url_tests`: 9 tests ok.
- `cargo test --test youtube_downloader_tests`: 7 tests ok.
- `cargo test`: 22 test suites ok (incluyendo API de presentaciones, audio streaming, agent tools y chat UI).

---

## Desviaciones de diseño

Ninguna. La implementación sigue al 100% las especificaciones de `design.md` y `specs/agent-tools/spec.md`, incluyendo argumentos exactos de `yt-dlp`, calidad forzada a 320K MP3, límites de 1200 s y 100 MiB, y modularidad extrema de archivos según `AGENTS.md`.

---

## Tareas restantes (Slices 2, 3 y 4 intactos)

```markdown
- [ ] 2.1 RED: Escribir pruebas unitarias de concurrencia y ciclo de vida para `StagedAudioRegistry` en `tests/staged_audio_registry_tests.rs`, verificando asignación de `staged_audio_id` opaco, estados `Available -> Reserved(lease_id) -> Consumed`, rechazo de segundo consumo con `audio_already_consumed`, liberación de reserva ante fallo de show item y purga automática de disco tras vencimiento del TTL de 15 minutos (probado con TTL corto inyectado). <!-- sdd-owner: implementation -->
- [ ] 2.2 GREEN: Implementar `StagedAudioRegistry` en `src/state/staged_audio_registry.rs` con mutex granular sobre HashMap, tarea de expiración programada por entrada y exportar en `src/state/mod.rs`. <!-- sdd-owner: implementation -->
- [ ] 2.3 RED: Escribir pruebas unitarias para `DownloadYouTubeAudio` y la guardia de promoción en `tests/download_youtube_audio_tests.rs`, validando fallback de título (solicitado > metadata > ID de 11 caracteres), registro en staging y reversión segura del archivo a `audio/staging/{stage_id}.part` si la inserción en base de datos falla durante la promoción. <!-- sdd-owner: implementation -->
- [ ] 2.4 GREEN: Implementar `DownloadYouTubeAudio` en `src/use_cases/download_youtube_audio.rs`, la guardia de reversión `StagedPromotionGuard` en `src/infra/filesystem/staged_promotion_guard.rs`, integrar la compensación en `src/infra/filesystem/local_audio_storage.rs` y `src/use_cases/upload_track.rs`, exportar en `src/use_cases/mod.rs`, integrar `StagedAudioRegistry` y `YoutubeDownloader` en `src/state/app_state.rs`, y actualizar la reconciliación inicial en `src/infra/filesystem/reconcile_audio_files.rs` y `src/main.rs` para purgar temporales huérfanos al arrancar. <!-- sdd-owner: implementation -->
- [ ] 2.5 TRIANGULATE: Añadir pruebas de estrés para reservas concurrentes del mismo `staged_audio_id`, fallos durante la compensación de filesystem y verificación de que el arranque nunca borra archivos bajo `audio/files` en `tests/staged_audio_registry_tests.rs`. <!-- sdd-owner: implementation -->
- [ ] 2.6 REFACTOR: Verificar la separación estricta entre el `stage_id` interno del filesystem y el `staged_audio_id` público entregado a las capas superiores, ejecutando `cargo test --test staged_audio_registry_tests --test download_youtube_audio_tests`. <!-- sdd-owner: implementation -->
- [ ] 3.1 RED: Escribir pruebas en `tests/agent_tools_tests.rs` validando la allowlist estricta de 7 herramientas, el schema exacto de `download_youtube_track` (`url` obligatoria, `title` opcional, `additionalProperties: false`), la propiedad opcional `staged_audio_id` en `add_track_from_staged`, la prevención de llamadas duplicadas a `download_youtube_track` en el mismo turno y el despacho asíncrono. <!-- sdd-owner: implementation -->
- [ ] 3.2 GREEN: Actualizar definiciones en `src/agent/tool_schemas.rs`, extender `TurnToolContext` en `src/agent/turn_context.rs` con la capability `staged_audio_id`, hacer asíncrono `AgentToolDispatcher::dispatch` en `src/agent/tool_dispatcher.rs`, y registrar la ejecución de `download_youtube_track` y el consumo de `add_track_from_staged`. <!-- sdd-owner: implementation -->
- [ ] 3.3 RED: Escribir pruebas de loop y grounding en `tests/agent_chat_tests.rs` con stub de proveedor de IA (`StubChatProvider`), validando encadenamiento en el mismo turno cuando `presentation_id` está activo (descarga + alta directa de pista), detención con pregunta de destino cuando no hay presentación activa (emitiendo `staged_audio_id` en el output), segundo turno que consume el ID estructurado, y solicitud de aclaración ante nombres ambiguos de presentación. <!-- sdd-owner: implementation -->
- [ ] 3.4 GREEN: Actualizar `src/agent/system_prompt.rs` con las directrices conversacionales de descarga de YouTube y encadenamiento, ajustar `compose_grounded_message` y el bucle principal en `src/use_cases/agent_chat.rs`, hacer opcional `presentation_id` en `ToolAction` de `src/domain/agent_tool.rs` para acciones de staging sin show, y añadir `staged_audio_id: Option<String>` a `AgentChatOutput`. <!-- sdd-owner: implementation -->
- [ ] 3.5 TRIANGULATE: Añadir pruebas de robustez en `tests/agent_chat_tests.rs` para asegurar que afirmaciones falsas del modelo sin ejecución de herramienta son suprimidas por el grounding, y que errores de descarga (`downloader_not_configured`, timeout) devuelven mensajes amables en español sin exponer trazas técnicas. <!-- sdd-owner: implementation -->
- [ ] 3.6 REFACTOR: Verificar que ninguna ruta exponga comandos de proceso ni paths absolutos en outputs o metadata del agente, confirmando que `cargo test --test agent_tools_tests --test agent_chat_tests` pase al 100%. <!-- sdd-owner: implementation -->
- [ ] 4.1 RED: Escribir pruebas de API HTTP en `tests/agent_chat_api_tests.rs` validando la lectura de `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS` en `Config`, la aceptación y round-trip de `staged_audio_id` en peticiones JSON y multipart, y la respuesta HTTP 200 con deadline unificado de 120 s ante timeout global reportando acciones confirmadas. <!-- sdd-owner: implementation -->
- [ ] 4.2 GREEN: Actualizar `src/config.rs` y `.env.example` con las nuevas variables, ajustar `src/main.rs`, actualizar los parsers de solicitud en `src/handlers/agent/json_request.rs` y `src/handlers/agent/multipart_request.rs`, e implementar el deadline absoluto unificado en `src/handlers/agent/chat.rs`. <!-- sdd-owner: implementation -->
- [ ] 4.3 RED: Escribir pruebas unitarias de frontend en `tests/agent-chat-state.test.mjs` y `tests/agent-chat-ui-components.test.mjs` para validar la detección de URLs de YouTube, la visualización del label "descargando el audio…" con `aria-live="polite"`, la retención de `staged_audio_id` en `chat-state.js` solo en memoria efímera (cero `localStorage`), su inclusión en la siguiente llamada API, y su limpieza tras agregar pista o ante comandos no relacionados. <!-- sdd-owner: implementation -->
- [ ] 4.4 GREEN: Implementar las micro-utilidades puras `frontend/js/utils/validation/contains-youtube-url.js` e `is-staged-audio-unrelated.js`, actualizar la molécula `frontend/js/components/molecules/agent-typing-indicator.js` para recibir `label` dinámico, actualizar el panel en `frontend/js/components/organisms/agent-chat-panel.js`, extender `frontend/js/core/state/chat-state.js` con el ciclo de vida de `stagedAudioId`, y cablear el transporte en `frontend/js/data/agent-chat-api.js` y `frontend/js/controllers/agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [ ] 4.5 TRIANGULATE: Añadir pruebas de integración en frontend verificando que errores de red preservan el `stagedAudioId` para reintentos, que desmontar el panel restaura el composer y elimina el indicador de actividad, y que mensajes con hosts parecidos (`notyoutube.com`) conservan el label general "Shirley está pensando...". <!-- sdd-owner: implementation -->
- [ ] 4.6 REFACTOR: Ejecutar la suite completa de pruebas con `cargo test` (incluyendo los harnesses Node de chat UI) para verificar que no existen regresiones en subida manual de audio, streaming con Range HTTP, CRUD de presentaciones ni Stage Mode. <!-- sdd-owner: implementation -->
```
