# Apply Progress: Shirley Show Redesign

**Change:** `shirley-show-redesign`  
**Current Slice:** Slice 2 (`redesign-s2-frontend-home`)  
**Completed Slices:** Slice 1 (`redesign-s1-backend-catalog`), Slice 2 (`redesign-s2-frontend-home`)  
**Status:** Ready for Settle / Verify / Slice 3  
**Execution Mode:** Auto  
**Artifact Store:** openspec  
**Delivery Strategy:** ask-on-risk (Chained PRs: stacked-to-main, Slice 2 = PR 2)

---

## Executive Summary: Slice 2 (Home Visual Library & Warm Identity)

Slice 2 (`redesign-s2-frontend-home`) has been fully implemented adhering to the Extreme Micro-Modularity and Atomic Design System standards defined in `AGENTS.md`.
The catalog home view has been transformed into an intimate, warm, and elegant visual library for Shirley:
1. **Design Tokens & Palette:** Enhanced `frontend/styles/tokens/colors.css` with warm ivory (`#FAF8F5`), sober metallic antique gold (`#B8860B`), and semantic jewel accents (violet `#6D28D9` for tracks, deep cyan `#0E7490` for lyrics, warm amber `#B45309` for notes). `elevations.css` incorporates soft warm paper shadows (`--shadow-paper`, `--shadow-paper-hover`). `spacing.css` standardizes the action touch target (`--touch-target-action: 48px`).
2. **Pure Format Utility:** Created `frontend/js/utils/format/relative-date.js` providing human-readable recency in Spanish ("Actualizado hoy", "Actualizado ayer", "Hace 2 días", "Hace 1 semana", etc.) with safe neutral fallback for invalid dates.
3. **Welcome Header Organism:** Created `frontend/js/components/organisms/welcome-header.js` and its 1:1 atomic stylesheet `frontend/styles/organisms/welcome-header.css`. Renders a personal greeting, purpose subtitle, warm stage quote, aggregate library counters (total repertoires and audio tracks), and a prominent "Nueva Presentación" action button with >= 48 px touch target.
4. **Enriched Presentation Card Organism:** Redesigned `frontend/js/components/organisms/presentation-card.js` and `frontend/styles/organisms/presentation-card.css`. Renders repertoire title link, relative recency, color-coded badges for each item kind, an ordered preview box showing the first 3 items with distinct icons/snippets (or "Repertorio sin elementos aún"), a dominant full-width "Abrir show" primary button (>= 48 px touch target), and discrete contained rename/delete secondary actions.
5. **View & Controller Integration:** Updated `frontend/js/components/views/presentations-view.js` and `frontend/styles/views/presentations-view.css` to orchestrate `welcome-header` and the card grid when repertoires exist, while preserving the canonical empty state when the library is empty. Updated `frontend/js/controllers/presentations-controller.js` to compute aggregate totals from the enriched `listPresentations()` response and link card selection directly to presentation details.
6. **Stylesheet Registration:** Registered `welcome-header.css` in `frontend/styles/index.css` and verified responsive layout across mobile and desktop.

---

## Completed Tasks: Slice 2

- [x] Ajustar tokens en `frontend/styles/tokens/colors.css`, `elevations.css` y `spacing.css` incorporando la paleta marfil cálido (`#FAF8F5`), oro envejecido (`#B8860B`), acentos temáticos (violeta `#6D28D9`, cian profundo `#0E7490`, ámbar cálido `#B45309`) y elevaciones suaves de papel. <!-- sdd-owner: implementation -->
- [x] Implementar la micro-utilidad pura `frontend/js/utils/format/relative-date.js` para formatear timestamps ISO en texto humano en español ("Actualizado hoy", "Hace 2 días", neutral fallback para fechas inválidas). <!-- sdd-owner: implementation -->
- [x] Implementar el organismo puro `frontend/js/components/organisms/welcome-header.js` que recibe saludo personalizado, métricas agregadas de la biblioteca (repertorios y pistas) y callback `onCreatePresentation`. <!-- sdd-owner: implementation -->
- [x] Crear la hoja de estilo atómica 1:1 `frontend/styles/organisms/welcome-header.css` con tipografía cálida, distribución responsive y botón de acción primaria con área táctil mínima de 48 px. <!-- sdd-owner: implementation -->
- [x] Rediseñar el organismo `frontend/js/components/organisms/presentation-card.js` para recibir el summary enriquecido (`tracks_count`, `lyrics_count`, `notes_count`, `total_items`, `preview_items`), renderizar fecha humana, badges cromáticos por tipo, caja de preview con los primeros 3 temas/snippets, botón dominante "Abrir show" (touch target >= 48 px) y menú discreto para renombrar/eliminar. <!-- sdd-owner: implementation -->
- [x] Rediseñar la hoja de estilo `frontend/styles/organisms/presentation-card.css` con elevación física, estados interactivos claros (hover/active/focus-visible) y separación visual entre acción primaria y destructivas. <!-- sdd-owner: implementation -->
- [x] Actualizar `frontend/js/components/views/presentations-view.js` y `frontend/styles/views/presentations-view.css` para orquestar `welcome-header` cuando hay repertorios, mantener el `empty-state` canónico cuando la lista está vacía, y montar el grid de tarjetas ricas. <!-- sdd-owner: implementation -->
- [x] Actualizar `frontend/js/controllers/presentations-controller.js` para consumir los datos enriquecidos de `listPresentations()`, calcular totales agregados para el header y enlazar el callback de "Abrir show" con la navegación directa. <!-- sdd-owner: implementation -->
- [x] Registrar `welcome-header.css` en `frontend/styles/index.css` y verificar visualmente en navegador con 0 presentaciones, con 1 presentación sin items y con múltiples presentaciones con items variados. <!-- sdd-owner: implementation -->

---

## Files Changed in Slice 2

### Created
- `frontend/js/utils/format/relative-date.js`
- `frontend/js/components/organisms/welcome-header.js`
- `frontend/styles/organisms/welcome-header.css`

### Modified
- `frontend/styles/tokens/colors.css`
- `frontend/styles/tokens/elevations.css`
- `frontend/styles/tokens/spacing.css`
- `frontend/styles/index.css`
- `frontend/styles/organisms/presentation-card.css`
- `frontend/styles/views/presentations-view.css`
- `frontend/js/components/organisms/presentation-card.js`
- `frontend/js/components/views/presentations-view.js`
- `frontend/js/controllers/presentations-controller.js`
- `frontend/js/data/list-presentations.js`
- `openspec/changes/shirley-show-redesign/tasks.md`

---

## Test Commands Run and Validation Evidence

1. **Backend Non-Regression Suite:**
   - Command: `cargo test`
   - Result: All 13 test suites (73 tests) passed cleanly with 0 failures, 0 warnings, and 0 regressions.
2. **Date Formatter Micro-Tests:**
   - Validated in Node runtime: null / undefined / invalid string handled gracefully; same-day formatted as "Actualizado hoy", yesterday as "Actualizado ayer", recent days as "Hace N días", weeks as "Hace N semanas", months as "Hace N meses".
3. **Live E2E Verification against Server on `:3100`:**
   - Verified 0 presentations scenario: renders canonical empty state with creation button, omitting aggregate counters.
   - Verified 1 presentation with 0 items: renders welcome header with 1 repertoire, 0 tracks, card displays zero badges, "Repertorio sin elementos aún", and prominent "Abrir show" button.
   - Verified multi-item presentation lifecycle: created temporary presentation with lyrics and notes items, verified enriched summary in `GET /api/presentations`, verified preview snippets and counts, and performed clean cascade deletion.

---

## Workload and PR Boundary

- **Work Unit:** `redesign-s2-frontend-home`
- **Authority Token:** `sha256:defc88bbcda04c0ca523134fdd5353c584539b8cf0af8a28e92633dc43ff2042`
- **Request:** `r2-acquire-001`
- **Lines Budget:** Max 3000 lines (actual slice size: ~410 insertions, 84 deletions across modified files + 3 new micro-modules).
- **PR Boundary:** PR 2 of 4 in stacked-to-main chain. Strictly frontend home library presentation, organisms, and styling. No backend schema or endpoints modified. Ready for settle.

---

## Executive Summary: Slice 1 (Backend Item PATCH, Catálogo Enriquecido y Reordenamiento Absoluto)

*(Historical reference from PR 1)*
Slice 1 (`redesign-s1-backend-catalog`) has been implemented under strict TDD.
All requirements for backend item editing (`PATCH /api/presentations/{presentation_id}/items/{item_id}`), enriched catalog summaries (`GET /api/presentations`), and absolute reordering via `target_position` (`POST /api/presentations/{presentation_id}/items/{item_id}/move`) are complete, validated with targeted unit, repository, use-case, and endpoint integration tests, and verified against all pre-existing regression suites.

### Completed Tasks (Slice 1)
- [x] RED: Escribir tests unitarios para `ShowItemUpdate` y `PresentationSummary` / `ShowItemPreview` en `tests/domain_item_tests.rs`.
- [x] GREEN: Implementar `src/domain/show_item_update.rs` y `src/domain/presentation_summary.rs`; exportar en `src/domain/mod.rs`.
- [x] GREEN: Definir trait `PresentationSummaryReader` en `src/ports/presentation_summary_reader.rs` y extender `ShowItemWriter` en `src/ports/show_item_writer.rs` con `update` y `move_to`.
- [x] RED: Escribir tests de integración en `tests/show_item_repository_tests.rs` para `update` y `move_to`.
- [x] GREEN: Implementar métodos `update` y `move_to` en `src/infra/sqlite/show_item_repository.rs`.
- [x] RED: Escribir tests de repositorio en `tests/presentation_repository_tests.rs` para `PresentationSummaryRepository::list_summaries`.
- [x] GREEN: Implementar `src/infra/sqlite/presentation_summary_repository.rs`.
- [x] GREEN: Implementar `src/use_cases/update_show_item.rs` y actualizar `list_presentations.rs` y `reorder_item.rs`.
- [x] RED: Escribir tests de endpoint en `tests/item_api_tests.rs` y `tests/presentation_api_tests.rs`.
- [x] GREEN: Actualizar DTOs en `src/handlers/dto.rs`, crear handler `src/handlers/items/update.rs`, actualizar `src/handlers/items/reorder.rs` y `src/handlers/presentations/list.rs`, registrar rutas en `src/router.rs` y componer dependencias en `src/state/app_state.rs`.
- [x] TRIANGULATE & REFACTOR: Ejecutar la suite completa con `cargo test`.

---

## Remaining Tasks (Slices 3 & 4)

### Slice 3: Bloques Táctiles de Secuencia, Adición Inmediata y Edición Rápida
- [ ] Crear el cliente de datos atómico `frontend/js/data/update-item.js` implementando la llamada `PATCH /api/presentations/{presentation_id}/items/{item_id}` con manejo tipado de errores vía `requestJson`. <!-- sdd-owner: implementation -->
- [ ] Implementar la molécula `frontend/js/components/molecules/quick-add-bar.js` con tres botones táctiles dedicados (`+ Pista`, `+ Letra`, `+ Nota`) con tamaño mínimo de 48 px, emitiendo `onAdd('track'|'lyrics'|'note')`. <!-- sdd-owner: implementation -->
- [ ] Crear la hoja de estilo atómica `frontend/styles/molecules/quick-add-bar.css` con diseño mobile-first y acentos cromáticos coincidentes con cada tipo de contenido. <!-- sdd-owner: implementation -->
- [ ] Modificar `frontend/js/components/organisms/add-item-dialog.js` para recibir el tipo inicial preseleccionado y abrir directamente el formulario correspondiente sin paso de navegación por tabs. <!-- sdd-owner: implementation -->
- [ ] Implementar el organismo puro `frontend/js/components/organisms/quick-edit-dialog.js` recibiendo el item actual, precargando el valor de título o body, autofocando el campo, con atajos de teclado (Enter para pistas, Cmd/Ctrl+Enter para textarea, Escape para cancelar), y conservando el borrador ante errores de red o validación. <!-- sdd-owner: implementation -->
- [ ] Crear la hoja de estilo atómica `frontend/styles/organisms/quick-edit-dialog.css` con overlay accesible, diseño responsivo y estado visual de error y carga sin destruir el formulario. <!-- sdd-owner: implementation -->
- [ ] Rediseñar `frontend/js/components/organisms/track-item.js` y `frontend/styles/organisms/track-item.css` con acento violeta, áreas táctiles mínimas de 48 px, botón de edición rápida (lápiz) con callback `onEdit`, manteniendo confirmación de borrado y controles de reordenamiento accesibles. <!-- sdd-owner: implementation -->
- [ ] Rediseñar `frontend/js/components/organisms/lyrics-item.js` y `frontend/styles/organisms/lyrics-item.css` con acento cian profundo, preservación de saltos de línea legibles, botón de edición rápida y áreas táctiles de 48 px. <!-- sdd-owner: implementation -->
- [ ] Rediseñar `frontend/js/components/organisms/note-item.js` y `frontend/styles/organisms/note-item.css` con acento ámbar cálido, tipografía clara de indicación de show, botón de edición rápida y áreas táctiles de 48 px. <!-- sdd-owner: implementation -->
- [ ] Actualizar `frontend/js/components/views/presentation-detail-view.js` y `frontend/styles/views/presentation-detail-view.css` para integrar la `quick-add-bar` fija o destacada y gestionar el montaje del `quick-edit-dialog`. <!-- sdd-owner: implementation -->
- [ ] Actualizar `frontend/js/controllers/presentation-detail-controller.js` para conectar `updateItem`, actualizar el estado local y la vista tras confirmación del servidor, preservar el borrador ante fallos, y manejar el caso 404 informando al usuario sin crear items duplicados. <!-- sdd-owner: implementation -->
- [ ] Registrar estilos nuevos en `frontend/styles/index.css` y verificar manualmente: adición directa de los 3 tipos, edición rápida con guardado y cancelación, y conservación de draft ante fallo de red simulado. <!-- sdd-owner: implementation -->

### Slice 4: Reordenamiento por Manija Táctil y Modo Escenario / Atril
- [ ] Actualizar `frontend/js/data/move-item.js` para soportar opcionalmente el parámetro `target_position` en la petición POST hacia `/api/presentations/{id}/items/{itemId}/move`. <!-- sdd-owner: implementation -->
- [ ] Implementar la molécula `frontend/js/components/molecules/drag-handle.js` y su estilo `frontend/styles/molecules/drag-handle.css` con `touch-action: none`, foco por teclado y señal visual clara de arrastre. <!-- sdd-owner: implementation -->
- [ ] Implementar el comportamiento headless `frontend/js/behaviors/use-drag-and-drop.js` capturando pointer events exclusivamente en la manija, calculando umbrales de inserción mediante `requestAnimationFrame`, aplicando feedback visual temporal sin alterar el DOM real, emitiendo `onReorder(itemId, targetPosition)` al soltar dentro de la lista, y restaurando el orden ante `pointercancel` o salida. <!-- sdd-owner: implementation -->
- [ ] Integrar `drag-handle` en `track-item.js`, `lyrics-item.js` y `note-item.js`, asegurando que las zonas fuera de la manija permitan scroll vertical nativo fluido. <!-- sdd-owner: implementation -->
- [ ] Implementar el micro-comportamiento headless `frontend/js/behaviors/use-wake-lock.js` gestionando la adquisición de `navigator.wakeLock.request('screen')`, degradación silenciosa cuando no esté soportado o sea denegado, liberación limpia al desactivar, y re-adquisición automática ante eventos `visibilitychange` si el modo sigue activo. <!-- sdd-owner: implementation -->
- [ ] Adaptar `track-item.js`, `lyrics-item.js` y `note-item.js` para recibir `mode: 'preparation' | 'stage'`: en modo stage se omiten por completo los nodos de edición, borrado, manija y flechas; track renderiza un botón gigante de play/pause (>= 48 × 48 px), lyrics usa tipografía display grande, y note usa tarjeta de alto contraste ámbar. <!-- sdd-owner: implementation -->
- [ ] Implementar el organismo puro `frontend/js/components/organisms/stage-mode-view.js` como overlay fixed de viewport completo, con barra superior fija de safe-area, botón prominente para salir, controles accesibles `A-` / `A+` para regular el tamaño de letra (variable CSS `--stage-lyrics-scale`), indicador sutil de estado de Wake Lock, y contenedor scrollable para la secuencia de items en modo stage. <!-- sdd-owner: implementation -->
- [ ] Crear la hoja de estilo atómica `frontend/styles/organisms/stage-mode-view.css` con fondo oscuro de alto contraste, protección de zonas seguras móviles (`env(safe-area-inset-*)`) y áreas táctiles de control >= 48 px. <!-- sdd-owner: implementation -->
- [ ] Actualizar `frontend/js/controllers/presentation-detail-controller.js` para:
- [ ] Registrar nuevos estilos en `frontend/styles/index.css` y realizar la verificación final manual de todos los criterios de aceptación E2E: drag táctil, scroll nativo, entrada/salida a Modo Escenario durante reproducción activa, exclusividad de pista y fallback cuando Wake Lock o Fullscreen no están disponibles. <!-- sdd-owner: implementation -->
