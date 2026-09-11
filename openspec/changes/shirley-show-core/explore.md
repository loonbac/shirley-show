# SDD Exploration: `shirley-show-core`

**Change**: `shirley-show-core`  
**Date**: 2026-09-11  
**Status**: Completed  
**Author**: Gentle AI (SDD Explore Phase)  

---

## 1. Visión y Metáfora del Proyecto

**shirley-show** nace bajo la premisa de ser *"un grupo de WhatsApp por presentación, pero diseñado para cantar"*.

En ensayos, presentaciones en vivo o conciertos acústicos, un cantante o músico necesita una herramienta ágil, sin distracciones y enfocada en la ejecución en vivo:
- Cada **Presentación** (show o setlist) agrupa los elementos en una línea de tiempo vertical simple y legible.
- Cada **Elemento** corresponde a uno de tres tipos esenciales:
  1. **Pista (`track`):** Archivo de audio (backing track) con controles inmediatos de reproducción (play/pausa, progreso).
  2. **Letra (`lyrics`):** Texto formateado y acordes/cues vocales con tipografía grande y legible en escenario.
  3. **Nota (`note`):** Apunte rápido o directiva escénica ("presentar banda", "cambio de afinación", "esperar aplauso").
- La experiencia es **mobile-first** y full-screen friendly, utilizable tanto desde el atril con un smartphone como desde una laptop o tablet.

---

## 2. Mapeo Arquitectónico a AGENTS.md (Micro-Modularidad Extrema)

Siguiendo el manifiesto de arquitectura (`AGENTS.md`), cada pieza del sistema debe ser atómica, componible e independiente. A continuación se define la estructura concreta para el backend en Rust y el frontend en Vanilla JS:

### 2.1 Backend: Rust + Axum (Casos de Uso 1-Acción-1-Archivo)

El backend expone una API REST ligera y sirve los activos estáticos de la interfaz. Se organiza bajo Clean Architecture / Hexagonal con modularidad estricta:

```
backend/
├── Cargo.toml
└── src/
    ├── main.rs                           # Bootstrapping Axum, router assembly y graceful shutdown
    ├── state.rs                          # AppState (Arc con repositorios y config de almacenamiento)
    ├── core/
    │   ├── domain/                       # Entidades y value objects puros
    │   │   ├── presentation.rs           # Entidad Presentation (id, title, created_at, updated_at)
    │   │   ├── item.rs                   # Entidad Item (id, presentation_id, position, item_type, content, audio_path)
    │   │   └── item_type.rs              # Enum ItemType (Track, Lyrics, Note)
    │   │
    │   ├── ports/                        # Traits mínimos (DIP / ISP)
    │   │   ├── presentation_repository.rs # Trait PresentationRepository
    │   │   ├── item_repository.rs         # Trait ItemRepository
    │   │   └── audio_storage.rs           # Trait AudioStorage (save, get_path, delete)
    │   │
    │   └── use_cases/                    # 1 archivo por caso de uso / comando
    │       ├── create_presentation.rs
    │       ├── list_presentations.rs
    │       ├── get_presentation.rs
    │       ├── rename_presentation.rs
    │       ├── delete_presentation.rs
    │       ├── add_item.rs               # Agregar letra o nota
    │       ├── upload_track.rs           # Agregar pista (multipart audio + registro de item)
    │       ├── reorder_item.rs           # Mover elemento arriba / abajo
    │       └── delete_item.rs            # Eliminar elemento
    │
    ├── infra/                            # Adaptadores de infraestructura
    │   ├── sqlite/
    │   │   ├── connection.rs             # Inicialización y PRAGMAs (WAL, busy_timeout)
    │   │   ├── migrations.rs             # DDL automático (presentations, items)
    │   │   ├── sqlite_presentation_repo.rs
    │   │   └── sqlite_item_repo.rs
    │   └── filesystem/
    │       └── local_audio_storage.rs    # Almacenamiento seguro en disco (data/audio/)
    │
    └── web/                              # Adaptadores HTTP (Axum handlers)
        ├── presentations_handlers.rs     # GET, POST, PATCH, DELETE /api/presentations
        ├── items_handlers.rs             # POST, PATCH, DELETE /api/presentations/:id/items
        ├── audio_handlers.rs             # GET /api/audio/:id (Streaming con Range Requests)
        └── static_handlers.rs            # Servido de assets frontend
```

### 2.2 Frontend: Vanilla JS Micro-Modular (5 Niveles + Headless Behaviors + Utils)

El frontend no utiliza frameworks pesados ni bundlers complejos para el Slice 1. Corre nativamente en el navegador vía ES Modules (`type="module"`), con una correspondencia estricta 1:1 entre CSS y componentes:

```
frontend/
├── index.html                            # Shell mínimo con imports de tokens y punto de entrada JS
├── styles/
│   ├── tokens/                           # Variables CSS de diseño
│   │   ├── colors.css                    # Paleta oscura/escenario (alto contraste, fondo oscuro)
│   │   ├── spacing.css                   # Escala 4px/8px
│   │   ├── typography.css                # Tamaños legibles a distancia de atril
│   │   ├── elevations.css                # Sombras y capas z-index
│   │   └── motion.css                    # Transiciones suaves
│   ├── base/
│   │   ├── reset.css                     # Reset box-sizing y márgenes
│   │   └── typography.css                # Jerarquía base
│   ├── layouts/
│   │   ├── shell.css                     # Contenedor de aplicación mobile-first
│   │   └── timeline.css                  # Flujo vertical continuo ("chat-like")
│   ├── atoms/                            # CSS 1:1 con átomos JS
│   │   ├── button.css
│   │   ├── icon.css
│   │   ├── badge.css
│   │   ├── input.css
│   │   └── spinner.css
│   ├── molecules/                        # CSS 1:1 con moléculas JS
│   │   ├── audio-bar.css
│   │   ├── reorder-controls.css
│   │   └── action-menu.css
│   ├── organisms/                        # CSS 1:1 con organismos JS
│   │   ├── topbar.css
│   │   ├── presentation-card.css
│   │   ├── track-card.css
│   │   ├── lyrics-card.css
│   │   ├── note-card.css
│   │   └── add-item-modal.css
│   └── views/
│       ├── presentations-view.css
│       └── show-view.css
└── js/
    ├── main.js                           # Router simple y orquestador de vistas
    ├── utils/                            # 1 función pura = 1 archivo
    │   ├── dom/builder.js                # el(tag, props, children)
    │   ├── format/duration.js            # formatDuration(sec) -> "03:45"
    │   ├── format/bytes.js               # formatBytes(bytes) -> "4.2 MB"
    │   ├── async/debounce.js             # debounce(fn, ms)
    │   └── http/api.js                   # Wrapper fetch tipado para la API
    ├── behaviors/                        # Lógica interactiva pura (headless)
    │   ├── useAudioController.js         # Wrapper singleton de HTMLAudioElement con eventos
    │   ├── useClickOutside.js            # Cierre de modales/menús
    │   └── useKeyboardShortcuts.js       # Atajos: Espacio (play/pause), Esc (cerrar)
    ├── core/state/                       # Señales y micro-estado
    │   ├── createSignal.js               # Primitiva reactiva básica
    │   ├── activePresentation.js         # Señal de presentación seleccionada
    │   └── playbackState.js              # Señal de pista en reproducción activa
    └── components/                       # Sistema Atómico
        ├── atoms/
        │   ├── button.js
        │   ├── icon.js
        │   ├── badge.js
        │   └── input.js
        ├── molecules/
        │   ├── audio-controls.js         # Play/Pausa + slider de tiempo + badge duración
        │   └── reorder-controls.js       # Botones Subir / Bajar
        ├── organisms/
        │   ├── topbar.js                 # Título + volver + botón agregar
        │   ├── track-item.js             # Renderizado de pista con reproductor
        │   ├── lyrics-item.js            # Renderizado de letra tipográfica
        │   ├── note-item.js              # Renderizado de nota/acotación
        │   └── add-item-modal.js         # Modal para subir pista o escribir letra/nota
        └── views/
            ├── presentations-view.js     # Lista de shows con opciones CRUD
            └── show-view.js              # Línea de tiempo vertical de la presentación
```

---

## 3. Análisis de Riesgos Técnicos y Mitigaciones

| Riesgo Técnico | Impacto | Estrategia de Mitigación |
| :--- | :--- | :--- |
| **Reproducción y Seeking de Audio en Móvil (iOS/Android)** | Los navegadores móviles bloquean autoplay y exigen HTTP Range Requests (`206 Partial Content`) para permitir saltar en la línea de tiempo de `<audio>`. | 1. Implementar streaming en Axum con cabeceras `Accept-Ranges: bytes` y soporte de `Range` (o usar `tower-http::services::ServeFile`).<br>2. Mantener un único elemento `<audio>` global administrado por `useAudioController.js` para desbloquear el audio context con la primera interacción del usuario. |
| **Límite de Tamaño en Subida de Audio (Multipart)** | Axum por defecto restringe el cuerpo de las peticiones a 2MB; pistas MP3 o WAV de playback pueden superar los 30MB-80MB. | Configurar explícitamente `DefaultBodyLimit::max(100 * 1024 * 1024)` en la ruta de subida (`/api/presentations/:id/tracks`). |
| **Lógica de Reordenamiento y Consistencia en SQLite** | Reordenar elementos (subir/bajar) puede generar posiciones duplicadas o carreras si no se sincroniza bien. | 1. Usar índices enteros continuos (`0..N-1`) por presentación.<br>2. El caso de uso `ReorderItem` ejecuta un swap atómico en una única transacción SQLite (`BEGIN IMMEDIATE` / `COMMIT`). |
| **Concurrencia y Bloqueos en SQLite** | Múltiples peticiones simultáneas en Axum (servidor asíncrono Tokio) pueden toparse con `SQLITE_BUSY`. | 1. Habilitar modo WAL (`PRAGMA journal_mode = WAL;`) al abrir la base de datos.<br>2. Fijar `PRAGMA busy_timeout = 5000;`.<br>3. Usar pool de conexiones con `sqlx` gestionado en `AppState`. |
| **Servido de Assets Estáticos y Cache Local** | En vivo o con red intermitente, recargar la página no debe perder la vista actual. | 1. Backend sirve toda la carpeta `frontend/` estáticamente.<br>2. Enrutador frontend preserva `activePresentationId` en URL hash (ej: `#/show/:id`) o `localStorage`. |

---

## 4. Definición de Alcance: Slice 1 vs No-Objetivos Explícitos

### 4.1 En Alcance (Slice 1: Núcleo Usable)

1. **Gestión de Presentaciones:**
   - Listar todas las presentaciones existentes.
   - Crear una nueva presentación con título.
   - Abrir una presentación para ver su contenido.
   - Renombrar una presentación existente.
   - Eliminar una presentación (con borrado en cascada de sus elementos y archivos huérfanos).

2. **Línea de Tiempo Vertical (Contenido del Show):**
   - Visualización ordenada verticalmente de los elementos del show.
   - Soporte para 3 tipos de elementos:
     - **Pista:** Subida de archivo de audio (`.mp3`, `.wav`, `.m4a`), visualización de nombre de pista y barra con Play/Pausa.
     - **Letra:** Texto multilínea con formato y tipografía amplia para escenario.
     - **Nota:** Texto corto de aviso/directiva escénica destacado con badge visual.

3. **Operaciones sobre Elementos:**
   - Agregar nuevo elemento (modal o drawer para seleccionar tipo: Pista, Letra o Nota).
   - Eliminar un elemento.
   - Reordenar elementos mediante controles explícitos **Subir / Bajar** (Up / Down).

4. **Reproducción de Audio:**
   - Control de reproducción atómico (sólo una pista sonando al mismo tiempo en toda la app).
   - Mostrar tiempo actual y duración total.
   - Pausa automática al iniciar otra pista o al salir de la presentación.

5. **Diseño y Usabilidad:**
   - Interfaz mobile-first, modo oscuro por defecto (ideal para escenarios con poca luz).
   - Tipografía e iconos accesibles al tacto (touch targets mínimos de 44x44px).

### 4.2 No-Objetivos Explícitos (Fuera de Alcance para Slice 1)

- **Drag-and-Drop táctil completo:** El arrastre fluido con touch en móviles suele requerir polyfills o cálculos complejos de coordenadas (`useDragAndDrop`). Para Slice 1, los botones Subir/Bajar garantizan 100% de fiabilidad inmediata.
- **Funcionalidades de IA:** Detección de acordes, transcripción automática de audio, sincronización karaoke por IA, recomendaciones de repertorio.
- **Autenticación y Cuentas de Usuario:** Sin sistema de login, JWT ni sesiones; se asume uso local o red cerrada de ensayo.
- **Sincronización en tiempo real multi-dispositivo vía WebSockets:** Para Slice 1 la interacción es cliente-servidor HTTP estándar.
- **Service Worker / PWA Offline avanzada:** Cacheo avanzado sin conexión a nivel navegador diferido para slices posteriores.
- **Almacenamiento en la Nube (S3 / R2):** El audio se almacena localmente en el disco del host.

---

## 5. Estrategia de Entrega y Presupuesto de Revisión (400 Líneas)

Para respetar el umbral de revisión (`review_budget: 400 líneas`):

1. **Fase 1 (Cimientos & Persistencia):**
   - Configuración `Cargo.toml`, entidades de dominio, esquema SQLite (`presentations`, `items`) y repositorio básico.
2. **Fase 2 (Casos de Uso & API Axum de Presentaciones):**
   - Casos de uso CRUD de presentaciones + handlers HTTP + tests dirigidos.
3. **Fase 3 (Casos de Uso & API de Items + Audio Upload/Streaming):**
   - Casos de uso de items (letra, nota, subida de pista, reordenar) + handlers HTTP + streaming de audio con Range.
4. **Fase 4 (Frontend Shell, Tokens & Átomos UI):**
   - Estructura HTML, CSS tokens/base/atoms y utilidades puras DOM/HTTP.
5. **Fase 5 (Frontend Vistas & Reproductor de Audio):**
   - Organismos (timeline, card de pista con audio controller, modales) y orquestación de vistas.
