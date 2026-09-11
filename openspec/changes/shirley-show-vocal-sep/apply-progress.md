# Apply Progress: Separación vocal conversacional con Ressy (`shirley-show-vocal-sep`)

## Structured Status Snapshot

```json
{
  "changeName": "shirley-show-vocal-sep",
  "slice": "vsep-s1-domain-dsp",
  "executionMode": "auto",
  "artifactStore": "openspec",
  "reviewBudgetLines": 400,
  "chainStrategy": "stacked-to-main",
  "workUnit": "vsep-s1-domain-dsp",
  "token": "sha256:cf7445031447c27d3ea1ef0433790707680828269e558c8dbc99c694e2a566ac",
  "request": "v1-acquire-001",
  "completedSliceTasks": 4,
  "totalSliceTasks": 4,
  "remainingChangeTasks": 12
}
```

## Slice 1 Implementation Summary (`vsep-s1-domain-dsp`)

Slice 1 implements the domain core and pure DSP audio modules for vocal separation:
1. `VocalSeparationMode`: Closed domain enum (`Coros`, `Instrumental`) with strict JSON serde (`"chorus"`, `"instrumental"`), title suffixes, and rejection of unknown strings or conversational aliases.
2. `avg_wave`: Pure streaming audio blender using `hound`. Opens 3 WAV inputs, enforces strict parity across sample rate, channels, format, and sample count, averages in lockstep with float32 weights `1:1:1`, clamps to `[-1.0, 1.0]`, writes deterministic 24-bit PCM WAV, and deterministically purges output on error without leaking filesystem paths.
3. `encode_mp3_320k`: Subprocess-safe audio transcode utility using `ffmpeg` (`-vn -codec:a libmp3lame -b:a 320k -map_metadata -1 -f mp3`) that validates output existence/size and purges output on error.

## Completed Tasks (Slice 1)

- [x] RED: Escribir pruebas unitarias en `tests/avg_wave_tests.rs` para `VocalSeparationMode` (parsing JSON `"chorus"`/`"instrumental"`, rechazo de alias/desconocidos) y `avg_wave` con fixtures sintéticos generados en memoria (promedio float de 3 WAVs mono/estéreo a 16-bit/24-bit/32-bit float, clamp ante picos +1.0/-1.0, rechazo de disparidad en sample rate/canales/longitud y purga del archivo de salida ante fallos). <!-- sdd-owner: implementation -->
- [x] GREEN: Agregar dependencia runtime `hound = "3.5"` a `Cargo.toml`, implementar `VocalSeparationMode` en `src/domain/vocal_separation_mode.rs`, y la función de promedio streaming `avg_wave` con errores tipados en `src/audio/avg_wave.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Incorporar pruebas de casos de borde en `tests/avg_wave_tests.rs` (silencio absoluto, WAVs vacíos de cero samples, valores extremos de cuantización a 24 bits, lectura en disco de archivos no existentes y fallo durante escritura con eliminación determinista de residuos). <!-- sdd-owner: implementation -->
- [x] REFACTOR: Pulir el bucle de iteración lockstep para garantizar consumo `O(1)` de memoria sin buffering en RAM de streams de 10 minutos, documentar tipos de error y registrar módulos en `src/lib.rs`. <!-- sdd-owner: implementation -->

## Strict TDD Cycle Evidence

| Phase | Target / Scope | Test Command / Action | Result |
|---|---|---|---|
| RED | `VocalSeparationMode`, `avg_wave`, `encode_mp3_320k` | `cargo test --test avg_wave_tests` | Failed with compilation errors (missing modules `audio`, `vocal_separation_mode`) as expected |
| GREEN | Added `hound = "3.5"`, created domain enum, `avg_wave`, `encode_mp3`, registered in `src/lib.rs` | `cargo test --test avg_wave_tests` | 12 tests passed |
| TRIANGULATE | Added edge cases: absolute silence, zero-sample empty WAVs, 24-bit quantization extremes, non-existent inputs, truncated stream mid-read, unsupported 8-bit format | `cargo test --test avg_wave_tests` | Caught 1 LSB roundoff bug on 24-bit max integer clamp; fixed scaling formula to symmetric `8_388_608.0` with `clamp(-8388608, 8388607)`. All 18 tests passed |
| REFACTOR | Added error type documentation, verified O(1) streaming iterators, validated zero path leakage in error messages | `cargo test && node --test tests/*.mjs` | Full test suite clean (all Rust integration and Node chat/wake-lock tests pass) |

## Files Changed

- `Cargo.toml` (added runtime dependency `hound = "3.5"`)
- `Cargo.lock`
- `src/domain/vocal_separation_mode.rs` (new module)
- `src/domain/mod.rs` (exported `VocalSeparationMode`, `VocalSeparationModeParseError`)
- `src/audio/avg_wave.rs` (new module)
- `src/audio/encode_mp3.rs` (new module)
- `src/audio/mod.rs` (new module)
- `src/lib.rs` (exported `pub mod audio;`)
- `tests/avg_wave_tests.rs` (new comprehensive test suite: 18 tests)
- `openspec/changes/shirley-show-vocal-sep/tasks.md` (marked S1 checkboxes `- [x]`)

## Deviations from Design

None. The implementation strictly adheres to `design.md`:
- `VocalSeparationMode` maps to `"chorus"` and `"instrumental"` via Serde and rejects all aliases.
- `avg_wave` implements `(a + b + c) / 3.0` with `clamp(-1.0, 1.0)`, streaming lockstep iteration, 24-bit PCM output, and deterministic deletion on failure.
- `encode_mp3_320k` invokes `ffmpeg` with `-vn -codec:a libmp3lame -b:a 320k -map_metadata -1 -f mp3` and purges output on failure.

## Remaining Tasks (Slices 2–4)

The following 12 implementation tasks remain for subsequent slices (PR2 to PR4):

- [ ] RED: Escribir pruebas herméticas en `tests/process_vocal_separator_tests.rs` utilizando binarios stub en directorios temporales (verificación de preflight `ffprobe` con rechazo >600s antes de GPU; ejecución secuencial de `gabox-invertido`, `frazer-becruily` y `anvuew` en Coros; resolución de stems excluyendo `(?i)vocal`; preflight de PolarFormer ausente devolviendo `instrumental_unavailable` sin pedir GPU; transcodificación a MP3 CBR 320 kbps; serialización con Semaphore(1); y cancelación que activa kill-and-wait y purga de workdir reteniendo el permiso hasta su terminación). <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar el trait `VocalSeparator` en `src/ports/vocal_separator.rs`, `VocalProcessGuard` con RAII y drop background en `src/infra/vocal/vocal_process_guard.rs`, `resolve_stem` en `src/infra/vocal/resolve_stem.rs`, `ProcessVocalSeparatorConfig` con resolución `VOCAL_SEP_BIN` en `src/infra/vocal/process_vocal_separator_config.rs`, `ProcessVocalSeparator` en `src/infra/vocal/process_vocal_separator.rs`, y el método `vocal_work_dir` en `LocalAudioStorage` junto con la purga de inicio en `reconcile_audio_files.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Agregar casos de prueba de fallo y contención en `tests/process_vocal_separator_tests.rs` (falla del segundo modelo en la cadena Coros con limpieza completa de salidas previas, error cuando `VOCAL_SEP_BIN` no existe o no es ejecutable, salida con bitrate MP3 distinto a 320 kbps o codec incorrecto, y dos tareas simultáneas donde la segunda espera sin solapar VRAM). <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Estructurar submódulos atómicos bajo `src/infra/vocal/`, asegurar que ningún comando invoque shells (`sh -c`) sino argumentos estructurados, y cablear el `Arc<Semaphore>::new(1)` en `AppState`. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir pruebas unitarias e integración en `tests/separate_vocals_tests.rs` y `tests/staged_audio_registry_tests.rs` con un fake `VocalSeparator` (separación exitosa desde `staged_audio_id`, desde `track_id` administrado sin modificar el archivo original, y desde `youtube_url` reutilizando `DownloadYouTubeAudio`; prioridad de fuentes; verificación de `commit_processed` que elimina el archivo staged fuente y retiene tombstone single-use; verificación de que ante fallo del separator la fuente staged permanece disponible para reintento; registro del MP3 resultante con TTL de 15 minutos; y degradación amable ante PolarFormer ausente). <!-- sdd-owner: implementation -->
- [ ] GREEN: Implementar `StagedAudioLeaseGuard` en `src/state/staged_audio_lease_guard.rs`, añadir `reserve_guarded` y `commit_processed` a `src/state/staged_audio_registry.rs`, implementar el caso de uso `SeparateVocals` en `src/use_cases/separate_vocals.rs` coordinando staging, resolución de audio y almacenamiento, y registrarlo en `src/state/app_state.rs`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Añadir pruebas de robustez en `tests/separate_vocals_tests.rs` (intento de separar con fuente inexistente o ya consumida, fallo al descargar de YouTube antes de llegar a la separación, fuente que excede el límite de duración, y validación de que el audio resultante puede ser consumido por `UploadTrack`/`add_track_from_staged` para crear una pista canónica). <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Desacoplar la resolución de fuente en funciones privadas puras, validar que `SeparateVocals` siga el principio de 1 struct por caso de uso, y asegurar el aislamiento de las capas de dominio frente a detalles de I/O. <!-- sdd-owner: implementation -->
- [ ] RED: Escribir pruebas unitarias y de integración para backend y frontend (en `tests/agent_tools_tests.rs`, `tests/agent_chat_tests.rs` y `tests/agent-chat-state.test.mjs`) que verifiquen: esquema estricto de `separate_vocals`; rechazo en dispatcher/context si falta destino confirmado; rechazo si el modo es inválido; flujo conversacional en dos turnos donde Shirley pregunta modo y destino antes de ejecutar GPU; encadenamiento en el mismo turno cuando la presentación activa ya existe (`separate_vocals` seguido de `add_track_from_staged`); deduplicación de llamadas idénticas; deadline dinámico de hasta 600 segundos para separación; y en el frontend, activación del label "separando voces…" en el estado del chat al detectar intención vocal, preservación de `stagedAudioId` en memoria y restauración limpia del composer en éxito, error o timeout. <!-- sdd-owner: implementation -->
- [ ] GREEN: Declarar `separate_vocals` en `tool_schemas.rs`, despacharlo en `tool_dispatcher.rs`, proteger la precondición de destino confirmado en `turn_context.rs`, actualizar instrucciones en `system_prompt.rs`, implementar la política de deadline dinámico en `agent_chat.rs` y `config.rs`, e implementar `is-vocal-separation-intent.js`, la gestión de estado en `chat-state.js` y el controlador en `agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Añadir pruebas de integración completas (en `tests/agent_chat_api_tests.rs` e integración frontend) que verifiquen: turno con timeout a los 600s reportando acciones previas y liberando GPU; respuesta amable ante PolarFormer ausente sin agregar pista fantasma; llamada repetida ignorada sin segunda ejecución en GPU; desconexión de cliente abortando la petición; y persistencia en un solo paso hacia el show activo cuando todos los parámetros están presentes. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Revisar que no queden temporizadores genéricos en el controlador pisando el label "separando voces…", asegurar la micro-modularidad de las funciones frontend, verificar accesibilidad con `aria-live="polite"` y comprobar que la suite completa de tests de Rust y Node pasa sin regresiones. <!-- sdd-owner: implementation -->

## Workload & PR Boundary

- PR 1 (current): `vsep-s1-domain-dsp` — 316 production lines across 4 new files (`vocal_separation_mode.rs`, `avg_wave.rs`, `encode_mp3.rs`, `audio/mod.rs`), 601 test lines in `avg_wave_tests.rs`. Cohesive, self-contained unit under the authority token budget.
- PR 2: Slice 2 (Infraestructura CLI, `VocalProcessGuard`, Mutex GPU y Adapter `ProcessVocalSeparator`).
- PR 3: Slice 3 (Caso de uso `SeparateVocals`, Staging y `StagedAudioRegistry`).
- PR 4: Slice 4 (Tool `separate_vocals`, Prompt Ressy, Timeout 600s y Frontend).
