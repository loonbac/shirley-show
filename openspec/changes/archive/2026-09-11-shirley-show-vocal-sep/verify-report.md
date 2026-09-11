```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:590d0e51a50d6483f1dc0fe7520ecf2dc28b9650775173f62900e36fb716dda4
verdict: pass
blockers: 0
critical_findings: 0
requirements: 10/10
scenarios: 55/55
test_command: cargo test && node --test tests/*.mjs
test_exit_code: 0
test_output_hash: sha256:d9a7778f0b7e75accf39de1fc2df9ecdb750d9c42857c4b3cb9d0097613e12ca
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:a5f4c585ee974ca44916ac30a98bbc189e067a7e0a6bc6d2e8d6bc525be724af
```

# Re-verificación FINAL: `shirley-show-vocal-sep`

## Veredicto

**PASS — los dos bloqueadores críticos anteriores están remediados.** La implementación satisface los 10 requirements y 55 scenarios recuperados de los dos deltas. No quedan tareas de implementación sin marcar. Las pruebas enfocadas, la suite completa, el build y el formatter pasan.

No se ejecutó E2E GPU vivo: `tests/e2e_real_gpu_test.rs` permaneció `ignored`. El reporte contiene advertencias no bloqueantes sobre el entorno observado, portabilidad de una prueba, workload y documentación histórica.

## Re-verificación de los dos ex-bloqueadores

### C1 anterior — Preflight PolarFormer alineado con el wrapper real: REMEDIADO

- `/run/current-system/sw/bin/karaoke-separator` tiene SHA-256 `28814361ccd62a6cc46cc33689ca539ad1cbcacd07d8b8cbdf8ce7807aba0a3a`.
- El wrapper observado fija `$HOME/Proyectos/separador-vocal/modelos`, usa `model_bs_polarformer_float16.ckpt` y `model_bs_polarformer_float16.yaml`, y no consume overrides `POLARFORMER_*`.
- `ProcessVocalSeparatorConfig::contractual_polarformer_paths` deriva exactamente esos paths.
- `validate_polarformer_artifacts_at` abre ambos artefactos mediante `File::open` y confirma que sean archivos regulares antes de adquirir el semáforo GPU.
- Los artefactos del host se observaron como archivos regulares y legibles: checkpoint de 102410301 bytes y YAML de 3599 bytes.
- Las pruebas cubren paths contractuales, archivo ausente, YAML ausente, directorio en lugar de archivo y overrides ignorados.

### C2 anterior — Evidencia de cancelación/timeout de ffprobe: REMEDIADO

`tests/process_vocal_separator_tests.rs` incluye y ejecuta pruebas conductuales para las cuatro rutas requeridas:

1. `test_cancellation_during_ffprobe_preflight_kills_and_reaps_child`.
2. `test_timeout_during_ffprobe_preflight_kills_and_reaps_child`.
3. `test_cancellation_during_ffprobe_post_verify_kills_child_purges_workdir_and_releases_permit`.
4. `test_timeout_during_ffprobe_post_verify_kills_child_purges_workdir_and_releases_permit`.

Las pruebas de cancelación demuestran inicio del proceso, presencia previa en `/proc/{pid}`, desaparición posterior, ausencia de output parcial y semántica correcta del permiso GPU. Las pruebas de timeout ejercitan las mismas rutas de guard/reaping y confirman limpieza y restauración del semáforo. La suite enfocada pasa 18/18.

## Cobertura de especificación

| Requirement | Escenarios | Estado | Evidencia principal |
|---|---:|---|---|
| Ephemeral Staged Audio Registry with TTL Lifecycle | 5/5 | PASS | TTL 900 s, cleanup, tombstones, retry y consumo único. |
| Conversational Vocal Separation Feedback and Client State Continuity | 5/5 | PASS | Label exacto, estado efímero, cancelación y restauración. |
| Agent Chat API Endpoint and Content Negotiation | 4/4 | PASS | JSON/multipart, staged handle y deadline especializado. |
| Response Contract and Action Reporting | 5/5 | PASS | Grounding por etapas, actions y staging de respuesta. |
| Vocal Separation Tool and GPU Execution Guardrails | 14/14 | PASS | Preflights, mutex, pipelines, stems, MP3, timeout y cleanup. |
| Fixed Tool Allowlist and Schema Definition | 4/4 | PASS | Allowlist exacta de ocho tools y validación cerrada. |
| Staged Track Creation Tool | 6/6 | PASS | Alta canónica, retry, expiración y single-use. |
| Presentation Identifier Integrity and Grounded Context | 5/5 | PASS | Destino confirmado, IDs autorizados y chaining. |
| Bounded Model Loop and Request Deadline | 5/5 | PASS | Cinco iteraciones y budgets 30/120/600 s. |
| Duplicate Tool Call Prevention within Turn | 2/2 | PASS | Deduplicación evita una segunda separación. |

**Cobertura total:** 10/10 requirements y 55/55 scenarios.

## Tareas

- Implementación: **16/16 completadas**.
- Marcadores de implementación sin marcar que coincidan con `^\s*- \[ \]`: **ninguno**.
- Estado de archive por completitud de tareas: **sin bloqueo**.

## Estado estructurado y actionContext

- Change activo inequívoco: `shirley-show-vocal-sep`.
- Estado nativo autoritativo: `ready` para verify; artifact store `openspec`.
- `actionContext.mode`: `repo-local`.
- Workspace y allowed edit root: `/home/loonbac/Proyectos/shirley-show`.
- Ownership probado: HEAD `0ba901a85f443be0fbc804d50eaefaa3ab8c714b`, árbol `a7283622d25bb6b83cec3e6ecea4fedc147256d3`; todos los archivos auditados están dentro del workspace autorizado.
- Árbol limpio antes de persistir este reporte.
- Attempt authority observada: token `sha256:d144d186f43396ea8e619830e2b7ed6ec0dadd630e1eb5215f077e1e1510a352`, work-unit `verify-vocal-sep`, request `vv-acquire-003`, máximo 500. **NO settle ejecutado.**

## Validación ejecutada

| Comando exacto | Exit | Hash SHA-256 del output | Resultado |
|---|---:|---|---|
| `cargo test --test process_vocal_separator_tests` | 0 | `9cad71ae803623f2c60897e350435f9051b8c1c8ac5551b9140800b28de96a2b` | 18/18 pruebas enfocadas pasan. |
| `cargo test && node --test tests/*.mjs` | 0 | `d9a7778f0b7e75accf39de1fc2df9ecdb750d9c42857c4b3cb9d0097613e12ca` | 243 pruebas Rust pasan, 1 E2E GPU queda ignored; cinco entrypoints Node pasan. |
| `cargo build` | 0 | `a5f4c585ee974ca44916ac30a98bbc189e067a7e0a6bc6d2e8d6bc525be724af` | Build dev correcto. |
| `cargo fmt -- --check` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | Formato limpio; output vacío. |

El `evidence_revision` es el SHA-256 de un manifiesto que fija HEAD/tree, conteos, hashes de specs/design/tasks/apply-progress/config, wrapper contractual, artefactos PolarFormer y outputs exactos de las validaciones.

## Strict TDD Compliance

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | PASS | `apply-progress.md` contiene `Strict TDD Cycle Evidence` con S1–S4, REM y REM2. |
| All tasks have tests | PASS | 16/16 tareas enlazan pruebas RED/GREEN/TRIANGULATE/REFACTOR. |
| RED cross-reference | PASS | Los archivos y targets reportados existen; REM2 registra el fallo previo por función ausente. |
| GREEN current | PASS | Suite enfocada y suite completa permanecen verdes. |
| Triangulation adequate | PASS | PolarFormer y ffprobe tienen variantes de éxito, ausencia, cancelación y timeout. |
| Safety net | PASS | Suite Rust/Node completa, build y fmt ejecutados. |

**TDD Compliance:** 6/6 checks completos.

### Test Layer Distribution

| Capa | Tests/checks | Archivos |
|---|---:|---:|
| Unit | 80 | 5 |
| Integración/herméticos | 75 | 5 |
| E2E manual ignored | 1 | 1 |
| **Total relacionado** | **156** | **11** |

### Changed File Coverage

Coverage omitido: no hay herramienta de coverage configurada y el threshold es 0.

### Assertion Quality

No se encontraron tautologías, ghost loops, assertions exclusivamente de tipo, smoke-only ni assertions CSS de implementación en las pruebas relacionadas. Los checks `typeof` de Node se acompañan de comportamiento/valor en sus respectivos casos. Las pruebas de ffprobe llaman producción y verifican proceso, cleanup y semáforo.

**Assertion quality:** 0 CRITICAL, 0 WARNING.

### Quality Metrics

- **Formatter:** PASS — `cargo fmt -- --check`.
- **Type/build Rust:** PASS — `cargo build`.
- **Node/JS:** PASS — `node --test tests/*.mjs` dentro de la suite completa.
- **Coverage:** no disponible.

## Review Workload / PR Boundary

- El alcance implementado coincide con las slices S1→S4 y las remediaciones; no se detectó scope creep temático fuera del cambio.
- La estrategia de commits conserva límites temáticos, pero no el presupuesto de líneas: los commits S1–S4 y las dos remediaciones superan 400 líneas de churn.
- El commit final `0ba901a` cambia 577 inserciones y 208 eliminaciones, **785 líneas de churn**, por encima del máximo 500 indicado para el attempt.
- No se encontró `size:exception` explícito.

**WARNING:** el límite de review workload no fue respetado, aunque no queda blocker funcional y la frontera temática está preservada.

## Coherencia de diseño y contrato operativo

- PASS: dominio, puerto, caso de uso, adapter, registry y frontend permanecen desacoplados por responsabilidad.
- PASS: el wrapper recibe argv estructurado `[modelo, input, output_dir]`; producción no usa shell eval.
- PASS: el contrato operativo observado usa FLAC y `NAME_(STEM)_MODEL`; los resolvers y pruebas coinciden.
- PASS: el preflight PolarFormer verifica exactamente los artefactos consumidos por el wrapper antes del GPU.
- PASS: cancelación y timeout cubren separator, conversión FLAC, encode MP3 y ambas fases ffprobe.
- WARNING: specs/proposal/design aún contienen referencias históricas a `NAME_MODEL_STEM` y el design describe overrides `POLARFORMER_*`; deben reconciliarse durante sync con el contrato operativo autorizado.
- WARNING: dos pruebas PolarFormer no ignoradas dependen de artefactos físicos del host; pasan aquí, pero reducen la portabilidad respecto de la estrategia original de CI completamente hermética.

## Observaciones del entorno

- No se lanzó ningún E2E vivo ni se ejecutó el test GPU ignored.
- No se escribió ni manipuló `.run/` durante esta verificación.
- Contra la precondición indicada en la solicitud, se observó un proceso vivo `target/debug/shirley-show` con PID 568498 y cwd del repositorio, y ya existía `.run/` con datos. Esto no afectó las suites herméticas, pero debe revisarse operativamente antes de afirmar que el servidor está apagado.

## Bloqueadores exactos

**Ninguno.** El cambio queda técnicamente apto para `sdd-sync`; archive sólo después de completar el sync y conservar cero tareas sin marcar.

## Key Learnings

1. El preflight debe abrir exactamente los artefactos consumidos por el wrapper real.
2. Las rutas de cancelación necesitan pruebas conductuales para cada fase de proceso externo.
3. Una suite local verde puede conservar riesgos de portabilidad por dependencias físicas del host.
4. Los límites de revisión deben medirse con churn real antes de cerrar cada slice.
