# Proposal: Shirley Show Core

**Change:** `shirley-show-core`  
**Slice:** 1 — núcleo usable  
**Status:** Proposed  
**Product metaphor:** “Un grupo de WhatsApp por presentación, pero diseñado para cantar”.

## Intent

Crear una herramienta mínima, rápida y legible para preparar y ejecutar una presentación musical desde teléfono o PC. Cada presentación funciona como un espacio independiente que reúne, en el orden real del show, pistas de audio, letras y notas escénicas.

El primer slice debe sustituir la improvisación de usar un grupo de WhatsApp como setlist sin incorporar cuentas, colaboración en tiempo real ni automatizaciones avanzadas.

## Problem

Cantantes y músicos suelen reunir pistas, letras e indicaciones en chats porque el formato es familiar, vertical y accesible desde el teléfono. Sin embargo, un chat no está diseñado para ejecutar un show:

- el orden útil se pierde entre mensajes;
- pistas, letras y notas no tienen controles ni presentación específicos;
- reordenar el repertorio exige reenviar o recrear contenido;
- encontrar y reproducir una pista durante una presentación añade fricción y riesgo;
- el contenido escénico queda mezclado con conversaciones y metadatos irrelevantes.

Esta fricción aumenta la carga cognitiva antes y durante una presentación, justo cuando las acciones deben ser inmediatas y predecibles.

## Target user and situation

La usuaria principal es una cantante o música que prepara y ejecuta presentaciones, ensayos o sets acústicos. Trabaja principalmente desde un teléfono colocado en un atril, aunque puede organizar el contenido desde una laptop o PC.

Durante el show necesita:

- reconocer de inmediato qué sigue;
- leer letras a distancia razonable;
- reproducir o pausar una pista con controles táctiles grandes;
- consultar indicaciones breves sin navegar por pantallas secundarias;
- mantener el orden del set sin depender de gestos delicados.

## Current-state gap

El grupo de WhatsApp aporta una lista cronológica familiar, pero carece de estructura de presentación. Shirley Show conserva la sencillez de una sola secuencia vertical y reemplaza cada mensaje genérico por un elemento escénico especializado.

| Necesidad | Grupo de WhatsApp | Resultado esperado en Shirley Show |
| --- | --- | --- |
| Separar cada show | Chats creados o reutilizados de forma ad hoc | Una presentación identificable con nombre libre |
| Mantener el setlist | Orden cronológico difícil de corregir | Orden explícito y persistente |
| Reproducir pistas | Adjuntos con controles no orientados al escenario | Pista con nombre y play/pausa prominentes |
| Leer letras | Mensajes entremezclados y tipografía de chat | Bloques de texto grandes dentro del orden del show |
| Recordar cues | Mensajes que compiten con el resto | Notas breves visualmente distinguibles |

## Product outcome

Al terminar este slice, la usuaria puede crear una presentación, construir su secuencia completa y recorrerla durante el show desde una única vista vertical. La experiencia debe sentirse tan directa como abrir un chat, pero cada contenido debe estar optimizado para cantar.

El producto se limita a dos contextos principales:

1. **Lista de presentaciones:** crear, abrir, renombrar y eliminar presentaciones.
2. **Detalle de presentación:** consultar y operar una sola lista ordenada de pistas, letras y notas.

## Business rules

### Presentations

1. Una presentación es la unidad principal de organización y tiene un identificador estable y un nombre libre, por ejemplo, “Viernes 24 Agosto”.
2. Crear una presentación requiere un nombre no vacío después de quitar espacios exteriores.
3. Renombrar cambia el nombre visible sin alterar la identidad, los elementos ni su orden.
4. Eliminar una presentación elimina también sus elementos y los archivos de audio asociados; la interfaz debe pedir confirmación explícita porque la acción es destructiva.
5. Una lista sin presentaciones debe explicar el estado vacío y ofrecer una acción visible para crear la primera.

### Ordered show items

1. Cada presentación contiene exactamente una secuencia vertical ordenada; no existen secciones, conversaciones, canciones contenedoras ni listas paralelas en este slice.
2. Cada elemento es independiente y tiene un solo tipo: `track`, `lyrics` o `note`.
3. Al agregar un elemento, la usuaria elige primero el tipo mediante una acción visible y el nuevo elemento se añade a la secuencia.
4. Las posiciones son contiguas y pertenecen a una sola presentación.
5. “Subir” intercambia atómicamente el elemento con su vecino anterior; “Bajar” hace lo mismo con el siguiente.
6. “Subir” no está disponible para el primer elemento y “Bajar” no está disponible para el último.
7. Eliminar un elemento requiere confirmación y cierra cualquier reproducción asociada antes de retirar sus datos o archivo.
8. La lista vacía de una presentación debe ofrecer de forma visible la acción para agregar pista, letra o nota.

### Track

1. Una pista requiere un archivo de audio aceptado y un nombre visible.
2. El slice admite archivos `.mp3`, `.wav` y `.m4a`, con un máximo de 100 MiB por subida.
3. Solo una pista puede reproducirse a la vez en toda la aplicación.
4. Iniciar otra pista pausa la activa; salir de la presentación o eliminar la pista activa también la pausa.
5. Los controles principales son play/pausa, progreso, tiempo actual y duración; la reproducción nunca comienza automáticamente.
6. La usuaria debe poder buscar una posición del audio en navegadores móviles compatibles mediante streaming HTTP con Range.
7. Una subida fallida no debe crear un elemento incompleto ni dejar un archivo huérfano visible como pista válida.

### Lyrics and notes

1. Una letra admite texto libre multilínea y se muestra con tipografía grande y legible en escenario.
2. Una nota admite texto breve y se distingue visualmente de una letra y de una pista.
3. Letra y nota requieren contenido no vacío después de quitar espacios exteriores.
4. La edición posterior de letras, notas o metadatos de pistas no forma parte de este slice; pueden eliminarse y volver a crearse.

### Access model

1. No hay login, registro, perfiles, roles ni permisos internos.
2. Toda persona con acceso al servidor puede ver y modificar el contenido; el uso se asume local o dentro de una red cerrada.
3. Esta limitación debe quedar clara en la documentación operativa para evitar exponer el servicio directamente como aplicación pública.

## Scope

### In scope for Slice 1

- Lista persistente de presentaciones.
- Crear, abrir, renombrar y eliminar una presentación.
- Vista de detalle con una única lista vertical ordenada.
- Agregar y eliminar elementos independientes de tipo pista, letra o nota.
- Reordenar mediante botones explícitos arriba/abajo.
- Subir audio, asignarle nombre y reproducirlo o pausarlo con controles grandes.
- Mostrar progreso, tiempo actual y duración de la pista.
- Pausar una pista al iniciar otra o abandonar la presentación.
- Estados de carga, vacío, error y confirmación necesarios para las operaciones anteriores.
- Experiencia mobile-first, responsive en PC, oscura por defecto y con objetivos táctiles mínimos de 44 × 44 px.
- Persistencia SQLite y almacenamiento local de audios en disco.
- Backend Rust + Axum que expone la API y sirve el frontend vanilla JS.

### First-slice boundaries

- La navegación se mantiene en el mínimo de pantallas: lista y detalle, usando modal o drawer solo para acciones acotadas.
- El orden se modifica exclusivamente con botones; no se implementa arrastre.
- La interacción es cliente-servidor HTTP estándar y no sincroniza cambios en vivo entre dispositivos.
- El almacenamiento y la ejecución pertenecen a una sola instancia del servidor.
- El slice ofrece creación y eliminación de elementos, pero no edición posterior del contenido.
- El alcance es el núcleo usable completo; mejoras de organización, colaboración y automatización se evalúan después de validar este flujo.

## Non-goals

- Drag-and-drop táctil o reordenamiento por gestos.
- IA, transcripción, detección de acordes, karaoke sincronizado o recomendaciones.
- Login, registro, perfiles, roles, permisos o recuperación de cuenta.
- Colaboración o sincronización en tiempo real mediante WebSockets.
- Comentarios, conversación, reacciones o funciones sociales de WhatsApp.
- PWA, modo offline avanzado o Service Worker.
- Almacenamiento cloud como S3 o R2.
- Despliegue, infraestructura pública, dominio, TLS o estrategia de hosting.
- Edición avanzada, formato enriquecido o importación masiva de repertorio.
- Mezcla, volumen independiente, crossfade, playlists automáticas o reproducción continua.

## Constraints

### Product and UX

- Mobile-first sin degradar el uso responsive en PC.
- Controles esenciales utilizables con una mano y sin precisión fina.
- Tipografía de letras apta para lectura durante una presentación.
- Modo oscuro predeterminado para reducir brillo en escenario.
- Mínima cantidad de pantallas y navegación predecible.
- Los fallos deben mostrarse en contexto y nunca simular que una operación persistió cuando no lo hizo.

### Architecture

- Backend en Rust + Axum.
- SQLite configurado con WAL y `busy_timeout` para reducir fallos transitorios por contención.
- Reordenamiento como swap atómico dentro de una transacción SQLite.
- Audios almacenados fuera de SQLite, en disco local, con rutas internas generadas por el sistema y sin confiar en nombres de archivo aportados por el cliente.
- Endpoint de audio con `Accept-Ranges: bytes`, respuestas `206 Partial Content` válidas y manejo de rangos inválidos para permitir seek móvil.
- Límite multipart configurado explícitamente en 100 MiB para la ruta de subida, sin ampliar indiscriminadamente el límite de todas las rutas.
- Frontend vanilla JavaScript servido por el backend mediante módulos ES nativos.
- Micro-modularidad conforme a `AGENTS.md`: un caso de uso por acción, ports mínimos, componentes UI sin acceso directo a red, comportamientos headless y CSS 1:1 por componente.
- TDD estricto cuando exista runner; la configuración actual identifica `cargo test` para backend.

### Data integrity and operations

- Las eliminaciones de presentación y pista deben coordinar base de datos y disco para no exponer registros rotos ni acumular archivos de forma silenciosa.
- Los errores por archivo ausente, formato inválido, límite excedido, disco lleno o base ocupada deben producir respuestas recuperables y mensajes accionables.
- No se deben interpolar rutas ni nombres del cliente para acceder al sistema de archivos.
- La aplicación debe conservar el identificador de presentación en la URL para que una recarga pueda reconstruir la vista; si ya no existe, vuelve de forma segura a la lista.

## Affected areas and implications

| Area | Change and impact |
| --- | --- |
| Domain model | Introduce presentations, typed items and per-presentation ordering invariants. |
| Persistence | Add SQLite schema, migrations, repositories, WAL configuration and transactional reordering. |
| Filesystem | Add managed audio directory, safe generated paths and lifecycle cleanup. |
| HTTP API | Add presentation/item CRUD, multipart upload, audio streaming and static asset serving. |
| Frontend state | Track active presentation and the sole active audio playback without a monolithic store. |
| UX/UI | Add list and show views, typed item components, large audio controls, confirmations and empty/error states. |
| Operations | Require writable persistent paths, sufficient disk capacity and backup of both database and audio directory. |
| Security posture | With no app-level auth, network exposure becomes an operator responsibility and public deployment is unsafe by default. |
| Support burden | Upload failures, unsupported mobile codecs and missing local files need recognizable messages rather than generic failures. |
| Delivery | The complete implementation is expected to exceed the 400 changed-line review budget; delivery slicing must be resolved under `ask-on-risk` before implementation crosses that threshold. |

## Edge cases

- No presentations exist, or a presentation exists with no elements.
- A name or text contains only whitespace, is very long, contains emoji or uses multiple writing systems.
- Two presentations have the same visible name; stable identifiers, not titles, must drive routing and persistence.
- The opened presentation or item was deleted in another tab; refresh or the next action returns safely to a recoverable state.
- The user presses reorder repeatedly or concurrent requests target neighboring items; the stored order remains contiguous and deterministic.
- Reordering the first item upward or the last item downward is a no-op rejected by the domain rule, not a corrupting update.
- An upload has an unsupported extension/type, exceeds 100 MiB, is interrupted, or fails because disk space is exhausted.
- A database write succeeds but filesystem cleanup fails, or a file write succeeds before database persistence fails; compensation and cleanup prevent a valid-looking broken item.
- An audio record exists but its file is missing or unreadable; the UI reports the unavailable track and remains usable.
- A browser does not support the encoded audio despite an allowed container extension; playback failure is explicit and does not block the rest of the show.
- A Range header is absent, valid, malformed or outside the file bounds; full and partial responses follow HTTP semantics without reading the whole file unnecessarily.
- Duration metadata is initially unavailable or slow; play/pausa remains understandable and timing uses a loading/unknown state.
- Navigation occurs while audio is playing; playback pauses and listeners/resources are released.
- Delete is cancelled, fails halfway or targets an already-deleted resource; the interface reconciles with persisted state.
- SQLite is briefly busy; `busy_timeout` allows recovery, and exhausted retries return a visible error without duplicating operations.
- The page reloads on a stale show URL; the user lands safely on the presentation list with an explanation.

## Tradeoffs

1. **Buttons over drag:** Up/down controls are slower for large setlists but more reliable, accessible and reviewable on touch devices. Drag remains a later enhancement.
2. **Single vertical sequence over richer grouping:** The model cannot group a track with its lyrics as one song, but preserves the chat-like simplicity and allows every item to occupy its exact stage order.
3. **Local storage over cloud durability:** Disk storage minimizes infrastructure and supports a usable local product, but makes backup, capacity and host failure operational concerns.
4. **No authentication over safe public access:** Removing account flows accelerates the first usable slice, but confines responsible use to trusted networks or external access controls.
5. **Vanilla JS over a frontend framework:** Native modules reduce tooling and runtime weight, while requiring explicit lifecycle and state discipline.
6. **100 MiB upload ceiling over arbitrary files:** The limit covers typical backing tracks and bounds resource consumption, but large lossless recordings may require conversion.
7. **Creation/deletion over in-place item editing:** Omitting editing keeps the first interaction model small, but correcting lyrics or notes requires recreation.
8. **Atomic consistency over maximum write concurrency:** Transactional swaps and coordinated deletion favor predictable show order over high concurrent edit throughput, which is not a primary slice-one need.

## Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Mobile seeking fails | A performer cannot jump to the required point in a track | Correct Range handling, focused HTTP tests and verification on representative mobile browsers |
| Upload limit remains at Axum default | Normal tracks fail unexpectedly | Apply a route-scoped 100 MiB `DefaultBodyLimit` and test below/above boundary behavior |
| Position races or duplicate indices | Setlist order becomes unreliable | Contiguous positions, unique ordering invariant where practical, and swap in one immediate transaction |
| SQLite lock contention | Actions fail intermittently | WAL, 5-second busy timeout, bounded transactions and clear errors |
| Database/filesystem divergence | Missing or orphaned audio | Generated IDs/paths, staged writes, compensating cleanup and reconciliation logging |
| Accidental destructive action | A complete show or track is lost | Explicit confirmation and clear destructive copy; backup guidance for operators |
| Public exposure without auth | Unauthorized reading or modification | Bind/deploy only in trusted environments or behind external access control; public deployment remains out of scope |
| Unsupported codec on a device | A valid upload cannot play during a show | Restrict accepted formats, surface playback errors and document browser/device verification |
| UI becomes monolithic during rapid delivery | Reuse and testability degrade | Enforce `AGENTS.md` dependency direction and atomic component/use-case boundaries |
| Slice exceeds review budget | Review quality declines | Pause under `ask-on-risk` and choose a delivery chain before implementation exceeds 400 changed lines |

## Rollback plan

This change introduces persistent data and local files, so rollback must preserve both as one logical dataset.

### Before enabling the slice

1. Keep database migrations additive and versioned; do not destructively alter an existing schema without a separately approved migration plan.
2. Configure the SQLite database and audio directory beneath one documented persistent data root.
3. Create a restorable backup of the SQLite database using a WAL-safe method and a matching snapshot of the audio directory before any migration or release replacement.
4. Record the application version and schema version associated with the backup.

### Application rollback

1. Stop writes by stopping the Axum process or placing the instance in maintenance mode.
2. Roll back the application/static assets to the last known-good release.
3. If the previous release supports the current additive schema, retain the current data and restart it.
4. If schema or data compatibility is not guaranteed, restore the matched SQLite and audio snapshots together before restarting.
5. Verify that presentations load, item order is contiguous, stored audio paths resolve and a representative track supports playback and seeking.

### Failure during a destructive or file operation

1. A failed upload removes its staged file and does not publish an item row.
2. A failed database operation leaves the prior order or record unchanged through transaction rollback.
3. If post-commit file deletion fails, keep the user-facing database state consistent, record the orphan for retry/reconciliation and do not silently reuse it.
4. If integrity cannot be established, stop further destructive writes, preserve logs and current files, and restore the latest matched backup.

Rollback does not promise recovery of user deletions made after the latest backup; confirmation and documented backups are the slice-one safeguards.

## Success criteria

The slice is successful when all of the following are demonstrably true:

1. From an empty installation, a user can create, open, rename and delete a presentation without entering account information.
2. A user can add at least one track, one lyrics block and one note to the same presentation and see them in a single persistent vertical order after reload.
3. Up/down controls correctly reorder adjacent items, protect first/last boundaries and preserve contiguous order under repeated requests.
4. A supported audio file up to 100 MiB can be uploaded, named, played, paused and seeked from a representative mobile browser using valid partial-content responses.
5. Starting a second track or leaving the presentation pauses the currently playing track, so no two tracks play simultaneously.
6. Lyrics remain readable and primary controls provide at least 44 × 44 px touch targets on target mobile layouts; the same flows remain usable on PC widths.
7. Empty, loading, upload failure, missing audio, persistence failure and stale-route states are explicit and recoverable without page corruption.
8. Deleting a presentation removes its items from the product and coordinates cleanup of associated audio without leaving valid records that point to missing files.
9. SQLite starts with WAL and the configured busy timeout, and reordering is covered by transaction-focused tests.
10. Domain and HTTP behavior are developed under strict TDD with `cargo test`, while frontend modules preserve the dependency boundaries defined by `AGENTS.md`.
11. The implementation does not include authentication, AI, drag-and-drop, cloud storage, real-time collaboration or deployment work.

## Validation indicators after first use

Because this is the initial usable slice, success is primarily task-based rather than growth-based. Early validation should observe:

- whether a performer can prepare a complete small show without returning to WhatsApp for ordering;
- whether opening the next item and controlling audio is immediate during a rehearsal;
- whether button-based reordering is sufficient before investing in touch drag;
- whether local storage and the 100 MiB limit cover the actual backing-track workflow;
- which omitted edit operations create enough rework to justify the next slice.
