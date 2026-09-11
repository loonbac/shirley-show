# Apply Progress: Separación vocal conversacional con Ressy (`shirley-show-vocal-sep`)

## Structured Status Snapshot

```json
{
  "changeName": "shirley-show-vocal-sep",
  "slice": "vsep-s3-usecase-staging",
  "executionMode": "auto",
  "artifactStore": "openspec",
  "reviewBudgetLines": 400,
  "chainStrategy": "stacked-to-main",
  "workUnit": "vsep-s3-usecase-staging",
  "token": "sha256:23063d57207a78a28c3a5abac8ebf0a4f422cef50e79e6a820e211b0c10e52e0",
  "request": "v3-acquire-001",
  "completedSliceTasks": 4,
  "totalSliceTasks": 4,
  "completedChangeTasks": 12,
  "remainingChangeTasks": 4
}
```

## Slice 1 Implementation Summary (`vsep-s1-domain-dsp`)

Slice 1 implements the domain core and pure DSP audio modules for vocal separation:
1. `VocalSeparationMode`: Closed domain enum (`Coros`, `Instrumental`) with strict JSON serde (`"chorus"`, `"instrumental"`), title suffixes, and rejection of unknown strings or conversational aliases.
2. `avg_wave`: Pure streaming audio blender using `hound`. Opens 3 WAV inputs, enforces strict parity across sample rate, channels, format, and sample count, averages in lockstep with float32 weights `1:1:1`, clamps to `[-1.0, 1.0]`, writes deterministic 24-bit PCM WAV, and deterministically purges output on error without leaking filesystem paths.
3. `encode_mp3_320k`: Subprocess-safe audio transcode utility using `ffmpeg` (`-vn -codec:a libmp3lame -b:a 320k -map_metadata -1 -f mp3`) that validates output existence/size and purges output on error.

## Slice 2 Implementation Summary (`vsep-s2-infra-gpu`)

Slice 2 implements the abstract port, CLI process adapter, GPU serialization mutex, process lifecycle guard, stem resolver, and startup reconciliation:
1. `VocalSeparator`: Object-safe port defining `VocalSeparationRequest`, `SeparatedAudio`, and closed `VocalSeparationError` with public Spanish error messages and error codes.
2. `VocalProcessGuard`: RAII guard managing child processes, isolated working directories, and `OwnedSemaphorePermit`. On cancellation, timeout, or drop, a spawned runtime task kills and waits for child termination and purges disk artifacts, holding the GPU permit until the child has fully exited so VRAM is never contended.
3. `resolve_stem`: Strict stem resolver that inspects dedicated model subdirectories, rejects symlinks and unexpected files, validates model tokens in filenames, excludes `(?i)vocal` stems, and requires exactly one vocal and one non-vocal WAV file.
4. `ProcessVocalSeparatorConfig`: Resolves `VOCAL_SEP_BIN` strictly (failing if explicit path is non-executable without falling back to PATH), and validates PolarFormer checkpoint/config existence.
5. `ProcessVocalSeparator`: Full pipeline implementation:
   - Preflights duration via `ffprobe` structured JSON; rejects unreadable audio and audio > 600s before requesting GPU.
   - Preflights PolarFormer checkpoint/YAML; returns `instrumental_unavailable` before requesting GPU.
   - Serializes GPU access through shared `Arc<tokio::sync::Semaphore>(1)`.
   - Executes `Coros` pipeline (`gabox-invertido` -> `frazer-becruily` -> `anvuew` sequentially in isolated directory), combines stems via `avg_wave`, and encodes to MP3 CBR 320 kbps.
   - Executes `Instrumental` pipeline via single `polarformer` execution and encodes to MP3 CBR 320 kbps.
   - Verifies post-transcode artifact: non-zero size, MP3 magic bytes (`ID3` or sync word `0xFF 0xFB`), and second `ffprobe` verification of `codec_name == "mp3"` and reported bitrate `320000`.
6. `LocalAudioStorage` & `reconcile_audio_files`: Added `vocal_work_dir` (`<audio>/vocal-work`) and startup reconciliation purges orphan vocal work directories.
7. `AppState`: Wired `gpu_semaphore: Arc<Semaphore>` and injected `vocal_separator: Arc<dyn VocalSeparator>` with `with_vocal_separator` override method.

## Slice 3 Implementation Summary (`vsep-s3-usecase-staging`)

Slice 3 implements the use case orchestration, staging lifecycle extension, and RAII lease guard:
1. `StagedAudioLeaseGuard`: RAII lease wrapper that automatically calls `registry.release` upon drop unless committed via `commit_consumed`, `commit_processed`, or `defuse`.
2. `StagedAudioRegistry` extensions:
   - `reserve_guarded`: reserves an available staged audio and returns `StagedAudioLeaseGuard`.
   - `commit_processed`: marks the staged audio as `Consumed` (retaining tombstone) AND deletes the physical temporary file from disk.
3. `SeparateVocals` use case:
   - Resolves sources in strict priority order: `staged_audio_id` -> `track_id` -> `youtube_url` (or returns `source_required`).
   - For `staged_audio_id`: reserves with RAII lease guard. On success, `commit_processed` deletes the source file and consumes the handle; on failure, auto-release preserves the source file for retry.
   - For `track_id`: reads managed track item via `ShowItemReader` in read-only mode, resolving path in `LocalAudioStorage`. The source track file is never deleted or altered.
   - For `youtube_url`: reuses `DownloadYouTubeAudio` directly without duplicating downloader, staging, or limits. Reserves downloaded handle, and deletes it after separation via `commit_processed`.
   - Protects output staging file via `StagingFileGuard`.
   - Enforces title suffix (`" (Coros)"` or `" (Instrumental)"`) and registers output in `StagedAudioRegistry` with 15-minute TTL.
   - Validated integration with `UploadTrack` / `add_track_from_staged` to create canonical show tracks from separated outputs.
4. `AppState`: wired `separate_vocals: Arc<SeparateVocals<SqliteShowItemRepository>>` in `new_with_all_components`, `with_vocal_separator`, and `with_youtube_downloader`.

## Completed Tasks (Slices 1, 2 & 3)

### Slice 1 (`vsep-s1-domain-dsp`)
- [x] RED: Escribir pruebas unitarias en `tests/avg_wave_tests.rs` para `VocalSeparationMode` (parsing JSON `"chorus"`/`"instrumental"`, rechazo de alias/desconocidos) y `avg_wave` con fixtures sintéticos generados en memoria (promedio float de 3 WAVs mono/estéreo a 16-bit/24-bit/32-bit float, clamp ante picos +1.0/-1.0, rechazo de disparidad en sample rate/canales/longitud y purga del archivo de salida ante fallos). <!-- sdd-owner: implementation -->
- [x] GREEN: Agregar dependencia runtime `hound = "3.5"` a `Cargo.toml`, implementar `VocalSeparationMode` en `src/domain/vocal_separation_mode.rs`, y la función de promedio streaming `avg_wave` con errores tipados en `src/audio/avg_wave.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Incorporar pruebas de casos de borde en `tests/avg_wave_tests.rs` (silencio absoluto, WAVs vacíos de cero samples, valores extremos de cuantización a 24 bits, lectura en disco de archivos no existentes y fallo durante escritura con eliminación determinista de residuos). <!-- sdd-owner: implementation -->
- [x] REFACTOR: Pulir el bucle de iteración lockstep para garantizar consumo `O(1)` de memoria sin buffering en RAM de streams de 10 minutos, documentar tipos de error y registrar módulos en `src/lib.rs`. <!-- sdd-owner: implementation -->

### Slice 2 (`vsep-s2-infra-gpu`)
- [x] RED: Escribir pruebas herméticas en `tests/process_vocal_separator_tests.rs` utilizando binarios stub en directorios temporales (verificación de preflight `ffprobe` con rechazo >600s antes de GPU; ejecución secuencial de `gabox-invertido`, `frazer-becruily` y `anvuew` en Coros; resolución de stems excluyendo `(?i)vocal`; preflight de PolarFormer ausente devolviendo `instrumental_unavailable` sin pedir GPU; transcodificación a MP3 CBR 320 kbps; serialización con Semaphore(1); y cancelación que activa kill-and-wait y purga de workdir reteniendo el permiso hasta su terminación). <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar el trait `VocalSeparator` en `src/ports/vocal_separator.rs`, `VocalProcessGuard` con RAII y drop background en `src/infra/vocal/vocal_process_guard.rs`, `resolve_stem` en `src/infra/vocal/resolve_stem.rs`, `ProcessVocalSeparatorConfig` con resolución `VOCAL_SEP_BIN` en `src/infra/vocal/process_vocal_separator_config.rs`, `ProcessVocalSeparator` en `src/infra/vocal/process_vocal_separator.rs`, y el método `vocal_work_dir` en `LocalAudioStorage` junto con la purga de inicio en `reconcile_audio_files.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Agregar casos de prueba de fallo y contención en `tests/process_vocal_separator_tests.rs` (falla del segundo modelo en la cadena Coros con limpieza completa de salidas previas, error cuando `VOCAL_SEP_BIN` no existe o no es ejecutable, salida con bitrate MP3 distinto a 320 kbps o codec incorrecto, y dos tareas simultáneas donde la segunda espera sin solapar VRAM). <!-- sdd-owner: implementation -->
- [x] REFACTOR: Estructurar submódulos atómicos bajo `src/infra/vocal/`, asegurar que ningún comando invoque shells (`sh -c`) sino argumentos estructurados, y cablear el `Arc<Semaphore>::new(1)` en `AppState`. <!-- sdd-owner: implementation -->

### Slice 3 (`vsep-s3-usecase-staging`)
- [x] RED: Escribir pruebas unitarias e integración en `tests/separate_vocals_tests.rs` y `tests/staged_audio_registry_tests.rs` con un fake `VocalSeparator` (separación exitosa desde `staged_audio_id`, desde `track_id` administrado sin modificar el archivo original, y desde `youtube_url` reutilizando `DownloadYouTubeAudio`; prioridad de fuentes; verificación de `commit_processed` que elimina el archivo staged fuente y retiene tombstone single-use; verificación de que ante fallo del separator la fuente staged permanece disponible para reintento; registro del MP3 resultante con TTL de 15 minutos; y degradación amable ante PolarFormer ausente). <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `StagedAudioLeaseGuard` en `src/state/staged_audio_lease_guard.rs`, añadir `reserve_guarded` y `commit_processed` a `src/state/staged_audio_registry.rs`, implementar el caso de uso `SeparateVocals` en `src/use_cases/separate_vocals.rs` coordinando staging, resolución de audio y almacenamiento, y registrarlo en `src/state/app_state.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Añadir pruebas de robustez en `tests/separate_vocals_tests.rs` (intento de separar con fuente inexistente o ya consumida, fallo al descargar de YouTube antes de llegar a la separación, fuente que excede el límite de duración, y validación de que el audio resultante puede ser consumido por `UploadTrack`/`add_track_from_staged` para crear una pista canónica). <!-- sdd-owner: implementation -->
- [x] REFACTOR: Desacoplar la resolución de fuente en funciones privadas puras, validar que `SeparateVocals` siga el principio de 1 struct por caso de uso, y asegurar el aislamiento de las capas de dominio frente a detalles de I/O. <!-- sdd-owner: implementation -->

## Strict TDD Cycle Evidence

| Slice | Phase | Target / Scope | Test Command / Action | Result |
|---|---|---|---|---|
| S1 | RED | `VocalSeparationMode`, `avg_wave`, `encode_mp3_320k` | `cargo test --test avg_wave_tests` | Failed with compilation errors (missing modules) as expected |
| S1 | GREEN | Added `hound = "3.5"`, domain enum, `avg_wave`, `encode_mp3`, registered in `src/lib.rs` | `cargo test --test avg_wave_tests` | 12 tests passed |
| S1 | TRIANGULATE | Edge cases: absolute silence, zero-sample empty WAVs, 24-bit quantization extremes, non-existent inputs, truncated stream | `cargo test --test avg_wave_tests` | Caught 1 LSB roundoff bug on 24-bit max clamp; fixed scaling formula. All 18 tests passed |
| S1 | REFACTOR | Documentation, O(1) streaming verification, zero path leakage validation | `cargo test && node --test tests/*.mjs` | Full test suite clean |
| S2 | RED | `VocalSeparator` port, `ProcessVocalSeparator`, `VocalProcessGuard`, preflights, stem resolution | `cargo test --test process_vocal_separator_tests` | Failed with compilation errors (missing `ports::vocal_separator`, `infra::vocal`) as expected |
| S2 | GREEN | Implemented port, guard, config, resolver, `ProcessVocalSeparator`, `LocalAudioStorage.vocal_work_dir`, `reconcile_audio_files` | `cargo test --test process_vocal_separator_tests` | 6 tests passed |
| S2 | TRIANGULATE | Added failure & contention cases: step 2 failure cleans intermediate files, non-existent/non-exec `VOCAL_SEP_BIN`, post-verify bad bitrate/codec, concurrent serialization with Semaphore(1) | `cargo test --test process_vocal_separator_tests` | Caught Linux ETXTBSY race on fast script creation; resolved with `file.sync_all()` + `drop(file)` + bounded retry. All 10 tests passed |
| S2 | REFACTOR | Atomized submodules under `src/infra/vocal/`, structured args without shell evaluation, wired `gpu_semaphore` in `AppState` | `cargo test && node --test tests/*.mjs` | All 162 Rust tests and 18 Node tests passed cleanly |
| S3 | RED | `StagedAudioLeaseGuard`, `reserve_guarded`, `commit_processed`, `SeparateVocals`, fake separator tests | `cargo test --test staged_audio_registry_tests` & `cargo test --test separate_vocals_tests` | Failed with expected compilation errors (missing methods and use case module) |
| S3 | GREEN | Implemented `StagedAudioLeaseGuard`, added `reserve_guarded` and `commit_processed` to `StagedAudioRegistry`, implemented `SeparateVocals`, wired in `AppState` | `cargo test --test staged_audio_registry_tests` & `cargo test --test separate_vocals_tests` | 11 registry tests passed; 6 separate vocals tests passed |
| S3 | TRIANGULATE | Added robustness tests: nonexistent/already consumed staged audio, nonexistent track, non-track item, youtube download failure aborts before separator, duration exceeded preserves source, output consumed by `UploadTrack` into canonical show track | `cargo test --test separate_vocals_tests` | All 13 separate vocals tests passed |
| S3 | REFACTOR | Decoupled source resolution into private helpers (`resolve_staged_source`, `resolve_track_source`, `resolve_youtube_source`), validated 1-struct use case adherence, checked zero regressions | `cargo test && node --test tests/*.mjs` | All 178 Rust tests and 18 Node tests passed cleanly |

## Files Changed (Slice 3)

- `src/state/staged_audio_lease_guard.rs` (new module: RAII lease guard with auto-release on drop, `commit_consumed`, `commit_processed`, and `defuse`)
- `src/state/staged_audio_registry.rs` (added `reserve_guarded` returning `StagedAudioLeaseGuard`, and `commit_processed` deleting source file from disk while retaining tombstone)
- `src/state/mod.rs` (exported `StagedAudioLeaseGuard`)
- `src/use_cases/separate_vocals.rs` (new use case: `SeparateVocals`, `VocalAudioSource`, `SeparateVocalsInput`, `SeparateVocalsOutput`, `SeparateVocalsError`, decoupled source resolvers, staging management)
- `src/use_cases/mod.rs` (exported `SeparateVocals` and types)
- `src/state/app_state.rs` (wired `separate_vocals: Arc<SeparateVocals<SqliteShowItemRepository>>`, updated `with_vocal_separator` and `with_youtube_downloader`)
- `tests/staged_audio_registry_tests.rs` (added tests for `reserve_guarded` auto-release on drop, `commit_processed` file deletion and tombstone retention)
- `tests/separate_vocals_tests.rs` (new test suite: 13 tests covering staged, track, youtube sources, priority, failures, degradation, and canonical promotion via `UploadTrack`)
- `openspec/changes/shirley-show-vocal-sep/tasks.md` (marked S3 checkboxes `- [x]`)

## Deviations from Design

None. The implementation strictly adheres to `design.md`:
- Source resolution strictly prioritizes `staged_audio_id` -> `track_id` -> `youtube_url`.
- `track_id` reads managed track items in read-only mode and never mutates or deletes the managed file.
- Temporary staged sources are deleted via `commit_processed` only upon successful registration of the result.
- Separator failure preserves the source staged file and its registry entry for subsequent retries.
- Resulting MP3 is registered in `StagedAudioRegistry` with 15-minute TTL and title suffix.

## Remaining Tasks (Slice 4)

The following 4 implementation tasks remain for Slice 4 (PR4):

- [ ] RED: Escribir pruebas unitarias y de integración para backend y frontend (en `tests/agent_tools_tests.rs`, `tests/agent_chat_tests.rs` y `tests/agent-chat-state.test.mjs`) que verifiquen: esquema estricto de `separate_vocals`; rechazo en dispatcher/context si falta destino confirmado; rechazo si el modo es inválido; flujo conversacional en dos turnos donde Shirley pregunta modo y destino antes de ejecutar GPU; encadenamiento en el mismo turno cuando la presentación activa ya existe (`separate_vocals` seguido de `add_track_from_staged`); deduplicación de llamadas idénticas; deadline dinámico de hasta 600 segundos para separación; y en el frontend, activación del label "separando voces…" en el estado del chat al detectar intención vocal, preservación de `stagedAudioId` en memoria y restauración limpia del composer en éxito, error o timeout. <!-- sdd-owner: implementation -->
- [ ] GREEN: Declarar `separate_vocals` en `tool_schemas.rs`, despacharlo en `tool_dispatcher.rs`, proteger la precondición de destino confirmado en `turn_context.rs`, actualizar instrucciones en `system_prompt.rs`, implementar la política de deadline dinámico en `agent_chat.rs` y `config.rs`, e implementar `is-vocal-separation-intent.js`, la gestión de estado en `chat-state.js` y el controlador en `agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [ ] TRIANGULATE: Añadir pruebas de integración completas (en `tests/agent_chat_api_tests.rs` e integración frontend) que verifiquen: turno con timeout a los 600s reportando acciones previas y liberando GPU; respuesta amable ante PolarFormer ausente sin agregar pista fantasma; llamada repetida ignorada sin segunda ejecución en GPU; desconexión de cliente abortando la petición; y persistencia en un solo paso hacia el show activo cuando todos los parámetros están presentes. <!-- sdd-owner: implementation -->
- [ ] REFACTOR: Revisar que no queden temporizadores genéricos en el controlador pisando el label "separando voces…", asegurar la micro-modularidad de las funciones frontend, verificar accesibilidad con `aria-live="polite"` y comprobar que la suite completa de tests de Rust y Node pasa sin regresiones. <!-- sdd-owner: implementation -->

## Workload & PR Boundary

- PR 1: `vsep-s1-domain-dsp` (merged/committed) — `VocalSeparationMode`, `avg_wave`, `encode_mp3_320k`.
- PR 2: `vsep-s2-infra-gpu` (merged/committed) — `VocalSeparator` port, `ProcessVocalSeparator`, `VocalProcessGuard`, `resolve_stem`, `ProcessVocalSeparatorConfig`, `LocalAudioStorage.vocal_work_dir`, `reconcile_audio_files`, `AppState.gpu_semaphore`.
- PR 3 (current slice): `vsep-s3-usecase-staging` — `SeparateVocals` use case, `StagedAudioLeaseGuard`, `StagedAudioRegistry` (`reserve_guarded`, `commit_processed`), `AppState.separate_vocals`. Cohesive and isolated use case and staging layer.
- PR 4: Slice 4 (Tool `separate_vocals`, Prompt Ressy, Timeout 600s y Frontend).
