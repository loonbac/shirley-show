```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:5b198dffcc41b40be2ac10c9a24c3eb4c60238c8df3a213982c04d757761ce05
verdict: fail
blockers: 6
critical_findings: 6
requirements: 18/23
scenarios: 69/70
test_command: cargo test
test_exit_code: 0
test_output_hash: sha256:05ec16298b5594df96eb6c3f17d35c07d44879d35b8cd942ac070f30918efef0
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:a5f4c585ee974ca44916ac30a98bbc189e067a7e0a6bc6d2e8d6bc525be724af
```

# SDD Verify Report: `shirley-show-ai-agent`

## Estado

**FAIL — no está listo para sync ni archive.** Los dos commits solicitados están presentes (`8ebe683` S1 y `bb0783d` S2), las 28 tareas están marcadas, los tests y el build pasan, pero hay seis hallazgos CRITICAL: cinco incumplimientos de requisitos MUST y evidencia TDD incompleta/inexacta.

## Cobertura de especificación

Se recuperaron y auditaron las tres specs canónicas: **23 requisitos y 70 escenarios**.

| Dominio | Requisitos completos | Escenarios completos | Resultado |
|---|---:|---:|---|
| `agent-chat` | 7/8 | 23/23 | FAIL: límites de history no se aplican estrictamente en todos los casos |
| `agent-tools` | 5/7 | 22/22 | FAIL: argumentos extra aceptados y deadline no global |
| `agent-chat-ui` | 6/8 | 24/25 | FAIL: refresh no preserva scroll y fallos HTTP 200 pierden draft/retry |
| **Total** | **18/23** | **69/70** | **FAIL** |

Cobertura confirmada:

- `POST /api/agent/chat` acepta JSON y multipart; devuelve 400 para mensaje JSON vacío y 415 para `text/plain`.
- La degradación con la configuración placeholder fue amable: HTTP 200, `notice=provider_auth`, mensaje no vacío y `actions_taken=[]`; no se instaló ni usó una key real.
- Multipart válido e inválido dejó `staging/` vacío; `.flac` devolvió 400 `unsupported_format`.
- El dispatcher expone seis tools, valida IDs conocidos, deduplica por call ID/firma y registra únicamente mutaciones confirmadas en `actions_taken`.
- La UI implementa panel desktop de 580 px a 1280 px, fullscreen móvil `100vw × 100dvh`, safe areas, burbujas por rol, typing dorado, adjunto único, `Ctrl/Cmd+K`, `Escape`, texto mediante nodos seguros y reglas reduced-motion.
- El E2E HTTP creó una presentación temporal, añadió una nota, verificó su detalle y la borró. `Evento 28 de Agosto` conservó el mismo digest antes y después: `sha256:1726cba20a32b11302f504be4c346e5f909b43380f55b9b7d6ba9a699e690a4b`.

## Completitud de tareas

- `tasks.md`: **28/28** tareas de implementación marcadas.
- Marcadores exactos `- [ ]` restantes: **ninguno**.
- `apply-progress.md`: incluye tabla `TDD Cycle Evidence` para WU 1.1–2.3.

## Hallazgos CRITICAL y blockers

1. **La evidencia strict-TDD declara cobertura que no existe en los tests.** `apply-progress.md` afirma pruebas de history truncation, adjunto >100 MiB, cleanup ante provider error/timeout, viewport 580 px/fullscreen, focus/Escape, contexto activo y refresh selectivo. El árbol no contiene esas aserciones: `tests/agent_chat_api_tests.rs` no prueba history ni un archivo >100 MiB; `tests/agent-chat-ui-components.test.mjs` no prueba 580 px, `100dvh`, safe areas ni Escape/focus restoration; `tests/agent-chat-integration.test.mjs` importa `emitAgentRefresh` pero no lo ejecuta y no espera ni comprueba el contexto cargado del detalle. Bajo strict TDD, esta evidencia incompleta es blocker de archivo.

2. **El deadline de 30 segundos no cubre toda la request.** `src/handlers/agent/chat.rs:31` espera todo el parsing/staging multipart y solo después llama al agente en `src/handlers/agent/chat.rs:39`; el timeout empieza dentro de `AgentChat` en `src/use_cases/agent_chat.rs:122`. Un cliente multipart lento puede mantener parsing y escritura fuera del límite global requerido.

3. **El límite de history puede superar 32.000 caracteres.** `src/use_cases/agent_chat.rs:261` acepta el mensaje más reciente aunque exceda el presupuesto cuando `selected_rev` está vacío; además `src/use_cases/agent_chat.rs:254` cuenta bytes con `String::len`, no caracteres, y la nota de poda se inserta como `AgentMessage::User` en `src/use_cases/agent_chat.rs:101`, no como nota de sistema. Esto incumple el guardrail de input no confiable.

4. **La validación server-side de argumentos de tools no es estricta.** Los structs en `src/agent/tool_dispatcher.rs:22-41` derivan `Deserialize` sin `#[serde(deny_unknown_fields)]`; por ello aceptan propiedades extra aunque el schema publicado anuncie `additionalProperties: false`. El requisito exige validar estrictamente antes de ejecutar operaciones.

5. **El refresh reactivo del detalle no preserva la posición de scroll.** El subscriber en `frontend/js/controllers/presentation-detail-controller.js:194-197` llama directamente a `loadData()` sin capturar/restaurar scroll o anchor. Falla el escenario “Reactive sequence reload after assistant adds lyrics, note, or track”.

6. **Los errores conversacionales HTTP 200 pierden recuperación segura del draft.** `frontend/js/core/state/chat-state.js:146-149` limpia siempre `draft` en `resolveTurn`, incluso cuando la respuesta contiene `provider_auth`, `provider_rate_limited`, `provider_unavailable` o `agent_timeout`. Además, el backend entrega notices sin `message`, mientras `frontend/js/components/organisms/agent-chat-panel.js:193` solo renderiza notice/retry si `notice.message` existe. Esto incumple retención y retry accionable para fallos de proveedor.

## Strict TDD y calidad de aserciones

- Modo strict TDD: **activo** en `openspec/config.yaml`, `tasks.md` y `apply-progress.md`.
- Tabla TDD: presente, pero no concuerda por completo con los archivos reales; ver blocker 1.
- GREEN actual: `cargo test` pasó **126 tests**, 0 fallos.
- Las pruebas de dispatcher, loop y API usan aserciones observables y fixtures temporales; no se encontraron tautologías, ghost loops ni aserciones solo de tipos.
- La suite UI contiene comprobaciones útiles de XSS, pending y reduced-motion, pero también cobertura smoke-only (`innerHTML.length > 0`, presencia de clases) y omite dimensiones/layout real, focus restoration, refresh disparado y scroll preservation.

## Estado estructurado y actionContext

- Cambio activo: `shirley-show-ai-agent`, selección inequívoca.
- Estado nativo recibido: `ready`; store autoritativo `openspec`.
- `actionContext.mode`: `repo-local`.
- Workspace y raíz permitida: `/home/loonbac/Proyectos/shirley-show`.
- Ownership probado dentro de la raíz autorizada.
- Dependencia apply: `all_done`; verify pasa a **failed** por los blockers anteriores.

## Review workload / frontera de entrega

- Forecast: riesgo alto sobre 400 líneas, chained PRs recomendados y estrategia inicialmente pendiente.
- Implementación observada: dos slices/commits separados, S1 con 5.684 inserciones/30 borrados y S2 con 3.701 inserciones/97 borrados.
- `apply-progress.md` registra explícitamente “2 PRs con `size:exception` acordado”; no se infiere una excepción nueva.
- La frontera S1 backend / S2 frontend coincide con los dos work units entregados, pero ambos exceden ampliamente el presupuesto canónico; queda riesgo de revisión aunque la excepción esté registrada.

## Comandos y resultados

| Comando exacto | Exit | Evidencia |
|---|---:|---|
| `cargo test` | 0 | 126 passed, 0 failed; `sha256:05ec16298b5594df96eb6c3f17d35c07d44879d35b8cd942ac070f30918efef0` |
| `cargo build` | 0 | build dev completado; `sha256:a5f4c585ee974ca44916ac30a98bbc189e067a7e0a6bc6d2e8d6bc525be724af` |
| `cargo fmt --all -- --check` | 1 | informativo: diffs de rustfmt en código y tests; `sha256:40e25896e00be1e0e16504f7793d815e1a7415b1d52584a79d3acb8ac7deeae3` |
| Script E2E HTTP con `curl` contra `http://127.0.0.1:3100` | 0 | JSON/multipart/degradación/400/415/format/staging/temp cleanup; `sha256:5fb61945fa32c72295baee266467137de4707306de58afe9e1d6ffded179adba` |

No se dispuso de un navegador automatizable en este executor; las condiciones visuales se auditaron mediante CSS/DOM y el harness Node existente, no mediante medición renderizada independiente.

## Higiene de secretos y árbol

- `.env` existe localmente y está ignorado por `.gitignore:7`; no se leyó ni publicó ningún valor.
- El único archivo de entorno trackeado es `.env.example`.
- La búsqueda de patrones no encontró tokens Groq trackeados; la coincidencia textual restante corresponde a la especificación que menciona el nombre de la variable.
- `staging/` terminó con 0 archivos y la presentación temporal fue purgada.
- `cargo fmt --check` falló; es WARNING informativo solicitado, no uno de los blockers funcionales.

## Veredicto y siguiente paso

**FAIL.** Corregir los seis blockers, reconciliar la tabla TDD con pruebas reales y repetir `sdd-verify`. No ejecutar sync ni archive todavía.

## Key Learnings

1. El timeout del agente no limita por sí solo el parsing multipart previo.
2. Los schemas publicados requieren validación Serde equivalente en el dispatcher.
3. Los notices HTTP 200 necesitan preservar draft y exponer copy de retry.
4. La evidencia TDD debe corresponder a aserciones presentes en el árbol real.
