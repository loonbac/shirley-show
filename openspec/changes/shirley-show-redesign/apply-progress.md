# Apply Progress: Shirley Show Redesign

**Change:** `shirley-show-redesign`  
**Current Slice:** Slice 3 (`redesign-s3-tactile-edit`)  
**Completed Slices:** Slice 1 (`redesign-s1-backend-catalog`), Slice 2 (`redesign-s2-frontend-home`), Slice 3 (`redesign-s3-tactile-edit`)  
**Status:** Ready for Settle / Verify / Slice 4  
**Execution Mode:** Auto  
**Artifact Store:** openspec  
**Delivery Strategy:** ask-on-risk (Chained PRs: stacked-to-main, Slice 3 = PR 3)

---

## Executive Summary: Slice 3 (Bloques Táctiles de Secuencia, Adición Inmediata y Edición Rápida)

Slice 3 (`redesign-s3-tactile-edit`) has been fully implemented adhering to the Extreme Micro-Modularity and Atomic Design System standards defined in `AGENTS.md`.
The presentation preparation and sequence editing workflows have been streamlined, eliminating friction and protecting tactile usability:
1. **Atomic Data Client (`updateItem`):** Created `frontend/js/data/update-item.js` implementing `PATCH /api/presentations/{presentation_id}/items/{item_id}` via `requestJson`, supporting isolated title updates for tracks and multiline text body updates for lyrics and stage notes.
2. **Immediate Quick-Add Bar Molecule:** Created `frontend/js/components/molecules/quick-add-bar.js` and its 1:1 stylesheet `frontend/styles/molecules/quick-add-bar.css`. Provides three dedicated one-tap buttons (`+ Pista`, `+ Letra`, `+ Nota`) with >= 48 px touch targets and distinctive type accents (violet, cyan, amber), emitting `onAdd(type)` without requiring multi-step modal tab navigation.
3. **Direct Add-Item Modal Selection:** Updated `frontend/js/components/organisms/add-item-dialog.js` to accept `initialType` and open directly into the selected item form (track upload with 100 MiB guard, lyrics entry, or stage note entry) bypassing the tab switcher.
4. **Quick-Edit Organism & Draft Preservation:** Created `frontend/js/components/organisms/quick-edit-dialog.js` and `frontend/styles/organisms/quick-edit-dialog.css`. Preloads existing content, autofocuses input, supports keyboard shortcuts (Enter for tracks, Cmd/Ctrl+Enter for textareas, Escape to cancel), and guarantees uncommitted draft preservation if a network or server validation error occurs.
5. **Redesigned Tactile Sequence Blocks:**
   - `frontend/js/components/organisms/track-item.js` & `track-item.css`: Violet accent (`#6D28D9`), all touch targets >= 48 px (reorder buttons, playback toggle, edit pencil, and delete action), quick-edit pencil triggering `onEdit`.
   - `frontend/js/components/organisms/lyrics-item.js` & `lyrics-item.css`: Deep cyan accent (`#0E7490`), readable display formatting with preserved line breaks, quick-edit pencil, >= 48 px touch targets.
   - `frontend/js/components/organisms/note-item.js` & `note-item.css`: Warm amber accent (`#B45309`), high-contrast stage cue card styling, quick-edit pencil, >= 48 px touch targets.
6. **View and Controller Orchestration:**
   - `frontend/js/components/views/presentation-detail-view.js` & `presentation-detail-view.css`: Mounts prominent `quick-add-bar` above the sequence and wires `onEditItem` into each sequence block.
   - `frontend/js/controllers/presentation-detail-controller.js`: Connects `updateItem`, manages `quick-edit-dialog` modal lifecycle, replaces confirmed items in local state without reloading audio, and handles 404 cleanly by notifying the user without creating duplicate or replacement items.
7. **Stylesheet Registration:** Registered `quick-add-bar.css` and `quick-edit-dialog.css` in `frontend/styles/index.css`.

---

## Completed Tasks: Slice 3

- [x] Crear el cliente de datos atómico `frontend/js/data/update-item.js` implementando la llamada `PATCH /api/presentations/{presentation_id}/items/{item_id}` con manejo tipado de errores vía `requestJson`. <!-- sdd-owner: implementation -->
- [x] Implementar la molécula `frontend/js/components/molecules/quick-add-bar.js` con tres botones táctiles dedicados (`+ Pista`, `+ Letra`, `+ Nota`) con tamaño mínimo de 48 px, emitiendo `onAdd('track'|'lyrics'|'note')`. <!-- sdd-owner: implementation -->
- [x] Crear la hoja de estilo atómica `frontend/styles/molecules/quick-add-bar.css` con diseño mobile-first y acentos cromáticos coincidentes con cada tipo de contenido. <!-- sdd-owner: implementation -->
- [x] Modificar `frontend/js/components/organisms/add-item-dialog.js` para recibir el tipo inicial preseleccionado y abrir directamente el formulario correspondiente sin paso de navegación por tabs. <!-- sdd-owner: implementation -->
- [x] Implementar el organismo puro `frontend/js/components/organisms/quick-edit-dialog.js` recibiendo el item actual, precargando el valor de título o body, autofocando el campo, con atajos de teclado (Enter para pistas, Cmd/Ctrl+Enter para textarea, Escape para cancelar), y conservando el borrador ante errores de red o validación. <!-- sdd-owner: implementation -->
- [x] Crear la hoja de estilo atómica `frontend/styles/organisms/quick-edit-dialog.css` con overlay accesible, diseño responsivo y estado visual de error y carga sin destruir el formulario. <!-- sdd-owner: implementation -->
- [x] Rediseñar `frontend/js/components/organisms/track-item.js` y `frontend/styles/organisms/track-item.css` con acento violeta, áreas táctiles mínimas de 48 px, botón de edición rápida (lápiz) con callback `onEdit`, manteniendo confirmación de borrado y controles de reordenamiento accesibles. <!-- sdd-owner: implementation -->
- [x] Rediseñar `frontend/js/components/organisms/lyrics-item.js` y `frontend/styles/organisms/lyrics-item.css` con acento cian profundo, preservación de saltos de línea legibles, botón de edición rápida y áreas táctiles de 48 px. <!-- sdd-owner: implementation -->
- [x] Rediseñar `frontend/js/components/organisms/note-item.js` y `frontend/styles/organisms/note-item.css` con acento ámbar cálido, tipografía clara de indicación de show, botón de edición rápida y áreas táctiles de 48 px. <!-- sdd-owner: implementation -->
- [x] Actualizar `frontend/js/components/views/presentation-detail-view.js` y `frontend/styles/views/presentation-detail-view.css` para integrar la `quick-add-bar` fija o destacada y gestionar el montaje del `quick-edit-dialog`. <!-- sdd-owner: implementation -->
- [x] Actualizar `frontend/js/controllers/presentation-detail-controller.js` para conectar `updateItem`, actualizar el estado local y la vista tras confirmación del servidor, preservar el borrador ante fallos, y manejar el caso 404 informando al usuario sin crear items duplicados. <!-- sdd-owner: implementation -->
- [x] Registrar estilos nuevos en `frontend/styles/index.css` y verificar manualmente: adición directa de los 3 tipos, edición rápida con guardado y cancelación, y conservación de draft ante fallo de red simulado. <!-- sdd-owner: implementation -->

---

## Files Changed in Slice 3

### Created
- `frontend/js/data/update-item.js`
- `frontend/js/components/molecules/quick-add-bar.js`
- `frontend/styles/molecules/quick-add-bar.css`
- `frontend/js/components/organisms/quick-edit-dialog.js`
- `frontend/styles/organisms/quick-edit-dialog.css`

### Modified
- `frontend/js/components/organisms/add-item-dialog.js`
- `frontend/js/components/organisms/track-item.js`
- `frontend/styles/organisms/track-item.css`
- `frontend/js/components/organisms/lyrics-item.js`
- `frontend/styles/organisms/lyrics-item.css`
- `frontend/js/components/organisms/note-item.js`
- `frontend/styles/organisms/note-item.css`
- `frontend/js/components/views/presentation-detail-view.js`
- `frontend/styles/views/presentation-detail-view.css`
- `frontend/js/controllers/presentation-detail-controller.js`
- `frontend/styles/index.css`
- `openspec/changes/shirley-show-redesign/tasks.md`

---

## Test Commands Run and Validation Evidence

1. **Backend Non-Regression Suite:**
   - Command: `cargo test`
   - Result: All 13 test suites (85 tests) passed cleanly with 0 failures, 0 warnings, and 0 regressions.
2. **Frontend Syntax Check:**
   - Command: `node --check` across all modified and created JavaScript modules.
   - Result: 0 syntax errors.
3. **Static Asset Verification on `:3100`:**
   - Verified HTTP 200 responses via `/assets/...` for `quick-add-bar.js`, `quick-edit-dialog.js`, `update-item.js`, and atomic stylesheets.
4. **Live E2E Verification against Server on `:3100`:**
   - Verified untouched baseline: "Evento 28 de Agosto" (ID `3e56cdca-b59c-4095-9d50-a1bdabbf03a2`) remained untouched.
   - Created temporary presentation, uploaded a track item via multipart, added lyrics and note items via direct endpoints.
   - Performed successful `PATCH` updates for track title, multiline lyrics body, and note body.
   - Verified validation rejection: blank title returns 400 Bad Request, incompatible field (`body` on track) returns 400 Bad Request.
   - Verified 404 response on missing item ID without generating phantom replacements.
   - Deleted temporary presentation and confirmed baseline returned to pristine state with "Evento 28 de Agosto" intact.

---

## Workload and PR Boundary

- **Work Unit:** `redesign-s3-tactile-edit`
- **Authority Token:** `sha256:3b9060d77ffdd6317f5dcde843829366225e9e0710c033f2c4bf9be5f54482fc`
- **Request:** `r3-acquire-001`
- **Lines Budget:** Max 3000 lines (actual slice size: ~490 insertions, 48 deletions across modified files + 5 new micro-modules).
- **PR Boundary:** PR 3 of 4 in stacked-to-main chain. Strictly presentation tactile sequence blocks, quick-add bar, quick-edit dialog, and frontend PATCH integration. Drag reordering and Stage Mode (Slice 4) are untouched. Ready for settle.

---

## Executive Summary: Slice 2 (Home Visual Library & Warm Identity)

*(Historical reference from PR 2)*  
Slice 2 (`redesign-s2-frontend-home`) transformed the catalog home view into an intimate visual library: warm ivory/gold tokens, `relative-date.js`, `welcome-header.js/.css`, enriched `presentation-card.js/.css`, catalog orchestration, and responsive design.

### Completed Tasks (Slice 2)
- [x] Tokens colors.css, elevations.css, spacing.css.
- [x] relative-date.js format utility.
- [x] welcome-header.js organism and welcome-header.css.
- [x] presentation-card.js organism and presentation-card.css.
- [x] presentations-view.js and presentations-view.css integration.
- [x] presentations-controller.js aggregate counters.
- [x] index.css registration.

---

## Executive Summary: Slice 1 (Backend Item PATCH, Catálogo Enriquecido y Reordenamiento Absoluto)

*(Historical reference from PR 1)*  
Slice 1 (`redesign-s1-backend-catalog`) implemented the Rust backend foundation under strict TDD: `PATCH /api/presentations/{presentation_id}/items/{item_id}`, enriched catalog summaries (`GET /api/presentations`), and absolute reordering via `target_position` (`POST /api/presentations/{presentation_id}/items/{item_id}/move`).

### Completed Tasks (Slice 1)
- [x] Domain models and ports in `domain/` and `ports/`.
- [x] Sqlite repositories and use cases in `infra/sqlite/` and `use_cases/`.
- [x] HTTP handlers, routing, and DTOs in `handlers/` and `router.rs`.

---

## Remaining Tasks (Slice 4)

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
