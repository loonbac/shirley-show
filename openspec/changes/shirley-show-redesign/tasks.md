# Tasks: Shirley Show Redesign

**Change:** `shirley-show-redesign`  
**Estado:** Ready for implementation  
**Estrategia de entrega:** ask-on-risk  
**Presupuesto por unidad de revisión:** 400 líneas  

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 1250–1480 lines (~300–370 lines per slice) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Slice 1: Backend PATCH + Catálogo) → PR 2 (Slice 2: Home visual) → PR 3 (Slice 3: Secuencia táctil + quick-add/edit) → PR 4 (Slice 4: Drag + Stage Mode) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

```text
Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High
```

---

## Slice 1: Backend Item PATCH, Catálogo Enriquecido y Reordenamiento Absoluto

**Objetivo del slice:** Proporcionar la infraestructura de datos para la edición de items (`PATCH`), resúmenes completos de presentaciones en un solo viaje de red (`GET /api/presentations`) y reordenamiento por posición absoluta en una transacción inmediata atómica.  
**Presupuesto estimado:** ~290–350 líneas.  
**Rollback:** Revertir `router.rs`, `use_cases`, `presentation_summary_repository.rs` y extensiones de handlers. Sin migraciones de esquema pendientes ni corrupción de audio/datos existentes.

### 1.1 TDD Modelos de Dominio y Puertos

- [x] RED: Escribir tests unitarios para `ShowItemUpdate` (variantes `TrackTitle` y `TextBody`, trim y validación de campos) y `PresentationSummary` / `ShowItemPreview` en `tests/domain_item_tests.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `src/domain/show_item_update.rs` y `src/domain/presentation_summary.rs`, exportándolos en `src/domain/mod.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Definir trait `PresentationSummaryReader` en `src/ports/presentation_summary_reader.rs` y extender el trait `ShowItemWriter` en `src/ports/show_item_writer.rs` con los métodos `update` y `move_to`, exportándolos en `src/ports/mod.rs`. <!-- sdd-owner: implementation -->

### 1.2 TDD Repositorios SQLite y Casos de Uso

- [x] RED: Escribir tests de integración en `tests/show_item_repository_tests.rs` para:
  1. `update`: actualizar título de pista (audio intacto) y body de letra/nota; actualización de `presentations.updated_at`; rechazo por item o presentación inexistente.
  2. `move_to`: mover hacia arriba, hacia abajo, al inicio, al final; no-op en misma posición; rechazo con error de límites fuera de rango (`< 0` o `>= N`); verificación de contigüidad `0..N-1` con `UNIQUE (presentation_id, position)`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar métodos `update` y `move_to` en `src/infra/sqlite/show_item_repository.rs` ejecutando bajo `TransactionBehavior::Immediate`, usando offset temporal para desplazamiento de rangos sin violar la restricción única, y actualizando el timestamp del padre. <!-- sdd-owner: implementation -->
- [x] RED: Escribir tests de repositorio en `tests/presentation_repository_tests.rs` para `PresentationSummaryRepository::list_summaries`: verificación de `tracks_count`, `lyrics_count`, `notes_count`, `total_items`, y hasta 3 `preview_items` ordenados por posición `0..2` con snippets normalizados y manejo de presentación con 0 items. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar `src/infra/sqlite/presentation_summary_repository.rs` con una única consulta SQL acotada con CTEs de agregación y `ROW_NUMBER() OVER (PARTITION BY presentation_id ORDER BY position)` con `LEFT JOIN`, exportándolo en `src/infra/sqlite/mod.rs`. <!-- sdd-owner: implementation -->
- [x] GREEN: Implementar el caso de uso `src/use_cases/update_show_item.rs` (validación contra el kind persistido, trim, y delegación atómica) y actualizar `src/use_cases/list_presentations.rs` y `src/use_cases/reorder_item.rs` (soporte de `target_position`), exportándolos en `src/use_cases/mod.rs`. <!-- sdd-owner: implementation -->

### 1.3 Handlers HTTP, Routing y Verificación API

- [x] RED: Escribir tests de endpoint en `tests/item_api_tests.rs` y `tests/presentation_api_tests.rs` cubriendo:
  1. `PATCH /api/presentations/{id}/items/{itemId}`: éxito 200 con `ShowItemResponse`, 400 por payload vacío, 400 por campo incompatible con tipo, 404 por presentación o item ausente.
  2. `GET /api/presentations`: 200 con estructura enriquecida `PresentationSummaryResponse` preservando campos existentes, 0 items devuelve ceros y `preview_items: []`.
  3. `POST /api/presentations/{id}/items/{itemId}/move`: target_position válido 200 con secuencia confirmada, target fuera de límites 409, ambigüedad (ambos o ningún selector) 400, retrocompatibilidad con `direction`. <!-- sdd-owner: implementation -->
- [x] GREEN: Actualizar DTOs en `src/handlers/dto.rs`, crear handler `src/handlers/items/update.rs`, actualizar `src/handlers/items/reorder.rs` y `src/handlers/presentations/list.rs`, registrar rutas en `src/router.rs` y componer dependencias en `src/state/app_state.rs`. <!-- sdd-owner: implementation -->
- [x] TRIANGULATE & REFACTOR: Ejecutar la suite completa con `cargo test`, verificar ausencia de fugas de memoria o transacciones colgadas, y confirmar que las pruebas de regresión existentes (upload multipart, streaming, borrado en cascada) permanecen verdes. <!-- sdd-owner: implementation -->

---

## Slice 2: Home Visual Library & Warm Identity

**Objetivo del slice:** Transformar la vista de inicio en una biblioteca cálida, íntima y elegante para Shirley, con un encabezado de bienvenida personal, métricas agregadas y tarjetas ricas que exhiben el repertorio y facilitan la apertura inmediata del show.  
**Presupuesto estimado:** ~300–360 líneas.  
**Rollback:** Restaurar `presentations-view.js`, `presentation-card.js` y estilos previos; la UI ignora los campos enriquecidos de la API.

### 2.1 Tokens de Diseño y Utilidad de Fecha

- [x] Ajustar tokens en `frontend/styles/tokens/colors.css`, `elevations.css` y `spacing.css` incorporando la paleta marfil cálido (`#FAF8F5`), oro envejecido (`#B8860B`), acentos temáticos (violeta `#6D28D9`, cian profundo `#0E7490`, ámbar cálido `#B45309`) y elevaciones suaves de papel. <!-- sdd-owner: implementation -->
- [x] Implementar la micro-utilidad pura `frontend/js/utils/format/relative-date.js` para formatear timestamps ISO en texto humano en español ("Actualizado hoy", "Hace 2 días", neutral fallback para fechas inválidas). <!-- sdd-owner: implementation -->

### 2.2 Componente Welcome Header

- [x] Implementar el organismo puro `frontend/js/components/organisms/welcome-header.js` que recibe saludo personalizado, métricas agregadas de la biblioteca (repertorios y pistas) y callback `onCreatePresentation`. <!-- sdd-owner: implementation -->
- [x] Crear la hoja de estilo atómica 1:1 `frontend/styles/organisms/welcome-header.css` con tipografía cálida, distribución responsive y botón de acción primaria con área táctil mínima de 48 px. <!-- sdd-owner: implementation -->

### 2.3 Rediseño de Presentation Card

- [x] Rediseñar el organismo `frontend/js/components/organisms/presentation-card.js` para recibir el summary enriquecido (`tracks_count`, `lyrics_count`, `notes_count`, `total_items`, `preview_items`), renderizar fecha humana, badges cromáticos por tipo, caja de preview con los primeros 3 temas/snippets, botón dominante "Abrir show" (touch target >= 48 px) y menú discreto para renombrar/eliminar. <!-- sdd-owner: implementation -->
- [x] Rediseñar la hoja de estilo `frontend/styles/organisms/presentation-card.css` con elevación física, estados interactivos claros (hover/active/focus-visible) y separación visual entre acción primaria y destructivas. <!-- sdd-owner: implementation -->

### 2.4 Integración en Vista y Controlador de Catálogo

- [x] Actualizar `frontend/js/components/views/presentations-view.js` y `frontend/styles/views/presentations-view.css` para orquestar `welcome-header` cuando hay repertorios, mantener el `empty-state` canónico cuando la lista está vacía, y montar el grid de tarjetas ricas. <!-- sdd-owner: implementation -->
- [x] Actualizar `frontend/js/controllers/presentations-controller.js` para consumir los datos enriquecidos de `listPresentations()`, calcular totales agregados para el header y enlazar el callback de "Abrir show" con la navegación directa. <!-- sdd-owner: implementation -->
- [x] Registrar `welcome-header.css` en `frontend/styles/index.css` y verificar visualmente en navegador con 0 presentaciones, con 1 presentación sin items y con múltiples presentaciones con items variados. <!-- sdd-owner: implementation -->

---

## Slice 3: Bloques Táctiles de Secuencia, Adición Inmediata y Edición Rápida

**Objetivo del slice:** Optimizar la preparación del show eliminando la fricción de modales con pestañas mediante una barra de adición directa (`+ Pista`, `+ Letra`, `+ Nota`), bloques táctiles diferenciados por tipo, y un diálogo enfocado de edición rápida conectado a `PATCH`.  
**Presupuesto estimado:** ~340–390 líneas.  
**Rollback:** Revertir `quick-add-bar`, `quick-edit-dialog`, `update-item.js` y modificaciones en organismos de items.

### 3.1 Cliente de Datos y Barra de Adición Inmediata

- [ ] Crear el cliente de datos atómico `frontend/js/data/update-item.js` implementando la llamada `PATCH /api/presentations/{presentation_id}/items/{item_id}` con manejo tipado de errores vía `requestJson`. <!-- sdd-owner: implementation -->
- [ ] Implementar la molécula `frontend/js/components/molecules/quick-add-bar.js` con tres botones táctiles dedicados (`+ Pista`, `+ Letra`, `+ Nota`) con tamaño mínimo de 48 px, emitiendo `onAdd('track'|'lyrics'|'note')`. <!-- sdd-owner: implementation -->
- [ ] Crear la hoja de estilo atómica `frontend/styles/molecules/quick-add-bar.css` con diseño mobile-first y acentos cromáticos coincidentes con cada tipo de contenido. <!-- sdd-owner: implementation -->
- [ ] Modificar `frontend/js/components/organisms/add-item-dialog.js` para recibir el tipo inicial preseleccionado y abrir directamente el formulario correspondiente sin paso de navegación por tabs. <!-- sdd-owner: implementation -->

### 3.2 Diálogo de Edición Rápida

- [ ] Implementar el organismo puro `frontend/js/components/organisms/quick-edit-dialog.js` recibiendo el item actual, precargando el valor de título o body, autofocando el campo, con atajos de teclado (Enter para pistas, Cmd/Ctrl+Enter para textarea, Escape para cancelar), y conservando el borrador ante errores de red o validación. <!-- sdd-owner: implementation -->
- [ ] Crear la hoja de estilo atómica `frontend/styles/organisms/quick-edit-dialog.css` con overlay accesible, diseño responsivo y estado visual de error y carga sin destruir el formulario. <!-- sdd-owner: implementation -->

### 3.3 Bloques Táctiles de Escenario (Track, Lyrics, Note)

- [ ] Rediseñar `frontend/js/components/organisms/track-item.js` y `frontend/styles/organisms/track-item.css` con acento violeta, áreas táctiles mínimas de 48 px, botón de edición rápida (lápiz) con callback `onEdit`, manteniendo confirmación de borrado y controles de reordenamiento accesibles. <!-- sdd-owner: implementation -->
- [ ] Rediseñar `frontend/js/components/organisms/lyrics-item.js` y `frontend/styles/organisms/lyrics-item.css` con acento cian profundo, preservación de saltos de línea legibles, botón de edición rápida y áreas táctiles de 48 px. <!-- sdd-owner: implementation -->
- [ ] Rediseñar `frontend/js/components/organisms/note-item.js` y `frontend/styles/organisms/note-item.css` con acento ámbar cálido, tipografía clara de indicación de show, botón de edición rápida y áreas táctiles de 48 px. <!-- sdd-owner: implementation -->

### 3.4 Orquestación en Detalle de Presentación

- [ ] Actualizar `frontend/js/components/views/presentation-detail-view.js` y `frontend/styles/views/presentation-detail-view.css` para integrar la `quick-add-bar` fija o destacada y gestionar el montaje del `quick-edit-dialog`. <!-- sdd-owner: implementation -->
- [ ] Actualizar `frontend/js/controllers/presentation-detail-controller.js` para conectar `updateItem`, actualizar el estado local y la vista tras confirmación del servidor, preservar el borrador ante fallos, y manejar el caso 404 informando al usuario sin crear items duplicados. <!-- sdd-owner: implementation -->
- [ ] Registrar estilos nuevos en `frontend/styles/index.css` y verificar manualmente: adición directa de los 3 tipos, edición rápida con guardado y cancelación, y conservación de draft ante fallo de red simulado. <!-- sdd-owner: implementation -->

---

## Slice 4: Reordenamiento por Manija Táctil y Modo Escenario / Atril

**Objetivo del slice:** Habilitar el reordenamiento táctil fluido por arrastre mediante una manija dedicada sin interferir con el scroll nativo, y proporcionar un Modo Escenario a pantalla completa, sin controles de edición, con tipografía escalable, control de audio continuo y Screen Wake Lock.  
**Presupuesto estimado:** ~340–390 líneas.  
**Rollback:** Revertir `stage-mode-view`, `use-drag-and-drop.js`, `use-wake-lock.js`, `drag-handle.js` y cambios en controladores de detalle.

### 4.1 Manija de Arrastre y Micro-Comportamiento Headless Drag-and-Drop

- [ ] Actualizar `frontend/js/data/move-item.js` para soportar opcionalmente el parámetro `target_position` en la petición POST hacia `/api/presentations/{id}/items/{itemId}/move`. <!-- sdd-owner: implementation -->
- [ ] Implementar la molécula `frontend/js/components/molecules/drag-handle.js` y su estilo `frontend/styles/molecules/drag-handle.css` con `touch-action: none`, foco por teclado y señal visual clara de arrastre. <!-- sdd-owner: implementation -->
- [ ] Implementar el comportamiento headless `frontend/js/behaviors/use-drag-and-drop.js` capturando pointer events exclusivamente en la manija, calculando umbrales de inserción mediante `requestAnimationFrame`, aplicando feedback visual temporal sin alterar el DOM real, emitiendo `onReorder(itemId, targetPosition)` al soltar dentro de la lista, y restaurando el orden ante `pointercancel` o salida. <!-- sdd-owner: implementation -->
- [ ] Integrar `drag-handle` en `track-item.js`, `lyrics-item.js` y `note-item.js`, asegurando que las zonas fuera de la manija permitan scroll vertical nativo fluido. <!-- sdd-owner: implementation -->

### 4.2 Micro-Comportamiento Screen Wake Lock

- [ ] Implementar el micro-comportamiento headless `frontend/js/behaviors/use-wake-lock.js` gestionando la adquisición de `navigator.wakeLock.request('screen')`, degradación silenciosa cuando no esté soportado o sea denegado, liberación limpia al desactivar, y re-adquisición automática ante eventos `visibilitychange` si el modo sigue activo. <!-- sdd-owner: implementation -->

### 4.3 Soporte de Variante "Stage" en Organismos de Items

- [ ] Adaptar `track-item.js`, `lyrics-item.js` y `note-item.js` para recibir `mode: 'preparation' | 'stage'`: en modo stage se omiten por completo los nodos de edición, borrado, manija y flechas; track renderiza un botón gigante de play/pause (>= 48 × 48 px), lyrics usa tipografía display grande, y note usa tarjeta de alto contraste ámbar. <!-- sdd-owner: implementation -->

### 4.4 Organismo Stage Mode View y Control de Tipografía

- [ ] Implementar el organismo puro `frontend/js/components/organisms/stage-mode-view.js` como overlay fixed de viewport completo, con barra superior fija de safe-area, botón prominente para salir, controles accesibles `A-` / `A+` para regular el tamaño de letra (variable CSS `--stage-lyrics-scale`), indicador sutil de estado de Wake Lock, y contenedor scrollable para la secuencia de items en modo stage. <!-- sdd-owner: implementation -->
- [ ] Crear la hoja de estilo atómica `frontend/styles/organisms/stage-mode-view.css` con fondo oscuro de alto contraste, protección de zonas seguras móviles (`env(safe-area-inset-*)`) y áreas táctiles de control >= 48 px. <!-- sdd-owner: implementation -->

### 4.5 Orquestación de Audio Continuo y Modo Escenario en Controlador

- [ ] Actualizar `frontend/js/controllers/presentation-detail-controller.js` para:
  1. Instanciar y vincular `useDragAndDrop`, serializando las peticiones de reordenamiento y revirtiendo al último orden confirmado por servidor ante error.
  2. Gestionar la apertura y cierre de Modo Escenario sin alterar la ruta hash, reutilizando la misma instancia del controlador singleton de audio y `playbackSignal` para que la música continúe sonando sin interrupciones ni reinicios.
  3. Ejecutar de forma progresiva `overlay.requestFullscreen()` y `wakeLock.activate()` al entrar a Modo Escenario, y liberar ambos al salir sin modal invasivo ante rechazos.
  4. Detener el audio únicamente cuando el usuario navega fuera de la presentación hacia el catálogo general. <!-- sdd-owner: implementation -->
- [ ] Registrar nuevos estilos en `frontend/styles/index.css` y realizar la verificación final manual de todos los criterios de aceptación E2E: drag táctil, scroll nativo, entrada/salida a Modo Escenario durante reproducción activa, exclusividad de pista y fallback cuando Wake Lock o Fullscreen no están disponibles. <!-- sdd-owner: implementation -->
