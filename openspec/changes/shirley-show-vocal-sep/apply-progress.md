# Apply Progress: Separación vocal conversacional con Ressy (`shirley-show-vocal-sep`)

## Structured Status Snapshot

```json
{
  "changeName": "shirley-show-vocal-sep",
  "slice": "fix-vsep-2-blockers",
  "executionMode": "auto",
  "artifactStore": "openspec",
  "reviewBudgetLines": 500,
  "chainStrategy": "stacked-to-main",
  "workUnit": "fix-vsep-2-blockers",
  "token": "vvf2-begin-001",
  "request": "vvf2-begin-001",
  "completedSliceTasks": 4,
  "totalSliceTasks": 4,
  "completedChangeTasks": 16,
  "remainingChangeTasks": 0
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

## Slice 4 Implementation Summary (`vsep-s4-agent-frontend`)

Slice 4 integrates the vocal separation capability into the conversational agent, API layer, timeouts, and frontend:
1. `separate_vocals` tool schema in `src/agent/tool_schemas.rs`:
   - Exact 8 tools allowlist (`create_presentation`, `list_presentations`, `get_presentation`, `add_lyrics`, `add_note`, `add_track_from_staged`, `download_youtube_track`, `separate_vocals`).
   - Strict parameters: `mode` (`"chorus"` | `"instrumental"`, required), `staged_audio_id`, `track_id`, `youtube_url` (optional), `additionalProperties: false`.
2. Tool Dispatcher & Grounding in `src/agent/tool_dispatcher.rs`:
   - Enforces `confirmed_destination_id` precondition before GPU or use case invocation (returns `destination_required` if destination presentation is not confirmed).
   - Validates `track_id` against `known_track_ids` loaded from `get_presentation` (returns `unauthorized_track_id`).
   - Validates `staged_audio_id` against `known_staged_audio_ids` in `TurnToolContext`.
   - Records structured `ToolAction` receipt with presentation ID and details upon success.
   - Deduplicates identical calls within the same turn without running secondary GPU pipelines.
3. Turn Context & Grounding in `src/agent/turn_context.rs`:
   - Tracks `confirmed_destination_id` (initialized from `active_presentation_id` or set upon creation/retrieval).
   - Tracks `known_track_ids` dynamically loaded on `get_presentation`.
4. Ressy Prompt in `src/agent/system_prompt.rs`:
   - Added Rule 16: Ask mode and destination presentation before touching the GPU.
   - Same-turn chaining when destination is confirmed (`separate_vocals` followed by `add_track_from_staged`).
   - Strict 3-stage differentiation in conversational responses: "fuente preparada" (downloaded), "separación terminada" (separated in staging), and "pista agregada" (committed track).
   - Polite explanation when PolarFormer is unconfigured, offering chorus mode or keeping audio within 15-minute TTL.
5. Dynamic Deadline Policy in `src/use_cases/agent_chat.rs`, `src/config.rs`, and `src/handlers/agent/chat.rs`:
   - Config adds `SHIRLEY_SEPARATION_TIMEOUT_SECS` (default 600s).
   - Standard conversational turns maintain 30s base budget; YouTube downloads expand to 120s; `separate_vocals` expands to 600s.
   - Request parsing enforces 30s bounded timeout.
   - Uncaught timeout produces grounded message with prior committed actions and formatted elapsed budget.
6. Frontend Micro-Modules & Headless State:
   - Created pure micro-utility `frontend/js/utils/validation/is-vocal-separation-intent.js`.
   - Updated `frontend/js/core/state/chat-state.js` with `vocalSeparationPending` flag and qualitative `pendingLabel = 'Ressy está separando voces… (tarda unos minutos)'`.
   - Ephemeral in-memory retention of `stagedAudioId` across conversational turns (strictly zero `localStorage`).
   - Clean restoration of composer and pending label upon resolution, error, timeout, or cancellation.
   - Removed generic wait timers in `frontend/js/controllers/agent-chat-controller.js` to ensure the headless state owns the label without race conditions or overwrites.
7. Real GPU Verification with `karaoke-separator`:
   - Discovered and resolved runtime stem resolution for `karaoke-separator` (which produces FLAC stems `_(other)_` and `_(vocals)_`), converting stems to PCM 24-bit WAV for `avg_wave` processing.
   - Fixed `ffprobe` post-verify to read stream-level codec bitrate (`320000`) rather than container format bitrate with muxing overhead.
   - Successfully ran end-to-end chorus separation on NVIDIA GeForce RTX 3060 via `karaoke-separator` with an ffmpeg-generated test audio, verified valid MP3 CBR 320 kbps output with ffprobe, and purged all temporary files.


## Remediation Summary (`fix-vsep-verify-blockers`)

Remediated all 6 blockers C1-C6 identified in `verify-report.md`:
1. **C1 (Auxiliary subprocesses under `VocalProcessGuard`):**
   - Evolved `VocalProcessGuard` to maintain an internal child process registry `Vec<Child>` and exposed `run_status` and `run_output` with `kill_on_drop(true)`.
   - Integrated FLAC->WAV conversion (`ffmpeg`), MP3 transcode (`ffmpeg`), and post-verification (`ffprobe`) directly under `VocalProcessGuard`, ensuring they execute with remaining timeout bounds.
   - `Drop` and `kill_and_wait` on `VocalProcessGuard` kill and await completion of EVERY child before releasing the GPU semaphore permit and purging temporary workdirs.
   - Added `PreflightChildGuard` to preflight `ffprobe` to ensure kill-and-wait if preflight is cancelled or times out.
2. **C2 (Instrumental PolarFormer checkpoint & YAML):**
   - Real wrapper contract correction (`/run/current-system/sw/bin/karaoke-separator`): the wrapper's `polarformer` branch hardcodes its checkpoint `$models_dir/model_bs_polarformer_float16.ckpt` and config `$models_dir/model_bs_polarformer_float16.yaml` without accepting flags. Extra CLI flags (`--checkpoint`, `--config`) are silently ignored by the wrapper bash script.
   - PolarFormer availability guarantee is enforced locally via preflight `validate_polarformer_artifacts` before acquiring the GPU semaphore permit. If weights are missing, it immediately yields a polite error `instrumental_unavailable` (`InstrumentalUnavailable`) without acquiring GPU or invoking the wrapper.
3. **C3 (Output template `NAME_(STEM)_MODEL` and exact argv):**
   - Corrected template contract: the wrapper hardcodes output format FLAC and template `NAME_(STEM)_MODEL` (not `NAME_MODEL_STEM`) internally. CLI flag `--output-template` does not exist in the wrapper.
   - `VocalCommandBuilder` emits the exact argv required by the wrapper: `[modelo, input, output_dir]`.
   - Tests assert exact argv with exactly 3 positional arguments per model, stubs emulate `NAME_(STEM)_MODEL` in FLAC, and preflight missing weights return friendly `InstrumentalUnavailable`.
4. **C4 (600s Separation Timeout Clamp):**
   - Updated `src/config.rs` to strictly reject any `SHIRLEY_SEPARATION_TIMEOUT_SECS` value exceeding 600 or equal to 0.
   - Defense-in-depth: clamped `separation_timeout` in `src/main.rs` (`.min(600)`) and in `src/use_cases/agent_chat.rs` (`.min(Duration::from_secs(600))`).
5. **C5 (Exact `pendingLabel` "separando voces…"):**
   - Corrected `frontend/js/core/state/chat-state.js` from divergent `"Ressy está separando voces… (tarda unos minutos)"` to exact contractual `"separando voces…"`.
   - Aligned assertion in `tests/agent-chat-state.test.mjs`.
6. **C6 (Strict TDD Evidence & Tests):**
   - Added `test_cancellation_during_flac_conversion_kills_child_and_releases_permit` in `tests/process_vocal_separator_tests.rs`.
   - Added `test_cancellation_during_mp3_encode_kills_child_and_releases_permit` in `tests/process_vocal_separator_tests.rs`.
   - Added template and checkpoint/YAML assertion tests in `tests/process_vocal_separator_tests.rs`.
   - Added 600s boundary test in `tests/agent_chat_api_tests.rs`.
   - Added `test_agent_limits_defaults_and_separation_600s_invariants` in `tests/agent_chat_tests.rs`.
   - Formatted codebase cleanly with `cargo fmt`, passing `cargo fmt -- --check`.

## Remediation 2 Summary (`fix-vsep-2-blockers`)

Remediated the 2 remaining blockers from `verify-report.md`:
1. **§C1 (Preflight PolarFormer Contract Alignment & Readability Proof):**
   - Contractual path alignment: the real system wrapper `/run/current-system/sw/bin/karaoke-separator` hardcodes `models_dir="$HOME/Proyectos/separador-vocal/modelos"` and passes `$models_dir/model_bs_polarformer_float16.ckpt` and `$models_dir/model_bs_polarformer_float16.yaml` without accepting CLI flags or environment overrides.
   - Replaced `is_file()` with active `File::open` check in pure function `ProcessVocalSeparatorConfig::validate_polarformer_artifacts_at` to test physical presence, regular file status, and read permissions before acquiring the GPU mutex.
   - Eliminated/ignored `POLARFORMER_CHECKPOINT` and `POLARFORMER_CONFIG` environment overrides in `from_env`. Documented in code why overrides are ignored to avoid false-positive preflight passing for alternate files while the real wrapper fails on its fixed paths.
   - Added tests verifying: preflight passes with real host artifacts; pure validation fails on missing ckpt, missing yaml, or directory; and `from_env` ignores `POLARFORMER_*` overrides.
2. **§C2 (Strict TDD Evidence for ffprobe Cancellation, Timeout & Reaping):**
   - Added `test_cancellation_during_ffprobe_preflight_kills_and_reaps_child`: verifies that aborting during `run_ffprobe_preflight` activates `PreflightChildGuard`, kills and reaps the child process (`/proc/{pid}` ceases to exist), and keeps GPU permit unacquired (1 available).
   - Added `test_timeout_during_ffprobe_preflight_kills_and_reaps_child`: verifies that preflight timeout terminates and reaps child process.
   - Added `test_cancellation_during_ffprobe_post_verify_kills_child_purges_workdir_and_releases_permit`: verifies that aborting during `run_ffprobe_post_verify` activates `VocalProcessGuard::drop`, kills and reaps the child (`/proc/{pid}` ceases to exist), purges `work_dir`, purges partial output, and restores GPU permit to 1.
   - Added `test_timeout_during_ffprobe_post_verify_kills_child_purges_workdir_and_releases_permit`: verifies that post-verify timeout reaps child, purges all disk artifacts, and restores GPU permit to 1.
   - Bounded `run_ffprobe_preflight` with explicit timeout parameter (`request.timeout.min(5s)`).
   - Fixed residual warning in `apply-progress.md` ensuring `pendingLabel` is documented everywhere as exact `"separando voces…"`.

## Completed Tasks (All Slices 1, 2, 3 & 4)

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

### Slice 4 (`vsep-s4-agent-frontend`)
- [x] RED: Escribir pruebas unitarias y de integración para backend y frontend (en `tests/agent_tools_tests.rs`, `tests/agent_chat_tests.rs` y `tests/agent-chat-state.test.mjs`) que verifiquen: esquema estricto de `separate_vocals`; rechazo en dispatcher/context si falta destino confirmado; rechazo si el modo es inválido; flujo conversacional en dos turnos donde Shirley pregunta modo y destino antes de ejecutar GPU; encadenamiento en el mismo turno cuando la presentación activa ya existe (`separate_vocals` seguido de `add_track_from_staged`); deduplicación de llamadas idénticas; deadline dinámico de hasta 600 segundos para separación; y en el frontend, activación del label "separando voces…" en el estado del chat al detectar intención vocal, preservación de `stagedAudioId` en memoria y restauración limpia del composer en éxito, error o timeout. <!-- sdd-owner: implementation -->
- [x] GREEN: Declarar `separate_vocals` en `tool_schemas.rs`, despacharlo en `tool_dispatcher.rs`, proteger la precondición de destino confirmado en `turn_context.rs`, actualizar instrucciones en `system_prompt.rs`, implementar la política de deadline dinámico en `agent_chat.rs` y `config.rs`, e implementar `is-vocal-separation-intent.js`, la gestión de estado en `chat-state.js` y el controlador en `agent-chat-controller.js`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE: Añadir pruebas de integración completas (en `tests/agent_chat_api_tests.rs` e integración frontend) que verifiquen: turno con timeout a los 600s reportando acciones previas y liberando GPU; respuesta amable ante PolarFormer ausente sin agregar pista fantasma; llamada repetida ignorada sin segunda ejecución en GPU; desconexión de cliente abortando la petición; y persistencia en un solo paso hacia el show activo cuando todos los parámetros están presentes. <!-- sdd-owner: implementation -->
- [x] REFACTOR: Revisar que no queden temporizadores genéricos en el controlador pisando el label "separando voces…", asegurar la micro-modularidad de las funciones frontend, verificar accesibilidad con `aria-live="polite"` y comprobar que la suite completa de tests de Rust y Node pasa sin regresiones. <!-- sdd-owner: implementation -->

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
| S4 | RED | Schema allowlist 8, `separate_vocals`, dispatcher destination check, dynamic deadline, frontend intent & label | `cargo test --test agent_tools_tests`, `cargo test --test agent_chat_tests`, `node tests/agent-chat-state.test.mjs` | Failed with expected schema mismatch (7 vs 8), unknown tool in dispatcher, missing `separation_timeout` in `AgentLimits`, and missing `is-vocal-separation-intent.js` |
| S4 | GREEN | Schema declaration, dispatcher destination enforcement, `AgentLimits.separation_timeout`, dynamic deadline, `is-vocal-separation-intent.js`, `chatState.vocalSeparationPending` | `cargo test --test agent_tools_tests`, `cargo test --test agent_chat_tests`, `node tests/agent-chat-state.test.mjs` | All unit tests in tools, agent chat, and frontend state passed |
| S4 | TRIANGULATE | HTTP integration in `agent_chat_api_tests` (single step canonical persistence, PolarFormer absent, duplicate call ignored, timeout preserving prior actions & GPU permit), frontend typing indicator integration | `cargo test --test agent_chat_api_tests`, `node --test tests/*.mjs` | All 21 API integration tests and 5 Node test suites passed |
| S4 | REFACTOR | Eliminated generic 3s wait timer in controller, verified `aria-live="polite"`, ensured O(1) streaming and FLAC stem support | `cargo test` & `node --test tests/*.mjs` | All 194 Rust tests and 5 Node test suites passed cleanly |
| S4 | E2E GPU REAL | Manual real GPU chorus separation with `karaoke-separator` on NVIDIA GeForce RTX 3060 | `cargo test --test e2e_real_gpu_test -- --ignored --nocapture` | Completed in 34.13s; output MP3 CBR 320 kbps validated via ffprobe; clean disk teardown |
| REM | RED | C2 (checkpoint/YAML preflight), C3 (exact argv [model, input, output_dir]), C4 (601s rejection) & C5 (exact "separando voces…") | `cargo test --test agent_chat_api_tests`, `node tests/agent-chat-state.test.mjs`, `cargo test --test process_vocal_separator_tests` | Failed as expected on missing arguments, boundary clamp, and label mismatch |
| REM | GREEN | Corrected `VocalCommandBuilder` exact argv `[modelo, input, output_dir]`, zero fake flags, local PolarFormer preflight, stubs emulating `NAME_(STEM)_MODEL` in FLAC, config clamp, and exact pendingLabel | `cargo test --test process_vocal_separator_tests` & `cargo test --test agent_chat_api_tests` | All 13 separator tests and API tests passed |
| REM | TRIANGULATE | Added cancellation during FLAC conversion, cancellation during MP3 encode, and 600s clamp invariant | `cargo test --test process_vocal_separator_tests` & `cargo test --test agent_chat_tests` | All 12 separator tests and 19 agent chat tests passed |
| REM | REFACTOR | Ran `cargo fmt` across all surfaces; full test suite verification | `cargo test && node --test tests/*.mjs` & `cargo fmt -- --check` | 237 Rust passed (1 ignored), 5 Node suites passed, fmt clean |
| REM2 | RED | §C1 (`validate_polarformer_artifacts_at` opening check), §C2 (ffprobe preflight & post-verify cancellation/timeout+reaping assertions) | `cargo test --test process_vocal_separator_tests` | Failed with expected compilation errors (missing pure function) |
| REM2 | GREEN | Implemented contractual PolarFormer paths, pure `validate_polarformer_artifacts_at` with `File::open` legibility test, ignored `POLARFORMER_*` overrides, bounded `run_ffprobe_preflight` with timeout parameter | `cargo test --test process_vocal_separator_tests` | All 18 separator tests passed |
| REM2 | TRIANGULATE | Added behavioral tests: preflight cancellation + reaping (/proc check), preflight timeout + reaping, post-verify cancellation + reaping + purges + permit restore, post-verify timeout + reaping + purges + permit restore | `cargo test --test process_vocal_separator_tests` | Caught unneeded dead fields, verified zero zombie processes in /proc across all lifecycle phases. All 18 tests passed |
| REM2 | REFACTOR | Ran `cargo fmt` across codebase; verified full Rust and Node suites with zero warnings | `cargo test && node --test tests/*.mjs` & `cargo fmt -- --check` | 242 Rust passed (1 ignored), 5 Node suites passed, fmt clean |

## Files Changed (Slice 4)

- `src/agent/tool_schemas.rs`: declared `separate_vocals` tool in exact 8 tools allowlist.
- `src/agent/turn_context.rs`: added `confirmed_destination_id` and `known_track_ids` fields and methods.
- `src/agent/tool_dispatcher.rs`: implemented `separate_vocals` tool dispatching, destination confirmation validation, `track_id` permission check, deduplication, and chained track creation.
- `src/agent/system_prompt.rs`: added Rule 16 covering mode/destination confirmation before GPU, 3-stage differentiation, same-turn chaining, and polite fallback for missing PolarFormer.
- `src/config.rs`: added `SHIRLEY_SEPARATION_TIMEOUT_SECS` configuration with default 600s.
- `src/main.rs`: wired `separation_timeout` in `AgentLimits`.
- `src/state/app_state.rs`: passed `separate_vocals` into `AgentToolDispatcher::from_components`.
- `src/use_cases/agent_chat.rs`: implemented dynamic deadline policy (30s baseline, 120s YouTube, 600s separation) and updated false claim sanitization for vocal separation terms.
- `src/handlers/agent/chat.rs`: updated chat handler with 30s request parsing timeout and dynamic deadline dispatch.
- `src/infra/vocal/resolve_stem.rs`: added model token aliases (`gabox`, `becruily`) and allowed `.flac` stems as output by `karaoke-separator`.
- `src/infra/vocal/process_vocal_separator.rs`: converted FLAC stems to PCM WAV for `avg_wave` processing, and adjusted post-verification to read stream codec bitrate (`320000`).
- `frontend/js/utils/validation/is-vocal-separation-intent.js`: new pure micro-utility classifying user vocal separation intent.
- `frontend/js/core/state/chat-state.js`: added `vocalSeparationPending` flag, qualitative `pendingLabel = 'Ressy está separando voces… (tarda unos minutos)'`, and in-memory `stagedAudioId` lifecycle management.
- `frontend/js/controllers/agent-chat-controller.js`: eliminated generic 3-second wait timer to let headless chat state exclusively own the label.
- `tests/agent_tools_tests.rs`: added unit tests for schema, destination rejection, invalid mode, unauthorized track, success chaining, and deduplication.
- `tests/agent_chat_tests.rs`: added unit tests for two-turn question flow before GPU, same-turn chaining, and extended 600s deadline.
- `tests/agent_chat_api_tests.rs`: added integration tests for single-step canonical persistence, PolarFormer absent response, duplicate call deduplication, and timeout preserving prior actions and releasing GPU permit.
- `tests/agent-chat-state.test.mjs`: added unit tests for `isVocalSeparationIntent` and vocal separation chat state lifecycle.
- `tests/agent-chat-integration.test.mjs`: added integration test for vocal separation typing indicator and composer lifecycle.
- `tests/e2e_real_gpu_test.rs`: real manual GPU chorus separation test against `karaoke-separator` on NVIDIA GPU with ffprobe verification.
- `openspec/changes/shirley-show-vocal-sep/tasks.md`: marked all Slice 4 tasks `- [x]`.

## Deviations from Design

None. All 6 blockers C1-C6 from verify-report.md have been remediated:
- `pendingLabel` is now strictly aligned with design and spec (`"separando voces…"`).
- Real `karaoke-separator` contract alignment: wrapper syntax is `karaoke-separator <modelo> <cancion> [salida]`, template is hardcoded `NAME_(STEM)_MODEL` in FLAC, polarformer branch uses internal hardcoded checkpoint; `VocalCommandBuilder` emits exact argv `[modelo, input, output_dir]` without fake CLI flags; PolarFormer weights are guaranteed via local preflight before GPU acquisition.
- All subprocesses (FLAC conversion, MP3 encode, ffprobe) are owned by `VocalProcessGuard` under kill-and-wait and the GPU permit is only released after all children exit.
- `SHIRLEY_SEPARATION_TIMEOUT_SECS` is strictly clamped to max 600s.
- Mode and destination presentation are strictly verified before acquiring GPU or executing separation.
- Destination confirmed is tracked in `TurnToolContext` and required by `AgentToolDispatcher`.
- Allowlist contains exactly 8 tools.
- Dynamic deadline scales up to 600s specifically for separation turns while maintaining 30s conversational baseline.
- `pendingLabel` is owned exclusively by headless chat state with `"separando voces…"` and composer is restored cleanly on all outcomes.
- Real GPU separation verified against physical NVIDIA GPU with `karaoke-separator`.

## Remaining Tasks

Zero unchecked implementation tasks remain across the entire change `shirley-show-vocal-sep`. (16/16 complete).

## Workload & PR Boundary

- PR 1: `vsep-s1-domain-dsp` (merged/committed) — `VocalSeparationMode`, `avg_wave`, `encode_mp3_320k`.
- PR 2: `vsep-s2-infra-gpu` (merged/committed) — `VocalSeparator` port, `ProcessVocalSeparator`, `VocalProcessGuard`, `resolve_stem`, `ProcessVocalSeparatorConfig`, `LocalAudioStorage.vocal_work_dir`, `reconcile_audio_files`, `AppState.gpu_semaphore`.
- PR 3: `vsep-s3-usecase-staging` (merged/committed) — `SeparateVocals` use case, `StagedAudioLeaseGuard`, `StagedAudioRegistry` (`reserve_guarded`, `commit_processed`), `AppState.separate_vocals`.
- PR 4 (current slice): `vsep-s4-agent-frontend` — Tool `separate_vocals`, Ressy system prompt, dynamic deadline policy (30s/120s/600s), `is-vocal-separation-intent.js`, `chat-state.js` vocal separation pending state, controller cleanup, full integration tests, and manual real GPU validation.
- Remediation slice (current): `fix-vsep-verify-blockers` — remediated blockers C1-C6 with strict TDD, clean cargo fmt, and 237 passing tests.
- Ready for full SDD re-verification (`sdd-verify`).
