```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:26fde9e82a04cc791d41e5bfcb4f84483d9bd948acedc65265d4db59b70461b1
verdict: fail
blockers: 5
critical_findings: 5
requirements: 6/10
scenarios: 51/52
test_command: cargo test
test_exit_code: 0
test_output_hash: sha256:d65fc5491d190bf21159c63625cceeea0819b6a6e6f3b33b3fdd6c08aee8be0e
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:55bbad32602b90409862370b8dc1f404d17bcc89b66ae3d7f18a84ec23a85e2d
```

# Verify Report: `shirley-show-agent-parity`

## Veredicto

**FAIL — no está listo para sync ni archive.** La compilación, el formato y todas las pruebas ejecutadas pasan, pero la revisión contra las 10 requirements y 52 scenarios canónicos encontró cinco incumplimientos CRITICAL.

## Cobertura de especificación

| Área | Resultado | Evidencia |
|---|---|---|
| Catálogo exacto | PASS | `src/agent/tool_schemas.rs` contiene exactamente 32 `ToolDefinition`; las dos suites de contrato comprueban el conteo y nombres. |
| Schemas cerrados | PASS parcial | Los 32 schemas de nivel superior usan `additionalProperties: false`; también lo usan los objetos anidados de tipos de entrada. Los structs de argumentos usan `#[serde(deny_unknown_fields)]`. Persisten fallos de validación semántica descritos abajo. |
| Tools seguras | FAIL | El dispatch y los casos felices funcionan, pero no se registran receipts `ToolAction(status="error")` tras errores de dominio; `editar_elemento` rechaza una combinación permitida por la spec; fecha/hora y fecha no se validan estrictamente antes del use case. |
| Barrera destructiva | FAIL | Registry server-side, `take` atómico, TTL, aislamiento, replay, cancelación y cascadas están implementados y probados; el clasificador acepta frases no autorizadas por el contrato exacto. |
| Afiche por magic bytes | PASS | JPEG, PNG y WebP se clasifican por firma; spoofing y archivos mayores a 10 MiB se rechazan; `subir_afiche` consume el handle una vez y usa `ReplaceEventPoster`. |
| Prompt compacto | NO PROBADO | El test estima 2.328 tokens para 7.448 caracteres, pero no usa el tokenizer publicado del modelo Groq. El escenario canónico de tokenización queda sin evidencia admisible. |
| Exclusiones locales | PASS | No hay tools de play/pause, Stage Mode o cámara; el prompt las remite a controles locales. |
| Dedupe | PASS | Firmas/call IDs evitan mutaciones duplicadas dentro del turno. |

**Requirements completas:** 6/10.  
**Scenarios comprobados:** 51/52; queda sin prueba exacta el escenario de tokenización Groq `< 2500`.

## Hallazgos CRITICAL / blockers

1. **La confirmación destructiva no exige las frases exactas canónicas.** `src/agent/destructive_confirmation.rs:188-196` acepta `si` sin tilde, `eliminalo`/`finalizalo` sin tildes y puntuación exterior adicional. Las specs solo autorizan `sí, elimínalo` / `sí elimínalo` y `sí, finalízalo` / `sí finalízalo`. Esto amplía una frontera de seguridad server-side.
2. **Las ocho tools seguras no registran `ToolAction` de error cuando el use case falla.** Las ramas `Err(e) => app_error_to_json(&e)` en `src/agent/tool_dispatcher.rs:1757,1827,1904,1965,1999,2058,2121,2184` devuelven el error pero no llaman `record_action`, incumpliendo el receipt obligatorio con `status="error"` y el ID afectado.
3. **`editar_elemento` contradice la spec canónica.** `src/agent/tool_dispatcher.rs:1841-1847` rechaza que `texto` y `titulo` estén presentes simultáneamente, mientras la requirement exige “al menos uno”; `tests/agent_tools_tests.rs` codifica la restricción más estrecha del diseño en lugar del contrato canónico.
4. **La validación de fechas no es estricta en la frontera del dispatcher.** `editar_evento` solo rechaza `fecha_hora` vacía y `normalize_event_fecha_hora` deja pasar texto inválido al use case; `editar_gasto` solo comprueba que `fecha` no esté vacía. La spec exige los formatos `YYYY-MM-DDTHH:MM` y `YYYY-MM-DD` antes de invocar casos de uso.
5. **La evidencia Strict TDD está incompleta y sobredeclara tokenización.** La tabla existe, pero no mapea las 31 tareas ni incluye Safety Net; los slices combinan tests e implementación en un único commit, por lo que RED no puede corroborarse desde el historial. Además, el test llamado “token budget” usa una heurística de caracteres/palabras, no el tokenizer del modelo Groq que `apply-progress.md` afirma haber verificado.

## Estado de tareas

- Implementación: **31/31** marcada completa.
- Marcadores de implementación sin completar: **ninguno**; no existen líneas que coincidan con `^\s*- \[ \]` en `tasks.md`.
- La completitud de checkboxes no elimina los blockers de conformidad anteriores.

## Structured status y actionContext

- Status nativo: `ready`, autoritativo, artifact store `openspec`.
- Modo: `repo-local`.
- Workspace/allowed root: `/home/loonbac/Proyectos/shirley-show`.
- Ownership de implementación y tests: probado dentro del workspace permitido.
- El servidor en `:3100`, `.run/` y `production-data/` no se iniciaron, leyeron ni modificaron. `production-data/` ya estaba sin tracking y permaneció así.

## Comandos de validación

| Comando exacto | Exit | Hash SHA-256 de output | Resultado |
|---|---:|---|---|
| `cargo test --test agent_tools_tests --test ticketing_ressy_tests --test agent_chat_tests --test agent_chat_api_tests --test agent_chat_ui_tests` | 0 | `98d6e4ca96f46ae98ae0d5f22f2c90fe6a93a363637e3f941ffbd5dfc200e123` | Suites dirigidas GREEN. |
| `node tests/agent-chat-state.test.mjs && node tests/agent-chat-ui-components.test.mjs && node tests/agent-chat-integration.test.mjs` | 0 | `e5aa8e7177a9ff672a506de5a97ba67f40d07dcc6d1c41c9c53b5173b5e7cbd6` | 22 + 14 + 14 checks GREEN. |
| `cargo test` | 0 | `d65fc5491d190bf21159c63625cceeea0819b6a6e6f3b33b3fdd6c08aee8be0e` | Suite integral GREEN; un test GPU real permanece ignored por diseño. |
| `cargo build` | 0 | `55bbad32602b90409862370b8dc1f404d17bcc89b66ae3d7f18a84ec23a85e2d` | Build dev exitoso. |
| `cargo fmt --check` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | Sin diferencias. |
| `cargo test --test agent_tools_tests test_system_prompt_token_budget_strictly_below_2500_tokens_with_32_tools -- --nocapture` | 0 | `62da6f7edac1d53f2406f3b8b03a8cb64c190bec4561e0774f86bbfe4efcabb8` | Heurística: 7.448 chars, 1.056 words, 2.328 tokens estimados; no es tokenización Groq. |

## Strict TDD Compliance

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | PASS | Existe `## TDD Cycle Evidence`. |
| Test files existen | PASS | Los 7 archivos reportados/modificados existen. |
| RED verificable | FAIL | No hay commits RED separados ni evidencia ejecutable preservada; tests e implementación llegaron juntos por slice. |
| GREEN actual | PASS | 135 tests/checks relacionados pasan en los 7 archivos modificados. |
| Triangulación | WARNING | Amplia para flujos principales; faltan variantes negativas para frase exacta y formatos de fecha. |
| Safety Net | FAIL | La tabla no contiene evidencia Safety Net por tarea/archivo modificado. |

**TDD Compliance:** incompleto; es blocker bajo strict TDD.

## Test Layer Distribution

| Layer | Tests/checks | Files | Herramienta |
|---|---:|---:|---|
| Unit | 22 | 1 | Node `assert` sobre estado/API cliente |
| Integration | 113 | 6 | Cargo integration tests + harness JS de composición |
| E2E navegador/red real | 0 | 0 | No disponible/ejecutado |
| **Total relacionado** | **135** | **7** | |

## Assertion Quality

- **0 CRITICAL** por tautologías, ghost loops o assertions sin producción.
- **1 WARNING:** `tests/agent_chat_api_tests.rs:1584-1622` describe cleanup de imagen no consumida, pero solo afirma HTTP 200; no comprueba un contador de archivos/handles antes y después. La semántica `Vec<u8>` request-scoped es coherente por inspección, pero la assertion no prueba el cleanup anunciado.
- Coverage de archivos cambiados: omitida; no hay herramienta de coverage configurada.

## Review Workload / límite de entrega

- Se respetaron conceptualmente los tres slices asignados: safe tools, destructivas y afiche.
- No se registró `size:exception`.
- El forecast recomendó PRs encadenados y dejó `Chain strategy: pending`; `apply-progress.md` afirma “stacked-to-main 3 chained PRs”, mientras el historial muestra tres commits directamente en `main`.
- Los commits excedieron ampliamente el presupuesto de 400 líneas: S1 `+3575/-118`, S2 `+2521/-175`, S3 `+1281/-149` (incluyen artefactos y tests). **WARNING:** la separación temática ayuda, pero el límite de revisión y la estrategia de cadena no quedaron satisfechos ni reconciliados explícitamente.

## Riesgos

- La ampliación de frases aceptadas reduce la fuerza de la barrera destructiva aunque replay, TTL y concurrencia sean correctos.
- Los errores de tools seguras no producen receipts reactivos/auditables conforme al contrato.
- El prompt estimado queda cerca del límite (172 tokens de margen estimado) sin evidencia del tokenizer real.
- Un suite GREEN no cubre las contradicciones entre tests, diseño y spec canónica.
