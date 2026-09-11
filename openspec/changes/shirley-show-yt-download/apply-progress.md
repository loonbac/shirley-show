# Progreso de aplicación: `shirley-show-yt-download`

**Fecha:** 2026-09-11  
**Slice activo:** Slice 3 (`yt-s3-tool-agent`)  
**Estrategia de entrega:** 4 PRs encadenados stacked-to-main (PR 1 = Slice 1, PR 2 = Slice 2, PR 3 = Slice 3)  
**Attempt authority:**
- Token: `sha256:c924dd1728579bfe9507ceab24e3b99ab57be1e4b9c2c2b1768864314142e70d`
- Work-unit: `"yt-s3-tool-agent"`
- Request: `y3-acquire-001`
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

## Archivos modificados y creados en Slice 3

### Modificados
- `src/domain/agent_tool.rs`: `ToolAction.presentation_id` actualizado a `Option<String>` con `#[serde(skip_serializing_if = "Option::is_none")]` para permitir acciones de staging de audio desacopladas de un show específico sin fabricar IDs fantasmas.
- `src/agent/tool_schemas.rs`: Allowlist extendida a exactamente 7 herramientas. Añadido el schema estricto de `download_youtube_track` (`url` requerida, `title` opcional, `additionalProperties: false`) y añadida la propiedad opcional `staged_audio_id` en `add_track_from_staged`.
- `src/agent/turn_context.rs`: `TurnToolContext` extendido con `staged_audio_id: Option<String>`, conjunto `known_staged_audio_ids: HashSet<String>`, constructores `with_staged_audio_id` / `with_full_context`, y réplica atómica en `CommittedTurnState` para conservación de estado ante timeouts.
- `src/agent/tool_dispatcher.rs`: `AgentToolDispatcher::dispatch` transformado a función asíncrona; incorporada ejecución de `download_youtube_audio` con registro de capability y acción auditada `Audio «título» preparado temporalmente` con payload `{status, title, byte_size, format}`, deduplicación por ID y firma canónica; y actualización de `add_track_from_staged` para resolver de forma prioritaria el `staged_audio_id` del registro (con lease transaccional, commit y limpieza de capability) o recurrir al adjunto request-scoped.
- `src/agent/system_prompt.rs`: Incorporadas directrices conversacionales para Ressy en español: encadenamiento en el mismo turno con show activo, detención y consulta de show destino tras preparar audio sin show, resolución directa en segundo turno sin re-descarga y diferenciación estricta entre audio preparado y pista agregada.
- `src/use_cases/agent_chat.rs`: Bucle conversacional actualizado para esperar el despacho asíncrono de herramientas; `AgentChatInput` y `AgentChatOutput` enriquecidos con `staged_audio_id: Option<String>`; `compose_grounded_message` extendido para adjuntar la pregunta de destino ante audio preparado pendiente; y añadido filtro de supresión de afirmaciones falsas del modelo sin respaldo de ledger en `actions_taken`.
- `src/state/app_state.rs`: Inyectados `download_youtube_audio` y `staged_audio_registry` en `AgentToolDispatcher::from_components`.
- `src/handlers/agent/json_request.rs`: Inicializado `staged_audio_id: None` en `AgentChatInput`.
- `src/handlers/agent/multipart_request.rs`: Inicializado `staged_audio_id: None` en `AgentChatInput`.
- `src/handlers/agent/chat.rs`: Añadido `staged_audio_id: None` en el envelope de timeout controlado.
- `tests/agent_tools_tests.rs`: Pruebas exhaustivas para la allowlist de 7 herramientas, schemas exactos, deduplicación de mutaciones, despacho asíncrono, capability `staged_audio_id` y aislamiento ante IDs no autorizados o consumidos.
- `tests/agent_chat_tests.rs`: Pruebas de integración del bucle conversacional con `StubChatProvider`, encadenamiento en el mismo turno, emisión de `staged_audio_id` sin destino, segundo turno con consumo estructurado, aclaración ante nombres ambiguos, y supresión de alucinaciones mediante grounding.
- `openspec/changes/shirley-show-yt-download/tasks.md`: Marcadas tareas 3.1 a 3.6 completadas `[x]`.

---

## Evidencia de Ciclo TDD Estricto (Slice 3)

| Tarea | Fase | Acción realizada | Resultado / Comando |
|---|---|---|---|
| 3.1 | RED | Creación de pruebas unitarias y de dispatcher en `tests/agent_tools_tests.rs` (allowlist de 7 herramientas, schemas exactos, despacho async, deduplicación y consumo de staging) | `cargo test --test agent_tools_tests` falló con 38 errores de compilación (`Value is not a future`, `staged_audio_id unknown`, etc.) |
| 3.2 | GREEN | Schemas de 7 tools en `tool_schemas.rs`, extensión de `TurnToolContext` con capability `staged_audio_id`, despacho asíncrono en `AgentToolDispatcher` con leases transaccionales de `StagedAudioRegistry` | `cargo test --test agent_tools_tests` pasó limpiamente (12 tests ok) |
| 3.3 | RED | Pruebas de bucle y grounding en `tests/agent_chat_tests.rs` con `StubChatProvider` (encadenamiento mismo turno, detención con pregunta de destino y staged_audio_id, segundo turno estructurado y ambigüedad de nombres) | `cargo test --test agent_chat_tests` falló por campos faltantes en `AgentChatInput`/`AgentChatOutput` |
| 3.4 | GREEN | Directrices conversacionales en `system_prompt.rs`, despacho async y prompt de contexto en `agent_chat.rs`, `staged_audio_id` en DTOs de chat, y grounding determinista con pregunta de destino | `cargo test --test agent_chat_tests` pasó (12 tests ok) |
| 3.5 | TRIANGULATE | Pruebas de robustez para supresión de alucinaciones del modelo sin llamadas a tools (`sanitize_ungrounded_claims`), y respuestas amables ante errores de configuración y timeout sin trazas técnicas | `cargo test --test agent_chat_tests` pasó (15 tests ok) |
| 3.6 | REFACTOR | Verificación integral de cero exposición de rutas de proceso y saneamiento de salidas. Ejecución de suite completa de pruebas | `cargo test` suite completa: 24 suites ok, 0 fallos |

---

## Comandos de verificación ejecutados

- `cargo check --tests`: 0 errores, 0 warnings.
- `cargo test --test agent_tools_tests`: 12 tests ok.
- `cargo test --test agent_chat_tests`: 15 tests ok.
- `cargo test --test agent_tools_tests --test agent_chat_tests`: 27 tests ok.
- `cargo test`: 24 test suites ok (incluyendo API HTTP, SQLite migrations, audio streaming y harnesses UI).

---

## Desviaciones de diseño

Ninguna. La implementación se ajusta fielmente a `design.md`, `specs/agent-tools/spec.md` y `specs/agent-chat/spec.md`:
- Allowlist de exactamente 7 herramientas locales.
- Capability de `staged_audio_id` vinculada estrictamente al turno conversacional (`TurnToolContext`), impidiendo que el modelo consulte o consuma identificadores arbitrarios sin autorización previa.
- Grounding server-side estricto que gobierna el mensaje final tras cualquier mutación y suprime afirmaciones falsas del modelo sin ejecución real de herramientas.
- `ToolAction.presentation_id` como `Option<String>` con serialización limpia (omitido si es `None`), evitando fabricar identificadores para descargas que aún no tienen presentación asignada.

---

## Tareas restantes (Slice 4 intacto)

```markdown
- [ ] 4.1 RED: Escribir pruebas de API HTTP en `tests/agent_chat_api_tests.rs` validando la lectura de `YT_DLP_BIN` y `SHIRLEY_AGENT_TIMEOUT_SECS` en `Config`, la aceptación y round-trip de `staged_audio_id` en peticiones JSON y multipart, y la respuesta HTTP 200 con deadline unificado de 120 s ante timeout global reportando acciones confirmadas. <!-- sdd-owner: implementation -->
- [ ] 4.2 GREEN: Actualizar `src/config.rs` y `.env.example` con las nuevas variables, ajustar `src/main.rs`, actualizar los parsers de solicitud en `src/handlers/agent/json_request.rs` y `src/handlers/agent/multipart_request.rs`, e implementar el deadline absoluto unificado en `src/handlers/agent/chat.rs`. <!-- sdd-owner: implementation -->
- [ ] 4.3 RED: Escribir pruebas unitarias de frontend en `tests/agent-chat-state.test.mjs` y `tests/agent-chat-ui-components.test.mjs` para validar la detección de URLs de YouTube, la visualización del label "descargando el audio…" con `aria-live="polite"`, la retención de `staged_audio_id` en `chat-state.js` solo en memoria efímera (cero `localStorage`), su inclusión en la siguiente llamada API, y su limpieza tras agregar pista o ante comandos no relacionados. <!-- sdd-owner: implementation -->
- [ ] 4.4 GREEN: Implementar las micro-utilidades puras `frontend/js/utils/validation/contains-youtube-url.js` e `is-staged-audio-unrelated.js`, actualizar la molécula `frontend/js/components/molecules/agent-typing-indicator.js` para recibir `label` dinámico, actualizar el panel en `frontend/js/components/organisms/agent-chat-panel.js`, extender `frontend/js/core/state/chat-state.js` con el ciclo de vida de `stagedAudioId`, y cablear el transporte en `frontend/js/data/agent-chat-api.js` y `frontend/js/controllers/agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [ ] 4.5 TRIANGULATE: Añadir pruebas de integración en frontend verificando que errores de red preservan el `stagedAudioId` para reintentos, que desmontar el panel restaura el composer y elimina el indicador de actividad, y que mensajes con hosts parecidos (`notyoutube.com`) conservan el label general "Shirley está pensando...". <!-- sdd-owner: implementation -->
- [ ] 4.6 REFACTOR: Ejecutar la suite completa de pruebas con `cargo test` (incluyendo los harnesses Node de chat UI) para verificar que no existen regresiones en subida manual de audio, streaming con Range HTTP, CRUD de presentaciones ni Stage Mode. <!-- sdd-owner: implementation -->
```
