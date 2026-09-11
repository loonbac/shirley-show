```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:471369614aec742cf7d0ce8c4aaf328588c99b0b47e2e1570c361837ff432999
verdict: fail
blockers: 6
critical_findings: 6
requirements: 6/10
scenarios: 49/55
test_command: cargo test && node --test tests/*.mjs
test_exit_code: 0
test_output_hash: sha256:6b92320b81d1ea410ef1f16ed0ff2ba543c3d6b56410ff7115864ea6c977d4d3
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:5e1910eb91fa715b99dec03e979080ae11bd9416a94213aaaa971f48278af446
```

# Verify Report: `shirley-show-vocal-sep`

## Veredicto

**FAIL — no está listo para sync ni archive.** La suite y el build están verdes, las 16 tareas están marcadas, y se confirmó la mayor parte del contrato. Sin embargo, quedan seis bloqueadores críticos de contrato/TDD en el control de procesos, la invocación instrumental, el template CLI, el límite de 600 segundos y el label exacto del frontend.

## Cobertura de especificación

Se recuperaron directamente los dos deltas del change: **10 requirements y 55 scenarios**.

| Requirement | Escenarios | Estado | Evidencia |
|---|---:|---|---|
| Ephemeral Staged Audio Registry with TTL Lifecycle | 5/5 | PASS | Registry en memoria, TTL 900 s, leases, consumo único y cleanup están implementados y probados. |
| Conversational Vocal Separation Feedback and Client State Continuity | 4/5 | FAIL | Continuidad efímera, limpieza y `aria-live` pasan; el label no coincide con el texto exacto requerido. |
| Agent Chat API Endpoint and Content Negotiation | 4/4 | FAIL requirement | Transporte y parsing pasan, pero la configuración permite ampliar separación por encima de 600 s. |
| Response Contract and Action Reporting | 5/5 | PASS | `actions_taken`, staging y persistencia canónica están cubiertos. |
| Vocal Separation Tool and GPU Execution Guardrails | 10/14 | FAIL | Receta, AVG, mutex, preflight y MP3 pasan; fallan garantías del template, PolarFormer configurado y cancelación de todos los hijos. |
| Fixed Tool Allowlist and Schema Definition | 4/4 | PASS | Ocho herramientas exactas y schema cerrado verificados. |
| Staged Track Creation Tool | 6/6 | PASS | Flujo canónico, retry y single-use cubiertos. |
| Presentation Identifier Integrity and Grounded Context | 5/5 | PASS | Destino confirmado antes de GPU y chaining cubiertos. |
| Bounded Model Loop and Request Deadline | 4/5 | FAIL | Extensión dinámica existe; la cancelación de subprocesses auxiliares no cumple kill-and-wait. |
| Duplicate Tool Call Prevention within Turn | 2/2 | PASS | Deduplicación de mutaciones y separación cubierta. |

**Total:** requirements completos **6/10**; scenarios completos **49/55**.

Escenarios incompletos:

1. `User confirms vocal separation and chat displays specialized typing indicator`.
2. `Valid chorus separation executes 3 models and blends stems via AVG_WAVE into 320k MP3` — la receta funciona, pero no se fuerza el template contractual `NAME_MODEL_STEM`.
3. `Valid instrumental separation with PolarFormer produces 320k MP3` — checkpoint/YAML se validan pero no se pasan al proceso.
4. `Separation timeout at 600 seconds terminates processes, purges partials, and frees GPU mutex`.
5. `Client disconnect or cancellation triggers kill-and-wait and purges temp directory`.
6. `Vocal separation subprocess timeout at 600s terminates process and frees GPU mutex`.

## Hallazgos críticos y bloqueadores

### C1 — Subprocesos auxiliares quedan fuera de `VocalProcessGuard`

`src/infra/vocal/process_vocal_separator.rs:346-357` ejecuta la conversión FLAC→WAV con `conv.status().await` sin adjuntar el hijo al guard, sin `kill_on_drop` y sin budget restante. `src/audio/encode_mp3.rs:32-51` ejecuta el ffmpeg final con `cmd.output().await` bajo el mismo patrón. Los ffprobe de `process_vocal_separator.rs:41-57` y `115-131` también usan futures de proceso que se descartan al timeout sin kill-and-wait explícito.

Esto permite que timeout/desconexión cancele el future y que el guard libere el permiso GPU después de limpiar sólo el hijo actualmente adjunto, mientras ffmpeg/ffprobe puede seguir vivo. Incumple la garantía de matar y esperar **todos** los hijos antes de liberar mutex y purgar temporales.

### C2 — La rama instrumental ignora checkpoint y YAML configurados

`process_vocal_separator.rs:227` valida los artefactos, pero descarta las rutas devueltas. La invocación en `process_vocal_separator.rs:417-420` sólo pasa `polarformer`, source y model_dir. No utiliza `polarformer_checkpoint` ni `polarformer_config`, por lo que no se puede demostrar que se ejecuta exclusivamente el checkpoint `model_bs_polarformer_float16.ckpt` y su YAML asociados.

### C3 — No se fuerza el output template `NAME_MODEL_STEM`

Las invocaciones de `karaoke-separator` pasan únicamente modelo, source y directorio de salida (`process_vocal_separator.rs:294-299` y `417-422`). No existe el builder descrito en design ni un argumento contractual que fuerce `NAME_MODEL_STEM`. Los stubs producen nombres convenientes por sí mismos, por lo que los tests no prueban esta precondición de la resolución determinista de stems.

### C4 — El timeout de separación no está acotado a 600 segundos

`src/config.rs:127-145` acepta cualquier entero positivo para `SHIRLEY_SEPARATION_TIMEOUT_SECS`; `src/main.rs:62-65` lo inyecta sin clamp y `src/use_cases/agent_chat.rs:255-259` lo usa directamente como deadline. Un valor de `601` o superior viola el límite contractual de hasta 600 s.

### C5 — El label especializado no coincide exactamente con spec/design

`frontend/js/core/state/chat-state.js:121` usa `Ressy está separando voces… (tarda unos minutos)`. El design exige que `pendingLabel` sea exactamente `separando voces…`, y los scenarios piden ese texto especializado. El test en `tests/agent-chat-state.test.mjs:503` codifica como esperado el texto divergente en vez del contrato.

### C6 — Evidencia strict-TDD incompleta para las garantías críticas

La tabla `Strict TDD Cycle Evidence` existe, pero sobreafirma cobertura:

- `tests/process_vocal_separator_tests.rs:336-388` cancela sólo mientras el separator está activo; no cancela durante conversión FLAC, encode MP3 ni ffprobe.
- `tests/process_vocal_separator_tests.rs:308-332` sólo verifica que el log empiece por `polarformer`; no exige checkpoint/YAML ni ausencia de otros argumentos/modelos.
- Ningún test exige el template `NAME_MODEL_STEM`.
- `tests/agent_chat_tests.rs:1165-1222` llama “600s” a una ejecución de 200 ms; no distingue 30, 120 y 600 s ni prueba el límite superior.
- `tests/agent-chat-state.test.mjs:503` afirma el label incorrecto.

En strict TDD, esta evidencia incompleta para escenarios críticos es bloqueante.

## Tareas

- Implementación: **16/16 completadas**.
- Marcadores de implementación sin marcar `- [ ]`: **ninguno**.
- No hay bloqueo por checkboxes; los bloqueos son de corrección y evidencia.

## Estado estructurado y actionContext

- Change activo: `shirley-show-vocal-sep`, selección inequívoca.
- Estado nativo: `ready` para verify; backend `openspec` autoritativo.
- `actionContext.mode`: `repo-local`.
- Workspace: `/home/loonbac/Proyectos/shirley-show`.
- Ownership probado: los cuatro commits `10cadee`, `98ad471`, `6270160`, `3cada28` y todos los archivos objetivo están dentro del workspace permitido.
- Árbol limpio antes de persistir el reporte.
- Servidor mantenido apagado; no se lanzó E2E vivo y no se modificó `.run/`.
- Attempt authority: token `sha256:b1845fdeeeb144313dd3a58dc4ca0646d88950a55f8f85480e021f6d1eef8002`, work-unit `verify-vocal-sep`, request `vv-acquire-001`, máximo 500. **NO settle ejecutado.**

## Validación ejecutada

| Comando | Exit | Hash de output | Resultado |
|---|---:|---|---|
| `cargo test && node --test tests/*.mjs` | 0 | `sha256:6b92320b81d1ea410ef1f16ed0ff2ba543c3d6b56410ff7115864ea6c977d4d3` | 234 Rust passed, 1 ignored; 5 Node file suites passed. |
| `cargo build` | 0 | `sha256:5e1910eb91fa715b99dec03e979080ae11bd9416a94213aaaa971f48278af446` | Build dev correcto. |
| `cargo fmt -- --check` | 1 | `sha256:e6bc2b7dfc9ed6f0dc721ffb264a506e45b0dbb2faf6ab94e16c77a6e1b895f9` | Informativo: 1323 líneas de diff de formato. |

El E2E GPU real **no se repitió**, según la instrucción. Se auditó `tests/e2e_real_gpu_test.rs` y la evidencia S4 registrada: ejecución manual de chorus en RTX 3060, 34.13 s, codec MP3 y bitrate aproximado 320 kbps, con cleanup final. Esa evidencia valida la ruta feliz real de chorus, pero no cubre PolarFormer, cancelación durante ffmpeg ni el template explícito.

## Strict TDD Compliance

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | PASS | Existe `Strict TDD Cycle Evidence` con RED/GREEN/TRIANGULATE/REFACTOR para S1–S4. |
| Test files exist | PASS | Los 10 archivos de test creados/modificados existen. |
| GREEN current | PASS | Suite completa actual en verde; el E2E GPU permanece `ignored` y se auditó por evidencia. |
| Triangulation adequate | FAIL | No cubre cancelación de ffmpeg/ffprobe, args checkpoint/YAML, template ni frontera real 600 s. |
| Safety net | PASS | `cargo test` y Node completos ejecutados. |
| Evidence accuracy | FAIL | S2/S4 declaran garantías que sus assertions no prueban. |

**TDD Compliance:** 4/6 checks completos; no admisible en strict TDD hasta corregir C6.

### Test Layer Distribution

Conteo de tests relacionados según declaraciones Rust y unidades Node del runner:

| Capa | Tests | Archivos |
|---|---:|---:|
| Unit | 30 | 3 |
| Integration/herméticos | 80 | 6 |
| E2E manual ignored | 1 | 1 |
| **Total** | **111** | **10** |

### Assertion Quality

No se encontraron tautologías, ghost loops ni smoke-only tests en las adiciones del change. Las assertions son conductuales en general, pero cuatro áreas tienen assertions insuficientes: checkpoint/YAML instrumental, template CLI, cancelación durante subprocesses auxiliares y frontera 600 s. El test de frontend además fija el valor contrario al contrato.

**Assertion quality:** 1 CRITICAL de alineación con spec y 4 WARNING de cobertura insuficiente, consolidados en C6.

### Changed File Coverage

Coverage omitido: no hay herramienta de coverage configurada y el threshold es 0.

### Quality Metrics

- **Type/build Rust:** PASS mediante `cargo build`.
- **Node/JS:** PASS mediante `node --test tests/*.mjs`.
- **Formatter:** WARNING; `cargo fmt -- --check` devuelve 1.

## Review Workload / PR Boundary

La estrategia de cuatro slices respeta las fronteras temáticas S1→S4 y no se observó trabajo ajeno a las tareas. Sin embargo, cada commit excede el presupuesto canónico de 400 líneas:

| Slice/commit | Líneas cambiadas | Producción | Tests/docs |
|---|---:|---:|---:|
| S1 `10cadee` | 1023 | 327 | 696 |
| S2 `98ad471` | 1744 | 1014 | 730 |
| S3 `6270160` | 1256 | 451 | 805 |
| S4 `3cada28` | 1940 | 702 | 1238 |

**WARNING:** el chain temático fue aplicado, pero el forecast `~930` total quedó muy por debajo de las 5576 líneas reales y no hay `size:exception` registrado. Esto no añade un séptimo blocker técnico, pero debe reconciliarse antes de afirmar cumplimiento del review budget.

## AGENTS.md, secretos y coherencia de diseño

- **PASS parcial:** dominio, puerto, caso de uso, registry, guard y micro-utilidad frontend están separados y las dependencias principales son unidireccionales.
- **WARNING:** `src/infra/vocal/process_vocal_separator.rs` concentra 536 líneas y múltiples responsabilidades (ffprobe, recipes, conversión, encode y post-validación), contrario al estándar de micro-modularidad extrema; conviene atomizar después de resolver los bloqueadores.
- **PASS secretos:** `.env` no está tracked; el scan de archivos tracked no encontró asignaciones de credenciales reales ni shell eval en producción. No se leyó ni imprimió contenido sensible.
- Las specs canónicas siguen en baseline post-merge de `shirley-show-yt-download` (7 tools/120 s), lo esperado antes de `sdd-sync`; no deben sincronizarse mientras verify siga en FAIL.

## Próximo paso

Corregir los seis bloqueadores mediante una nueva work unit con strict TDD; luego repetir verify completo. No ejecutar `sdd-sync` ni archive todavía.

## Key Learnings

1. Los subprocesses auxiliares también necesitan ownership de kill-and-wait bajo cancelación.
2. Validar artefactos PolarFormer no sustituye pasarlos explícitamente al proceso configurado.
3. Los tests de deadline deben distinguir realmente presupuestos de treinta, ciento veinte y seiscientos segundos.
4. Un E2E GPU exitoso no demuestra por sí solo todas las garantías de cleanup.
5. Los labels contractuales exactos deben probarse contra la especificación, no contra la implementación.
