```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:4123f27dc0a2712fc85323c06eccaad6946906dce80f7dfc096ecf628236fb31
verdict: fail
blockers: 2
critical_findings: 2
requirements: 8/10
scenarios: 52/55
test_command: cargo test && node --test tests/*.mjs
test_exit_code: 0
test_output_hash: sha256:d0ccd0a4e97c545fa4db523a514565281d74e0d85375ade0759e5302ab477c47
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:21aacdad5c16e73e83abf0d62570ab602d770a8dafb621029e2013d3e0711792
```

# Re-verify Report: `shirley-show-vocal-sep`

## Veredicto

**FAIL — no está listo para sync ni archive.** La remediación corrige el argv real del wrapper, el template FLAC, el límite de 600 segundos, el label exacto y el ownership de los procesos GPU/ffmpeg. Las suites enfocada y completa están verdes. Persisten dos bloqueadores críticos: el preflight PolarFormer puede validar artefactos distintos de los que usa el wrapper real y la evidencia strict-TDD no prueba cancelación durante `ffprobe`.

## Cobertura de especificación

Se recuperaron directamente los dos deltas del change: **10 requirements y 55 scenarios**. Para el contrato operativo del wrapper se aplicó la corrección autorizada por esta re-verificación: argv exacto `[modelo, cancion, salida]`, salida FLAC con template `NAME_(STEM)_MODEL`, y PolarFormer mediante preflight local.

| Requirement | Escenarios | Estado | Evidencia |
|---|---:|---|---|
| Ephemeral Staged Audio Registry with TTL Lifecycle | 5/5 | PASS | TTL 900 s, tombstones, retry, consumo único y cleanup pasan. |
| Conversational Vocal Separation Feedback and Client State Continuity | 5/5 | PASS | Label exacto `separando voces…`, estado efímero, `aria-live` y restauración pasan. |
| Agent Chat API Endpoint and Content Negotiation | 4/4 | PASS | Transporte y configuración de timeout hasta 600 s pasan. |
| Response Contract and Action Reporting | 5/5 | PASS | Staging, acciones y grounding pasan. |
| Vocal Separation Tool and GPU Execution Guardrails | 12/14 | FAIL | Chorus, argv/template, mutex, stems, MP3 y auxiliares bajo guard pasan; quedan preflight PolarFormer y cancelación de `ffprobe` sin garantía completa. |
| Fixed Tool Allowlist and Schema Definition | 4/4 | PASS | Ocho herramientas exactas y schema cerrado pasan. |
| Staged Track Creation Tool | 6/6 | PASS | Persistencia canónica, retry y single-use pasan. |
| Presentation Identifier Integrity and Grounded Context | 5/5 | PASS | Destino antes de GPU y chaining pasan. |
| Bounded Model Loop and Request Deadline | 4/5 | FAIL | Clamp dinámico pasa; la evidencia de cancelación no cubre `ffprobe`. |
| Duplicate Tool Call Prevention within Turn | 2/2 | PASS | Deduplicación evita una segunda separación. |

**Total:** requirements completos **8/10**; scenarios completos **52/55**.

Escenarios no demostrados completamente:

1. `Valid instrumental separation with PolarFormer produces 320k MP3` bajo configuración `POLARFORMER_*` divergente del path fijo del wrapper.
2. `Client disconnect or cancellation triggers kill-and-wait and purges temp directory` durante preflight/post-verificación `ffprobe`.
3. `Vocal separation subprocess timeout at 600s terminates process and frees GPU mutex` para la fase post-verificación `ffprobe`.

## Hallazgos críticos y bloqueadores

### C1 — El preflight PolarFormer puede validar archivos que el wrapper real no consume

`src/infra/vocal/process_vocal_separator_config.rs:32-42` permite que `POLARFORMER_CHECKPOINT` y `POLARFORMER_CONFIG` apunten a cualquier path. `validate_polarformer_artifacts` en `:82-96` valida esos paths sólo con `is_file()`. Sin embargo, el wrapper real fija `models_dir="$HOME/Proyectos/separador-vocal/modelos"` en `/run/current-system/sw/bin/karaoke-separator:19-22` y pasa exclusivamente esos archivos internos a PolarFormer en `:99`; no consume los overrides de la aplicación.

Por tanto, dos archivos válidos en paths alternativos hacen que el preflight pase y se adquiera la GPU aunque falten los pesos que realmente abrirá el wrapper. Además, `is_file()` no verifica explícitamente legibilidad mediante apertura. En el host actual los dos artefactos internos existen y son legibles, pero el contrato de configuración sigue admitiendo el falso positivo. Esto incumple la garantía de rechazo antes del mutex para toda configuración soportada.

### C2 — Strict TDD no cubre cancelación de `ffprobe`

La tabla `Strict TDD Cycle Evidence` existe y las pruebas nuevas cubren cancelación durante separator, conversión FLAC y encode MP3. No existe una prueba de cancelación/timeout durante `run_ffprobe_preflight` ni durante `run_ffprobe_post_verify`: la búsqueda sólo encuentra los tests de FLAC y MP3 en `tests/process_vocal_separator_tests.rs:483` y `:583`.

`PreflightChildGuard` fue añadido en `src/infra/vocal/process_vocal_separator.rs:39-105`, pero su `Drop` inicia kill y delega `wait()` a una tarea sin que un test demuestre terminación/reaping. El post-verify usa el guard común, pero tampoco se cancela en esa fase en pruebas. La evidencia REM sobreafirma C1/C6; bajo strict TDD, esta ausencia es CRITICAL.

## Remediación C1–C6 anterior

| Bloqueador anterior | Estado | Evidencia |
|---|---|---|
| C1 auxiliares fuera de guard | PARCIAL | FLAC, encode y post-verify usan `VocalProcessGuard`; queda evidencia de cancelación `ffprobe` incompleta. |
| C2 PolarFormer | PARCIAL | Preflight local existe, pero puede divergir de los paths fijos del wrapper y no abre los archivos para probar lectura. |
| C3 template/argv | PASS | `VocalCommandBuilder` emite exactamente tres argumentos; wrapper confirma FLAC y `NAME_(STEM)_MODEL`. |
| C4 timeout >600 | PASS | Config rechaza 0 y 601; `main` y `AgentChat` aplican defensa `.min(600)`. |
| C5 label exacto | PASS | Estado y test exigen exactamente `separando voces…`. |
| C6 evidencia strict-TDD | PARCIAL | Nuevos tests son conductuales, pero falta cancelación en las dos fases `ffprobe`. |

## Tareas

- Implementación: **16/16 completadas**.
- Marcadores de implementación sin marcar `- [ ]`: **ninguno**.
- No existe bloqueo por checkboxes.

## Estado estructurado y actionContext

- Change activo inequívoco: `shirley-show-vocal-sep`.
- Estado nativo autoritativo: `ready` para verify; store `openspec`.
- `actionContext.mode`: `repo-local`.
- Workspace y allowed root: `/home/loonbac/Proyectos/shirley-show`.
- Ownership probado: HEAD `652adec25bad82d2c03c22dbf9f5115192db2aa1`; todos los archivos auditados están dentro del workspace permitido.
- Árbol limpio antes de generar este reporte.
- Servidor apagado; no se ejecutó E2E vivo, el test GPU real permaneció `ignored` y no se tocó `.run/`.
- Attempt authority observada: token `sha256:a074cd13ed0c47b12d3a58dc4ca0646d88950a55f8f85480e021f6d1eef8002`, work-unit `verify-vocal-sep`, request `vv-acquire-002`, máximo 500. **NO settle ejecutado.**

## Validación ejecutada

| Comando exacto | Exit | Hash SHA-256 del output | Resultado |
|---|---:|---|---|
| `cargo test --test process_vocal_separator_tests && cargo test --test agent_chat_tests && cargo test --test agent_chat_api_tests && node --test tests/agent-chat-state.test.mjs tests/agent-chat-integration.test.mjs` | 0 | `c311b27156ca4e56ecf1c6f34612a3288a8357df2dcd17918334cacc1e1f28ae` | 13 + 19 + 21 Rust tests y 2 suites Node pasan. |
| `cargo test && node --test tests/*.mjs` | 0 | `d0ccd0a4e97c545fa4db523a514565281d74e0d85375ade0759e5302ab477c47` | 237 Rust pasan, 1 GPU E2E ignored; 5 suites Node pasan. |
| `cargo build` | 0 | `21aacdad5c16e73e83abf0d62570ab602d770a8dafb621029e2013d3e0711792` | Build dev correcto. |
| `cargo fmt -- --check` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | Formato limpio; output vacío. |

El `evidence_revision` es el SHA-256 de un manifiesto que incluye HEAD/tree, hashes de specs/design/tasks/apply-progress/config, wrapper local y resultados exactos de las validaciones anteriores.

## Strict TDD Compliance

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | PASS | Existe `Strict TDD Cycle Evidence` con fases S1–S4 y REM. |
| Test files exist | PASS | Todos los archivos reportados existen. |
| RED cross-reference | PASS | Los targets y fallos previos están descritos y corresponden a los módulos implementados. |
| GREEN current | PASS | Pruebas enfocadas y suite completa pasan ahora. |
| Triangulation adequate | FAIL | No hay cancelación/timeout durante preflight ni post-verify `ffprobe`. |
| Safety net | PASS | Suite Rust/Node completa y build ejecutados. |

**TDD Compliance:** 5/6 checks completos; no admisible en strict TDD mientras C2 siga abierto.

### Test Layer Distribution

Conteo de declaraciones Rust y suites Node relacionadas con el change:

| Capa | Tests | Archivos |
|---|---:|---:|
| Unit | 30 | 3 |
| Integration/herméticos | 84 | 6 |
| E2E manual ignored | 1 | 1 |
| **Total** | **115** | **10** |

### Changed File Coverage

Coverage omitido: no hay herramienta de coverage configurada y el threshold es 0.

### Assertion Quality

No se encontraron tautologías, ghost loops, assertions sólo de tipo, smoke-only ni assertions CSS de implementación en los tests relacionados. Los checks de arrays vacíos tienen assertions complementarias de conducta/no ejecución. La deficiencia es de escenario ausente para `ffprobe`, no de calidad de las assertions existentes.

**Assertion quality:** 0 CRITICAL, 0 WARNING.

### Quality Metrics

- **Formatter:** PASS — `cargo fmt -- --check`.
- **Type/build Rust:** PASS — `cargo build`.
- **Node/JS:** PASS — suite completa Node.
- **Coverage:** no disponible.

## Review Workload / PR Boundary

La estrategia temática S1→S4 se conserva y no se detectó código ajeno al cambio. Sin embargo, el presupuesto no se respetó:

- Los cuatro commits originales cambian aproximadamente 1023, 1744, 1256 y 1940 líneas respectivamente, todos por encima del presupuesto canónico de 400.
- La remediación `652adec` cambia **1850 líneas** (1493 inserciones, 357 eliminaciones) en 31 archivos, por encima del máximo 500 registrado para la work unit.
- No existe `size:exception` explícito.

**WARNING:** hay incumplimiento de workload/review budget, aunque el alcance permanece temáticamente dentro de C1–C6 y no se añade como blocker técnico adicional.

## Coherencia de diseño y contrato real

- PASS: separación dominio/puerto/caso de uso/registry/adapter y estado frontend.
- PASS: no hay shell eval en producción; `tokio::process::Command` usa argv estructurado.
- PASS: contrato observado del wrapper: `karaoke-separator <modelo> <cancion> [salida]`, FLAC y `NAME_(STEM)_MODEL`.
- WARNING: specs/proposal/design aún contienen referencias históricas a `NAME_MODEL_STEM`; deben reconciliarse con el contrato autorizado antes del sync canónico.
- WARNING: `apply-progress.md` contiene una línea residual que vuelve a describir el label antiguo, aunque código, tests y el resto del mismo artefacto usan el valor correcto.

## Bloqueadores exactos

1. Alinear el preflight PolarFormer con los paths fijos realmente consumidos por el wrapper y verificar legibilidad real, evitando que overrides alternativos produzcan un falso positivo antes de GPU.
2. Añadir evidencia strict-TDD conductual de cancelación/timeout y reaping durante preflight y post-verificación `ffprobe`.

No ejecutar `sdd-sync` ni archive hasta una nueva re-verificación limpia.

## Key Learnings

1. Un preflight sólo es válido si comprueba exactamente los artefactos que consumirá el wrapper real.
2. El ownership genérico de procesos necesita pruebas de cancelación en cada ruta de integración distinta.
3. Un argv contractual debe derivarse del wrapper instalado y no de flags asumidos.
4. Las suites verdes no sustituyen la cobertura strict-TDD de escenarios críticos.
