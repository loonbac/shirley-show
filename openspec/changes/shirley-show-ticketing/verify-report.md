```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:f97c899559a934574aa94e503917b7f4b40a1022d4b91988cbed20c65ca5e67e
verdict: fail
blockers: 2
critical_findings: 2
requirements: 30/31
scenarios: 66/66
test_command: env SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-4899ad8-full cargo test
test_exit_code: 0
test_output_hash: sha256:0a2d1bf972656632c564f530ec2e009e185ab9b3a8760bb75214eef98addedd5
build_command: cargo build
build_exit_code: 0
build_output_hash: sha256:88d370b7115a0914965105ad3447da73d022ff47db23cf5c9be94ffa4f577915
```

# Verify Report: `shirley-show-ticketing`

## Veredicto

**FAIL — no está listo para sync ni archive.** El commit `4899ad8b4761ac9368c8ce53c5e902a5dbfa4617` corrige funcionalmente C2, C3 y C4, y completa el schema documental de C7. C5 sigue contradiciendo el requisito atómico de eliminación de evento/afiche, y la nueva prueba concurrente de C4 puede terminar sin ejecutar ninguna aserción. Tests y build están verdes.

## Estado estructurado y alcance

- Estado SDD recibido: `ready`; cambio activo único `shirley-show-ticketing`.
- `actionContext.mode`: `repo-local`.
- Workspace autoritativo y raíz permitida: `/home/loonbac/Proyectos/shirley-show`.
- Implementación verificada en HEAD `4899ad8b4761ac9368c8ce53c5e902a5dbfa4617`.
- Authority de este intento: `sha256:736217420c828cfaad50be49ab53277dd76da8016eeb800b7f4d72e6cd916da3`, work-unit `verify-ticketing`, request `tv-acquire-003`, máximo 500; no se ejecutó settle.
- El árbol contenía únicamente `?? production-data/` antes y después. No se leyó ni modificó `production-data/` ni `.run/`, y no se inició ni tocó el servidor `:3100`.

## Completitud de tareas

- `tasks.md`: **50/50** tareas de implementación marcadas `[x]`.
- Líneas que coinciden con `^\s*- \[ \]`: **ninguna**.
- No existe bloqueador por checkbox.

## Cobertura de specs

Conteo canónico recuperado de ocho specs: **31 requirements y 66 scenarios**.

| Dominio | Requirements | Scenarios | Resultado |
|---|---:|---:|---|
| `checkin` | 3/3 | 8/8 | Completo. |
| `entrada-publica` | 4/4 | 7/7 | Completo. |
| `entradas` | 6/6 | 14/14 | C3 corregido: payload negativo, tipos inválidos y JSON malformado retornan 400. |
| `eventos` | 4/5 | 13/13 | C4 está corregido; la eliminación DB+afiche sigue sin ser atómica ante fallo del filesystem. |
| `gastos` | 3/3 | 7/7 | Completo. |
| `interesados` | 3/3 | 7/7 | Completo. |
| `ressy-ticketing` | 4/4 | 6/6 | C2 corregido: `active_event_id` se conserva y se reenvía en el turno siguiente. |
| `resumen` | 3/3 | 4/4 | Completo. |
| **Total** | **30/31** | **66/66** | **Un requirement incompleto.** |

Los dos scenarios explícitos de eliminación pasan en el camino normal y en 404; el texto normativo del requirement también exige atomicidad ante fallo, condición que la implementación no satisface.

## Re-verificación de los cinco ex-bloqueadores

| ID | Estado | Evidencia |
|---|---|---|
| C2 | ✅ Corregido | `chat-state.js` consume `response.active_event_id`, crea contexto `type: 'event'` y las pruebas verifican el reenvío en el segundo turno. |
| C3 | ✅ Corregido | Los handlers de venta, edición de contacto y check-in capturan `JsonRejection`; los casos `cantidad: -2`, tipo incorrecto y JSON malformado pasan con HTTP 400. |
| C4 | ✅ Código corregido / ❌ evidencia débil | `update_event` usa una transacción SQLite `IMMEDIATE`, comparación exacta de tipo y piso de capacidad. La prueba concurrente tiene una rama sin aserciones. |
| C5 | ❌ Parcial | Se rechaza `image/jpg` y el reemplazo propaga el fallo del afiche anterior con compensación DB. Sin embargo, borrar un evento confirma primero SQLite y solo después intenta el filesystem, devolviendo `cleanup_pending`; además se ignora el fallo de borrado tras mover a `.trash`. |
| C7 | ✅ Schema corregido / ❌ auditoría de assertion quality | La tabla tiene las ocho columnas requeridas y 57/57 filas completas; todos sus paths/comandos existen. La prueba concurrente nueva incumple la calidad de aserciones strict-TDD. |

## Hallazgos CRITICAL y bloqueadores exactos

### 1. La eliminación de evento y afiche no cumple la atomicidad normativa

`openspec/changes/shirley-show-ticketing/specs/eventos/spec.md:96` exige eliminar atómicamente evento, hijos y afiche. `src/use_cases/ticketing/events.rs:179-192` confirma primero la eliminación de SQLite y, si luego falla el filesystem, conserva la eliminación de datos y responde `cleanup_pending = true`. Eso implementa recuperación eventual, no todo-o-nada. Adicionalmente, `src/infra/filesystem/local_poster_storage.rs:168` descarta el resultado de `fs::remove_file(&trash_target)` y puede devolver `Ok(true)` aunque el archivo permanezca en `.trash`.

**Impacto:** el requirement `Event Deletion with Cascade and Asset Cleanup` permanece incompleto y C5 no está cerrado.

### 2. La prueba concurrente de C4 puede pasar sin ninguna aserción

`tests/ticketing_event_repo_tests.rs:550-555` solo comprueba `res2.is_err()` dentro de `if res1.is_ok()`. Si la actualización gana la carrera y la venta falla —o si ambas operaciones fallan por otro motivo— el test termina verde sin afirmar estado final, resultado de actualización ni invariantes persistidas. Strict TDD clasifica como CRITICAL una prueba cuya precondición puede impedir que se ejecute el camino de aserción.

**Impacto:** la implementación parece correcta por inspección y por las pruebas secuenciales, pero la evidencia concurrente reportada para C4/C7 no demuestra ambos órdenes válidos de la carrera.

## Strict TDD Compliance

Strict TDD está activo por `openspec/config.yaml`; se aplicó la guía global `~/.pi/agent/gentle-ai/support/strict-tdd-verify.md`.

| Check | Resultado | Detalle |
|---|---|---|
| TDD Evidence reported | ✅ | Existe `Strict TDD Cycle Evidence Table`. |
| Schema protocolario completo | ✅ | 57/57 filas contienen `Task`, `Test File`, `Layer`, `Safety Net`, `RED`, `GREEN`, `TRIANGULATE`, `REFACTOR`. |
| Test targets verificables | ✅ | 50/50 filas con path apuntan a archivos existentes; 7/7 filas de refactor apuntan a comandos Cargo disponibles. |
| RED confirmado | ✅ | Las 15 filas `N/A (new)` corresponden a archivos ausentes en la base `07ba309`; las demás declaran safety net. |
| GREEN actual | ✅ | `cargo test`: 329 passed, 0 failed, 1 ignored. |
| Triangulación y assertion quality | ❌ | 47 filas declaran triangulación y 10 caso único, pero el test concurrente de C4 puede omitir todas sus aserciones. |

**TDD Compliance: 5/6 checks; CRITICAL por assertion quality.**

## Test Layer Distribution

Distribución declarada y auditada en las 57 filas de evidencia:

| Layer | Filas de evidencia | Herramientas |
|---|---:|---|
| Unit | 17 | Rust test / Node assert |
| Integration | 40 | SQLite temporal, Axum `oneshot`, fake DOM y wrappers Cargo→Node |
| E2E vivo | 0 | Omitido para respetar la prohibición de `:3100` |
| **Total** | **57** | |

## Changed File Coverage

Coverage analysis skipped — `cargo-llvm-cov` no está disponible. No es bloqueador.

## Assertion Quality

| Archivo | Línea | Aserción | Hallazgo | Severidad |
|---|---:|---|---|---|
| `tests/ticketing_event_repo_tests.rs` | 550-555 | `if res1.is_ok() { assert!(res2.is_err()) }` | La condición puede omitir toda aserción en un orden válido de la carrera. | CRITICAL |
| `frontend/test/events-ui.test.js` | 54, 65, 115, 122 | assertions sobre `is-active` / `is-finalized` | Acoplamiento a clases CSS en vez de comportamiento accesible observable. | WARNING |

No se encontraron tautologías, assertions sin llamada a producción ni ghost loops adicionales en las suites relacionadas.

**Assertion quality: 1 CRITICAL, 4 WARNING.**

## Comandos ejecutados

| Comando exacto | Exit | Resultado/hash |
|---|---:|---|
| `env SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-4899ad8-focused cargo test --test ticketing_api_tests --test ticketing_event_repo_tests --test ticketing_poster_storage_tests` | 0 | 18 passed; `sha256:b2198090d338d5d526a3d13c496745a8876938f3dbae086c70328c455fdfc2c8`. |
| `node tests/agent-chat-integration.test.mjs` | 0 | 12 grupos verdes; `sha256:4c8ad2d6ee3cc1e99f4202507a097750c88bf46394ff21533f1bc5f008831229`. |
| `node tests/agent-chat-state.test.mjs` | 0 | 18 grupos verdes; `sha256:4ed991c068714cb4794d32d196f4388ad9c48cfcfce2ce2c1dab1ab0f6d553a1`. |
| `env SHIRLEY_DATA_DIR=/tmp/shirley-show-verify-4899ad8-full cargo test` | 0 | 329 passed, 0 failed, 1 ignored; `sha256:0a2d1bf972656632c564f530ec2e009e185ab9b3a8760bb75214eef98addedd5`. |
| `cargo build` | 0 | `sha256:88d370b7115a0914965105ad3447da73d022ff47db23cf5c9be94ffa4f577915`. |
| `cargo fmt --check` | 0 | Salida vacía; `sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. |
| `cargo clippy --all-targets --all-features -- -D warnings` | 101 | `clippy::collapsible-if` en `src/use_cases/ticketing/posters.rs:56`; `sha256:8b4e8fa0c6b7f85cb14473c1033ee9d0b3ce9fbd1089dc53ff7b96460e2c6dea`. |
| `git diff --check 4899ad8^ 4899ad8` | 2 | Dos líneas con trailing whitespace en `apply-progress.md`. |

El fallo de clippy y el whitespace son WARNING de calidad, no bloqueadores funcionales por sí mismos.

## Diseño y coherencia modular

La dirección de dependencias es coherente: handlers y Ressy consumen casos de uso; SQLite/filesystem permanecen tras puertos; frontend conserva el contexto de evento sin mezclarlo con presentación. C2–C4 respetan los límites diseñados.

Persiste una divergencia explícita entre artefactos: la spec exige atomicidad DB+filesystem, mientras `design.md` reconoce que no existe transacción distribuida y adopta limpieza recuperable con `cleanup_pending`. La implementación sigue el diseño, no el requirement normativo.

## Review Workload / frontera de entrega

- `tasks.md` recomendó seis slices encadenados; los seis slices sí aparecen como seis commits separados (`bd54faa` a `03fa345`), aunque `Chain strategy` permanece `pending` y no hay evidencia de PRs encadenadas.
- La remediación `4899ad8` cambia 784 líneas de código/tests (721 inserciones y 63 eliminaciones, excluyendo `apply-progress.md` y `verify-report.md`), por encima del máximo 500 declarado y del presupuesto canónico 400.
- No se registró `size:exception`. Esto es **WARNING de gobernanza/revisión**; el scope nominal permanece concentrado en los cinco bloqueadores.

## Siguiente recomendación

Volver a `sdd-apply` para: (1) reconciliar la semántica normativa de eliminación atómica con el diseño/implementación recuperable y propagar el borrado en `.trash`; (2) hacer que la prueba concurrente afirme ambos órdenes posibles y el estado final. Después repetir `sdd-verify`. No ejecutar sync, archive, settle, publish ni deploy.

## Key Learnings

1. Una transacción SQLite no puede hacer atómico por sí sola un borrado posterior en filesystem.
2. Una prueba concurrente debe afirmar todos los órdenes válidos y nunca depender de una rama opcional.
3. La evidencia TDD protocolaria puede estar completa y aun fallar por calidad real de las aserciones.
4. Mover un archivo a cuarentena no demuestra su eliminación si se ignora el resultado final.
