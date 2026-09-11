```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:31d63979ab718df7ca0939cfbfb32cebead83232d4fb44d26bd329de829687fc
verdict: fail
blockers: 5
critical_findings: 5
requirements: 7/10
scenarios: 49/51
test_command: cargo test
test_exit_code: 0
test_output_hash: sha256:46946cf5e33257ebe955273c9de8f71d5323b453734bafa83eeb0c0addfb8f72
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:59639588a6f1826f129cbcd121edc07279dab94786f5fa5bde6b3439b2e845cc
```

# Verify report: `shirley-show-yt-download`

## Veredicto

**FAIL — no está listo para archivar.** La suite completa y el build pasan, pero hay cinco hallazgos críticos: dos incumplimientos reproducibles del estado/typing del frontend, un riesgo de bloqueo del subproceso, evidencia TDD incompleta y límites de revisión incumplidos.

## Cobertura de especificación

Se recuperaron **10 requisitos** y **51 escenarios** de `specs/agent-tools/spec.md` y `specs/agent-chat/spec.md`.

| Requisito | Estado | Evidencia |
|---|---|---|
| YouTube Audio Download Tool and Execution Guardrails | ❌ Incompleto | `YouTubeUrl`, argumentos estructurados, `320K`, límites, timeout y purga tienen pruebas verdes; sin embargo, `process_youtube_downloader.rs` espera al child antes de drenar stdout/stderr, por lo que una salida mayor que el pipe puede bloquear al child y producir un timeout falso. |
| Fixed Tool Allowlist and Schema Definition | ✅ Completo | Las pruebas confirman exactamente siete tools y schemas cerrados. |
| Staged Track Creation Tool | ✅ Completo | Reserva, promoción, compensación, single-use y retry están cubiertos y verdes. |
| Presentation Identifier Integrity and Grounded Context | ✅ Completo | Encadenamiento mismo turno, flujo de dos turnos y ambigüedad están cubiertos. |
| Bounded Model Loop and Request Deadline | ✅ Completo | Default/configuración de 120 s, límite de cinco iteraciones y subtimeout de 90 s están implementados y verdes. |
| Duplicate Tool Call Prevention within Turn | ✅ Completo | La descarga y demás mutaciones duplicadas se rechazan en pruebas. |
| Ephemeral Staged Audio Registry with TTL Lifecycle | ❌ Incompleto | TTL de producción es 900 s y las pruebas de expiración pasan; `get_descriptor`/`get_state` no purgan oportunistamente y pueden observar una entrada expirada antes de la tarea, y una reserva que vence y luego se confirma puede conservar el tombstone indefinidamente. |
| Conversational Download Progress and Client State Continuity | ❌ Incompleto | El probe dirigido reproduce dos fallos: se reenvía un ID que `beginTurn` ya limpió y cualquier request lento cambia al label de descarga aunque no contenga YouTube. El texto implementado también es `Ressy está descargando el audio…`, no el literal especificado `descargando el audio…`. |
| Agent Chat API Endpoint and Content Negotiation | ✅ Completo | JSON/multipart, `staged_audio_id`, 415 y deadline HTTP están cubiertos. |
| Response Contract and Action Reporting | ✅ Completo | `actions_taken`, grounding, `active_presentation_id` y `staged_audio_id` están cubiertos. |

**Escenarios completos: 49/51.** Escenarios no conformes:

1. `User message with YouTube URL switches typing indicator to qualitative download status`: el label no coincide con el literal normativo.
2. `Non-download user message retains default thinking indicator`: después de 3,1 s el controller lo cambia a `Ressy está descargando el audio…`.

## Estado de tareas

- `tasks.md`: **24/24** marcadas `[x]`.
- Marcadores de implementación pendientes `- [ ]`: **ninguno**.
- La completitud de checkboxes no compensa los bloqueos funcionales, TDD y de review workload.

## Hallazgos críticos y bloqueadores exactos

1. **CRITICAL — capability staged obsoleta enviada en comandos no relacionados.** En `frontend/js/controllers/agent-chat-controller.js`, `stagedAudioId` se captura antes de `chatState.beginTurn()`. Aunque `beginTurn()` limpia el estado para un comando no relacionado, la API recibe el valor anterior. El probe observó `staged_audio_id="staged-old-capability"` al enviar `Crea una presentación nueva`.
2. **CRITICAL — typing de descarga falso en requests lentos.** El timer de 3 s de `agent-chat-controller.js` aplica incondicionalmente `Ressy está descargando el audio…`. El probe observó ese label para `Lista mis presentaciones`, incumpliendo el estado genérico obligatorio.
3. **CRITICAL — stdout/stderr no se drenan concurrentemente.** En `src/infra/youtube/process_youtube_downloader.rs`, `c.wait().await` ocurre antes de `read_fut.await`. Si cualquiera de los pipes llena su buffer, el child no puede terminar y el parent espera hasta timeout. Esto contradice el diseño cancel-safe con pipes drenados concurrentemente.
4. **CRITICAL — evidencia TDD estricta incompleta.** `apply-progress.md` sólo contiene la tabla `Evidencia de Ciclo TDD Estricto (Slice 4)` para 6/24 tareas. No hay evidencia de ciclos RED/GREEN de Slices 1–3 ni columna/evidencia de safety net por tarea.
5. **CRITICAL — review budget incumplido sin `size:exception`.** El límite era 400 líneas por unidad. Excluyendo OpenSpec, los commits contienen S1 **1299**, S2 **1250**, S3 **1318** y S4 **955** líneas cambiadas. `tasks.md` dejó `Chain strategy: pending`, `apply-progress.md` declara cuatro PRs stacked-to-main, no existe aceptación `size:exception`, y el historial local no prueba esos límites/PRs.

## Advertencias

1. **WARNING — TTL observable parcialmente fuera de contrato.** Los getters públicos del registry no hacen purge oportunista y el camino reserved-at-expiry → consumed no programa una segunda eliminación del tombstone.
2. **WARNING — formato.** `cargo fmt --all -- --check` falla en archivos Rust de los cuatro slices; es informativo, pero el árbol no está rustfmt-clean.
3. **WARNING — assertion quality.** Se encontraron 17 assertions acopladas a clases/estilos en `tests/agent-chat-ui-components.test.mjs` y una comprobación débil de no persistencia en `tests/agent-chat-state.test.mjs:385` (`typeof globalThis.localStorage`). No se encontraron tautologías ni ghost loops.
4. **WARNING — bitrate físico no revalidado en esta sesión.** Los argumentos exactos contienen `--audio-format mp3 --audio-quality 320K`, pero `.run/data/audio/files` estaba vacío; no había fixture para ejecutar `ffprobe`. No se lanzó E2E vivo.

## Validaciones solicitadas

| Área | Resultado |
|---|---|
| `cargo test` | ✅ Exit 0; 24 binarios/suites Rust, 0 fallos; incluye los tres harnesses Node. |
| `cargo build` | ✅ Exit 0. |
| `cargo fmt --all -- --check` | ⚠️ Exit 1; diferencias de rustfmt. |
| `YouTubeUrl` estricta | ✅ Hosts/esquemas/credenciales/puertos/IDs/metacaracteres y canonicalización cubiertos. |
| Args exactos de `yt-dlp` | ✅ Orden exacto probado, incluidos `--no-config`, `--no-playlist`, MP3, `320K`, `100M`, filtro live/duración y URL canónica. |
| Registry TTL | ⚠️ 900 s y cleanup normal verdes; getters/tombstone expirado incompletos. |
| Encadenamiento | ✅ Mismo turno y segundo turno verdes. |
| `staged_audio_id` | ❌ Transporte normal verde, pero se fuga el ID anterior en un comando no relacionado. |
| Timeout 120 s / subtimeout 90 s | ✅ Configuración y deadline verdes; permanece el riesgo de pipe deadlock descrito. |
| Typing | ❌ Probe dirigido con dos incumplimientos. |
| MP3 320K | ✅ Flag estructurado verificado; ➖ ffprobe físico omitido por ausencia de fixture. |
| Secretos | ✅ No se encontraron firmas de secretos reales en archivos tracked; `GROQ_API_KEY` está vacío en `.env.example`. La key de tests es un fixture explícitamente falso. |

## TDD Compliance

| Check | Result | Details |
|---|---|---|
| TDD Evidence reported | ❌ Parcial | Sólo Slice 4, 6/24 tareas. |
| Test files referenced exist | ✅ | Todos los archivos citados existen. |
| RED artifacts present | ✅ | Los ocho grupos de pruebas RED de `tasks.md` tienen archivos reales. |
| GREEN remains true | ✅ | `cargo test` pasó completamente. |
| Triangulation adequate | ✅ | Casos positivos, negativos, concurrencia, timeout, cleanup y UI están presentes. |
| Safety Net for modified files | ❌ | No está documentado por tarea/slice. |

**TDD Compliance: 4/6 checks satisfechos; el protocolo documental incompleto es bloqueante bajo strict TDD.**

### Test Layer Distribution

| Layer | Tests | Files | Herramientas |
|---|---:|---:|---|
| Unit/component | 43 | 4 | Rust test harness + Node fake DOM |
| Integration | 71 | 8 | Rust/Axum/tempdir/process stubs + Node fake DOM |
| E2E vivo | 0 | 0 | Omitido por servidor apagado y restricción de `.run/` |
| **Total** | **114** | **11 archivos únicos** | |

### Changed File Coverage

Coverage omitido: no hay herramienta de coverage configurada y el threshold es 0.

### Assertion Quality

| Archivo | Líneas | Hallazgo | Severidad |
|---|---|---|---|
| `tests/agent-chat-ui-components.test.mjs` | 47–482 | 17 assertions sobre clases, CSS literal o `style.display`, acopladas a detalles de implementación. | WARNING |
| `tests/agent-chat-state.test.mjs` | 385 | `typeof globalThis.localStorage === 'undefined'` no demuestra por sí sola que producción nunca persista. | WARNING |

**Assertion quality: 0 CRITICAL, 18 WARNING.**

### Quality Metrics

- **Rust type/build check:** ✅ `cargo build` sin errores.
- **Formatter:** ⚠️ `cargo fmt --all -- --check` exit 1.
- **Frontend type checker:** ➖ No disponible para JavaScript vanilla.
- **Coverage:** ➖ No disponible.

## Review workload / frontera de entrega

La estrategia propuesta requería cuatro slices revisables de hasta 400 líneas. El código está separable por commits (`22225a4`, `c3bb057`, `4e31276`, `579ef84`), pero cada unidad supera ampliamente el presupuesto. Además, S4 reside en el commit `579ef84` cuyo título describe sólo un ajuste de nombre, mientras `a69f7fb` contiene únicamente planificación del change `shirley-show-vocal-sep`. No se puede confirmar una cadena real de cuatro PRs desde el historial local.

## Structured status y actionContext

- El status inyectado marcaba selección ambigua entre dos changes; la petición del usuario resolvió explícitamente el change como `shirley-show-yt-download`.
- `actionContext.mode`: `repo-local`.
- Workspace autorizado: `/home/loonbac/Proyectos/shirley-show`.
- La propiedad de todos los archivos verificados está dentro del workspace autorizado.
- El árbol quedó limpio después de las verificaciones.
- No se escribió en `.run/`, no se tocó `Evento 28 de Agosto`, no se lanzó el servidor y no se ejecutó E2E vivo.
- Attempt authority usada únicamente para `verify-yt-download`; **NO settle** ejecutado.

## Comandos y resultados

1. `cargo test` → exit **0**. Output hash: `sha256:46946cf5e33257ebe955273c9de8f71d5323b453734bafa83eeb0c0addfb8f72`.
2. `cargo build` → exit **0**. Output hash: `sha256:59639588a6f1826f129cbcd121edc07279dab94786f5fa5bde6b3439b2e845cc`.
3. `cargo fmt --all -- --check` → exit **1**. Output hash: `sha256:c56cb10a427b7abab8aebee0eba7a888059baba8f5f97120bb20e30c36e980bd`.
4. Probe dirigido exacto → exit **1**, con dos incumplimientos reproducidos. Output hash: `sha256:dd197d22379a82808e60412b9f497d989896666aad046a35b62d341a088f9f18`.

```bash
node --input-type=module <<'EOF'
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { setupDomEnvironment } from './tests/fake-dom.mjs';
setupDomEnvironment();
const { createChatState } = await import(pathToFileURL(resolve('frontend/js/core/state/chat-state.js')).href);
const { createAgentChatController } = await import(pathToFileURL(resolve('frontend/js/controllers/agent-chat-controller.js')).href);
let failures = 0;
{
  const state = createChatState({ stagedAudioId: 'staged-old-capability' });
  let payload;
  globalThis.fetch = async (_url, opts) => {
    payload = JSON.parse(opts.body);
    return { ok: true, status: 200, headers: { get: () => 'application/json' }, json: async () => ({ message: 'ok', actions_taken: [] }) };
  };
  const ctrl = createAgentChatController({ chatState: state });
  await ctrl.sendAndOpen('Crea una presentación nueva');
  console.log(`unrelated payload staged_audio_id=${JSON.stringify(payload.staged_audio_id)}`);
  if (payload.staged_audio_id !== null) failures++;
  ctrl.destroy();
}
{
  const state = createChatState();
  let release;
  globalThis.fetch = () => new Promise((resolveFetch) => {
    release = () => resolveFetch({ ok: true, status: 200, headers: { get: () => 'application/json' }, json: async () => ({ message: 'ok', actions_taken: [] }) });
  });
  const ctrl = createAgentChatController({ chatState: state });
  const pending = ctrl.sendAndOpen('Lista mis presentaciones');
  await new Promise((r) => setTimeout(r, 3100));
  const label = ctrl.panelEl.querySelector('.agent-typing-indicator')?.getAttribute('aria-label');
  console.log(`non-download label after 3.1s=${JSON.stringify(label)}`);
  if (label !== 'Ressy está pensando...') failures++;
  release();
  await pending;
  ctrl.destroy();
}
console.log(`focused_spec_failures=${failures}`);
process.exitCode = failures === 0 ? 0 : 1;
EOF
```

Salida del probe:

```text
unrelated payload staged_audio_id="staged-old-capability"
non-download label after 3.1s="Ressy está descargando el audio…"
focused_spec_failures=2
```

## Próxima acción recomendada

Corregir los cinco bloqueadores en una nueva unidad acotada, añadir pruebas de regresión para el orden `beginTurn`/payload, el timer no-YouTube y presión de stdout/stderr, reconciliar la evidencia TDD y volver a ejecutar verify. No archivar este change todavía.

## Key Learnings

1. El controller captura la capability staged antes de que el estado pueda invalidarla.
2. Un temporizador global puede mostrar progreso de descarga para mensajes sin YouTube.
3. Esperar un child antes de drenar sus pipes puede provocar un deadlock reproducible.
4. Los cuatro slices superaron ampliamente el presupuesto canónico de revisión.
