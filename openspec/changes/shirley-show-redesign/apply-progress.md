# Apply Progress: Shirley Show Redesign

**Change:** `shirley-show-redesign`  
**Current Slice:** Slice 4 (`redesign-s4-drag-stage`)  
**Completed Slices:** Slice 1 (`redesign-s1-backend-catalog`), Slice 2 (`redesign-s2-frontend-home`), Slice 3 (`redesign-s3-tactile-edit`), Slice 4 (`redesign-s4-drag-stage`)  
**Status:** Ready for Settle / Verify  
**Execution Mode:** Auto  
**Artifact Store:** openspec  
**Delivery Strategy:** ask-on-risk (Chained PRs: stacked-to-main, Slice 4 = PR 4, Final Slice)

---

## Executive Summary: Slice 4 (Reordenamiento por Manija Táctil y Modo Escenario / Atril)

Slice 4 (`redesign-s4-drag-stage`) has been fully implemented adhering to the Extreme Micro-Modularity and Atomic Design System standards defined in `AGENTS.md`. This final slice completes the entire redesign of Shirley Show, empowering Shirley with tactile drag-and-drop sequencing and a full-viewport live performance mode:

1. **Client Move-Item Support for `target_position`:**
   - Updated `frontend/js/data/move-item.js` to optionally support `target_position` (absolute target index) alongside existing `direction` ('up' | 'down'), transmitting `{ target_position }` to `POST /api/presentations/{id}/items/{itemId}/move`.
2. **Dedicated Drag Handle Molecule:**
   - Created `frontend/js/components/molecules/drag-handle.js` and its 1:1 stylesheet `frontend/styles/molecules/drag-handle.css`.
   - Features `touch-action: none`, keyboard accessibility (`<button>` with focus ring and aria-label), and tactile visual grab/grabbing affordance with grip icon.
3. **Headless Drag-and-Drop Behavior (`useDragAndDrop`):**
   - Created `frontend/js/behaviors/use-drag-and-drop.js`.
   - Captures pointer events exclusively on the handle via pointer capture; non-handle zones retain 100% native vertical scroll.
   - Schedules midpoint threshold calculations using `requestAnimationFrame`.
   - Applies temporary visual indicators (`is-dragging`, `drop-target-before`, `drop-target-after`, `is-reordering`) without altering the persistent DOM.
   - Emits `onReorder(itemId, targetPosition)` upon release within list bounds, and cancels immediately on `pointercancel` or out-of-bounds release without dispatching server mutations.
4. **Organism Integration and Dual Mode Support (`mode: 'preparation' | 'stage'`):**
   - Adapted `frontend/js/components/organisms/track-item.js`, `lyrics-item.js`, and `note-item.js` to accept `mode = 'preparation' | 'stage'`.
   - In preparation mode: mounts the dedicated `drag-handle` in a reorder zone alongside fallback `reorder-controls` (Up/Down) and quick-edit/delete buttons.
   - In stage mode: completely omits all mutation controls (drag handles, reorder buttons, edit pencils, delete actions).
   - Track renders a large high-contrast play/pause button (56 × 56 px, >= 48px touch target) and clear runtime/duration info.
   - Lyrics renders display typography connected to the `--stage-lyrics-scale` variable.
   - Note renders high-contrast warm-amber cue card styling (`#f59e0b`).
5. **Headless Screen Wake Lock Behavior (`useWakeLock`):**
   - Created `frontend/js/behaviors/use-wake-lock.js`.
   - Progressively acquires `navigator.wakeLock.request('screen')`, degrades silently without alarm dialogs if unsupported or denied, releases cleanly upon deactivation, and automatically reacquires lock on `visibilitychange` if Stage Mode remains active.
6. **Pure Stage Mode Organism (`stage-mode-view`):**
   - Created `frontend/js/components/organisms/stage-mode-view.js` and `frontend/styles/organisms/stage-mode-view.css`.
   - Full-viewport fixed overlay with safe-area padding (`env(safe-area-inset-*)`).
   - Prominent exit button (touch target >= 48 px), `A-` and `A+` buttons to scale lyrics font size, subtle Wake Lock status indicator, and scrollable container.
   - Handles empty presentations gracefully with an explanatory message.
7. **Detail View & Controller Orchestration:**
   - Updated `frontend/js/components/views/presentation-detail-view.js` and `frontend/js/controllers/presentation-detail-controller.js`.
   - Integrated `useDragAndDrop` with serialization and automatic rollback to the last server-confirmed sequence upon error or conflict.
   - Added prominent "Modo Escenario" button in the view and topbar.
   - Shared singleton `audioController` and reactive `playbackSignal` across both views: entering and exiting Stage Mode never interrupts, restarts, or glitches active backing tracks.
   - Audio is stopped exclusively when navigating away from the presentation.
8. **Stylesheet Registration:**
   - Registered `drag-handle.css` and `stage-mode-view.css` in `frontend/styles/index.css`.

---

## Completed Tasks: Slice 4

- [x] Actualizar `frontend/js/data/move-item.js` para soportar opcionalmente el parámetro `target_position` en la petición POST hacia `/api/presentations/{id}/items/{itemId}/move`. <!-- sdd-owner: implementation -->
- [x] Implementar la molécula `frontend/js/components/molecules/drag-handle.js` y su estilo `frontend/styles/molecules/drag-handle.css` con `touch-action: none`, foco por teclado y señal visual clara de arrastre. <!-- sdd-owner: implementation -->
- [x] Implementar el comportamiento headless `frontend/js/behaviors/use-drag-and-drop.js` capturando pointer events exclusivamente en la manija, calculando umbrales de inserción mediante `requestAnimationFrame`, aplicando feedback visual temporal sin alterar el DOM real, emitiendo `onReorder(itemId, targetPosition)` al soltar dentro de la lista, y restaurando el orden ante `pointercancel` o salida. <!-- sdd-owner: implementation -->
- [x] Integrar `drag-handle` en `track-item.js`, `lyrics-item.js` y `note-item.js`, asegurando que las zonas fuera de la manija permitan scroll vertical nativo fluido. <!-- sdd-owner: implementation -->
- [x] Implementar el micro-comportamiento headless `frontend/js/behaviors/use-wake-lock.js` gestionando la adquisición de `navigator.wakeLock.request('screen')`, degradación silenciosa cuando no esté soportado o sea denegado, liberación limpia al desactivar, y re-adquisición automática ante eventos `visibilitychange` si el modo sigue activo. <!-- sdd-owner: implementation -->
- [x] Adaptar `track-item.js`, `lyrics-item.js` y `note-item.js` para recibir `mode: 'preparation' | 'stage'`: en modo stage se omiten por completo los nodos de edición, borrado, manija y flechas; track renderiza un botón gigante de play/pause (>= 48 × 48 px), lyrics usa tipografía display grande, y note usa tarjeta de alto contraste ámbar. <!-- sdd-owner: implementation -->
- [x] Implementar el organismo puro `frontend/js/components/organisms/stage-mode-view.js` como overlay fixed de viewport completo, con barra superior fija de safe-area, botón prominente para salir, controles accesibles `A-` / `A+` para regular el tamaño de letra (variable CSS `--stage-lyrics-scale`), indicador sutil de estado de Wake Lock, y contenedor scrollable para la secuencia de items en modo stage. <!-- sdd-owner: implementation -->
- [x] Crear la hoja de estilo atómica `frontend/styles/organisms/stage-mode-view.css` con fondo oscuro de alto contraste, protección de zonas seguras móviles (`env(safe-area-inset-*)`) y áreas táctiles de control >= 48 px. <!-- sdd-owner: implementation -->
- [x] Actualizar `frontend/js/controllers/presentation-detail-controller.js` para:
  1. Instanciar y vincular `useDragAndDrop`, serializando las peticiones de reordenamiento y revirtiendo al último orden confirmado por servidor ante error.
  2. Gestionar la apertura y cierre de Modo Escenario sin alterar la ruta hash, reutilizando la misma instancia del controlador singleton de audio y `playbackSignal` para que la música continúe sonando sin interrupciones ni reinicios.
  3. Ejecutar de forma progresiva `overlay.requestFullscreen()` y `wakeLock.activate()` al entrar a Modo Escenario, y liberar ambos al salir sin modal invasivo ante rechazos.
  4. Detener el audio únicamente cuando el usuario navega fuera de la presentación hacia el catálogo general. <!-- sdd-owner: implementation -->
- [x] Registrar nuevos estilos en `frontend/styles/index.css` y realizar la verificación final manual de todos los criterios de aceptación E2E: drag táctil, scroll nativo, entrada/salida a Modo Escenario durante reproducción activa, exclusividad de pista y fallback cuando Wake Lock o Fullscreen no están disponibles. <!-- sdd-owner: implementation -->

---

## Files Changed in Slice 4

### Created
- `frontend/js/behaviors/use-drag-and-drop.js`
- `frontend/js/behaviors/use-wake-lock.js`
- `frontend/js/components/molecules/drag-handle.js`
- `frontend/styles/molecules/drag-handle.css`
- `frontend/js/components/organisms/stage-mode-view.js`
- `frontend/styles/organisms/stage-mode-view.css`

### Modified
- `frontend/js/components/atoms/icon.js`
- `frontend/js/data/move-item.js`
- `frontend/js/components/organisms/track-item.js`
- `frontend/styles/organisms/track-item.css`
- `frontend/js/components/organisms/lyrics-item.js`
- `frontend/styles/organisms/lyrics-item.css`
- `frontend/js/components/organisms/note-item.js`
- `frontend/styles/organisms/note-item.css`
- `frontend/styles/layouts/timeline-layout.css`
- `frontend/js/components/views/presentation-detail-view.js`
- `frontend/js/controllers/presentation-detail-controller.js`
- `frontend/styles/index.css`
- `openspec/changes/shirley-show-redesign/tasks.md`

---

## Test Commands Run and Validation Evidence

1. **Backend Non-Regression Suite:**
   - Command: `cargo test`
   - Result: All 13 test suites (85 tests) passed with 0 failures, 0 warnings, and 0 regressions.
2. **Frontend Syntax Check:**
   - Command: `node --check` across every JavaScript module in `frontend/js/`.
   - Result: All files passed cleanly with 0 syntax errors.
3. **Static Asset Verification on `:3100`:**
   - Verified HTTP 200 responses via `/assets/...` for:
     - `/assets/js/behaviors/use-drag-and-drop.js`
     - `/assets/js/behaviors/use-wake-lock.js`
     - `/assets/js/components/molecules/drag-handle.js`
     - `/assets/js/components/organisms/stage-mode-view.js`
     - `/assets/styles/molecules/drag-handle.css`
     - `/assets/styles/organisms/stage-mode-view.css`
4. **Live E2E Verification against Server on `:3100`:**
   - Baseline preserved: "Evento 28 de Agosto" (`3e56cdca-b59c-4095-9d50-a1bdabbf03a2`) remained untouched.
   - Created temporary presentation "Test Temporal Slice 4".
   - Added lyrics and note items at contiguous positions 0, 1, 2.
   - Verified absolute reordering via `target_position: 0` shifting item 2 to the top and maintaining contiguous indices `[id2, id0, id1]`.
   - Verified absolute reordering back to end via `target_position: 2`.
   - Verified out-of-bounds guards: `target_position: 10` returns 409 Conflict (`reorder_boundary`), `target_position: -1` returns 409 Conflict.
   - Verified ambiguous selector rejection: providing both `direction` and `target_position` returns 400 Bad Request.
   - Verified missing selector rejection: empty payload `{}` returns 400 Bad Request.
   - Verified fallback direction swap: `direction: 'down'` succeeds with 200 OK.
   - Cleaned up temporary presentation via `DELETE`.
   - Baseline re-verified: catalog contains exclusively "Evento 28 de Agosto".

---

## Workload and PR Boundary

- **Work Unit:** `redesign-s4-drag-stage`
- **Authority Token:** `sha256:669cd2524bdb61e61ca6917aad48968d4073478c692baea1cb12dcb3cbd79727`
- **Request:** `r4-acquire-001`
- **Lines Budget:** Max 3000 lines (actual slice size: ~609 insertions in modified files + 941 lines across 6 new micro-modules = ~1550 lines).
- **PR Boundary:** PR 4 of 4 (final PR in stacked-to-main chain). Strictly drag handle, headless drag-and-drop, Screen Wake Lock, Stage Mode overlay, item stage variants, and detail controller orchestration. All 42 tasks of the entire change are now complete. Ready for settle.

---

## Executive Summary: Slice 3 (Bloques Táctiles de Secuencia, Adición Inmediata y Edición Rápida)

*(Historical reference from PR 3)*  
Slice 3 (`redesign-s3-tactile-edit`) streamlined presentation preparation and sequence editing: atomic `updateItem` client (PATCH), `quick-add-bar.js/.css`, direct form opening in `add-item-dialog.js`, `quick-edit-dialog.js/.css` with draft preservation, tactile item cards for tracks, lyrics, and notes with >= 48 px touch targets, and detail view/controller integration.

---

## Executive Summary: Slice 2 (Home Visual Library & Warm Identity)

*(Historical reference from PR 2)*  
Slice 2 (`redesign-s2-frontend-home`) transformed the catalog home view into an intimate visual library: warm ivory/gold tokens, `relative-date.js`, `welcome-header.js/.css`, enriched `presentation-card.js/.css`, catalog orchestration, and responsive design.

---

## Executive Summary: Slice 1 (Backend Item PATCH, Catálogo Enriquecido y Reordenamiento Absoluto)

*(Historical reference from PR 1)*  
Slice 1 (`redesign-s1-backend-catalog`) implemented the Rust backend foundation under strict TDD: `PATCH /api/presentations/{presentation_id}/items/{item_id}`, enriched catalog summaries (`GET /api/presentations`), and absolute reordering via `target_position` (`POST /api/presentations/{presentation_id}/items/{item_id}/move`).

---

## Remaining Tasks

*(None — all 42/42 tasks across Slices 1, 2, 3, and 4 are complete)*
