# Verificación Responsive con Navegador Real (Chromium CDP)

Este documento describe la arquitectura, ejecución e interpretación del arnés de verificación responsive automatizado para **Shirley Show**, el cual evalúa el comportamiento real del layout en un navegador Chromium headless a través de Chrome DevTools Protocol (CDP).

---

## 1. Objetivos y Filosofía

1. **Navegador real, cero suposiciones**: La verificación estática de CSS o el análisis con DOM simulado (como jsdom o happy-dom) no calculan reflows reales, subpíxeles, scrollbars móviles, ni el comportamiento de viewport meta (`width=device-width, viewport-fit=cover`). Este arnés utiliza una instancia real de Chromium.
2. **Cero dependencias npm nuevas**: Aprovecha las capacidades de la plataforma: Node.js 24 (`WebSocket` global nativo, `fetch`, `util.parseArgs`) y CDP sin bibliotecas pesadas como Puppeteer o Playwright.
3. **Cobertura exhaustiva de anchos**: Evalúa cada ruta crítica en 9 anchos de pantalla:
   - `320px`: Móvil ultra compacto (iPhone SE 1ra gen).
   - `360px`: Android compacto estándar.
   - `390px`: iPhone moderno (12/13/14/15 base).
   - `414px`: iPhone Plus / XR.
   - `480px`: Móvil ancho / phablet.
   - `640px`: Breakpoint móvil/tablet inicial.
   - `768px`: Tablet en orientación vertical (iPad portrait).
   - `1024px`: Tablet horizontal / laptop compacta.
   - `1440px`: Escritorio / desktop estándar.
4. **Pruebas no intrusivas**: El test de integración en Rust (`tests/responsive_browser_tests.rs`) está marcado como `#[ignore]` para no ralentizar la suite unitaria estándar (`cargo test`), pero puede ejecutarse a demanda.

---

## 2. Arquitectura del Arnés

El sistema consta de 4 componentes cohesivos:

```text
tests/responsive_browser_tests.rs
       │ (cargo test -- --ignored)
       ▼
scripts/verify-responsive.sh
       │
       ├─► 1. Inicia backend shirley-show (puerto efímero + base temporal)
       ├─► 2. Siembra datos realistas vía API REST (presentación, pistas, evento, entradas, afiche)
       ├─► 3. Ejecuta scripts/verify-responsive.mjs
       ├─► 4. Limpia procesos y archivos temporales al salir (trap EXIT)
       └─► 5. Propaga el código de salida
              │
              ▼
scripts/verify-responsive.mjs
       │
       ├─► Arranca Chromium headless (--headless=new, CDP sobre WebSocket)
       ├─► Matriz de 54 pruebas: 6 rutas × 9 anchos de pantalla
       ├─► Aplica Emulation.setDeviceMetricsOverride (mobile: true)
       ├─► Evalúa probe de layout en el contexto del DOM real
       ├─► Captura PNG de alta resolución por cada estado
       └─► Genera reporte legible en terminal y archivo JSON estructurado
```

---

## 3. Requisitos del Entorno

- **Chromium** instalado y disponible en el `PATH` (o ejecutable binario compatible).
- **Node.js >= 24** (incluye soporte global de `WebSocket`).
- **Cargo y Rust** para compilar y ejecutar el servidor y la prueba.

---

## 4. Instrucciones de Ejecución

### Opción A: A través de Cargo (Recomendado para CI / Verificación Local)

Ejecuta la prueba de integración dedicada con salida en tiempo real:

```bash
cargo test --test responsive_browser_tests -- --ignored --nocapture
```

### Opción B: Ejecución Directa mediante Script Bash

Levanta el servidor con base de datos temporal, siembra datos, corre la verificación completa y limpia todo automáticamente:

```bash
./scripts/verify-responsive.sh
```

### Opción C: Ejecución del Runner Node contra un Servidor Existente

Si ya tienes una instancia de Shirley Show corriendo (por ejemplo en `http://127.0.0.1:3000`):

```bash
node scripts/verify-responsive.mjs \
  --base-url="http://127.0.0.1:3000" \
  --presentation-id="<UUID-PRESENTACION>" \
  --event-id="<UUID-EVENTO>" \
  --ticket-token="<TOKEN-ENTRADA>" \
  --screenshots-dir="./target/responsive-screenshots"
```

Opciones disponibles en `verify-responsive.mjs`:

| Opción | Descripción | Valor por defecto |
| :--- | :--- | :--- |
| `--base-url` | URL base del servidor frontend/backend | `http://127.0.0.1:3000` (o `SHIRLEY_BASE_URL`) |
| `--presentation-id` | UUID de presentación para vista de detalle | Auto-detectado o vacío |
| `--event-id` | UUID de evento para vista de detalle y puerta | Auto-detectado o vacío |
| `--ticket-token` | Scan token público de entrada para `/ticket/:token` | Auto-detectado o vacío |
| `--screenshots-dir` | Directorio de salida para capturas PNG | `./target/responsive-screenshots` |
| `--report-file` | Ruta personalizada para el reporte JSON | `<screenshots-dir>/responsive-report.json` |
| `--chrome-port` | Puerto de depuración remota de Chromium | Puerto efímero asignado dinámicamente |

---

## 5. Rutas y Aspectos Verificados

### 5.1 Rutas Verificadas (6 Rutas Principales)

1. **`#/presentations` (Catálogo de Presentaciones)**:
   - Hero card de bienvenida con IA Ressy.
   - Barra superior de navegación y tarjetas de presentación.
2. **`#/presentations/:id` (Detalle de Presentación)**:
   - Encabezado con título y acciones de show (`Modo Escenario`, adición rápida).
   - Línea de tiempo con pistas de audio, notas de dirección y letras sincronizadas.
3. **`#/eventos` (Catálogo de Eventos)**:
   - Tarjetas de evento con afiche, fechas, aforo y estado de venta.
4. **`#/eventos/:id` (Detalle de Evento)**:
   - Pestañas de navegación (`Resumen`, `Entradas`, `Interesados`, `Gastos`).
   - Tablas financieras y desglose de medios de pago en pantallas móviles.
5. **`#/ticket/:token` (Entrada Digital Pública)**:
   - Tarjeta de entrada digital con código QR en SVG renderizado.
   - Botón de descarga de foto/afiche con branding oficial.
6. **`#/puerta/:id` (Control de Acceso en Puerta)**:
   - Visor de cámara para lectura de QR y formulario accesible de ingreso manual.

### 5.2 Probes de Layout y Tolerancia

El probe evaluado en cada ruta y ancho realiza cuatro comprobaciones clave:

1. **Desborde Horizontal (`scrollWidth vs innerWidth`)**:
   - Comprueba si `document.documentElement.scrollWidth` o `document.body.scrollWidth` exceden el ancho visible del viewport.
   - Identifica elementos visibles cuyo borde derecho (`rect.right`) sobrepasa el viewport (`window.innerWidth + 1.5px`).
   - **Tolerancia integrada**: Ignora legítimamente elementos contenidos dentro de contenedores con scroll horizontal explícito (`overflow-x: auto` o `overflow-x: scroll`, como la barra de pestañas `.events-tabs-nav` o tablas responsivas envueltas en `.events-table-wrapper`).
   - **Tolerancia a elementos ocultos**: Descarta nodos con `display: none`, `visibility: hidden`, `opacity: 0`, diálogos no abiertos (`dialog:not([open])`) o marcados con `aria-hidden="true"`.
2. **Controles Fuera de Pantalla**:
   - Detecta si botones, enlaces interactivos o campos de formulario quedan cortados por el margen izquierdo (`rect.left < -2px`) o derecho (`rect.right > innerWidth + 1.5px`).
3. **Touch Targets Pequeños (< 44px)**:
   - Revisa botones (`button`, `[role="button"]`), campos (`input`, `select`, `textarea`) y pestañas (`[role="tab"]`).
   - Distingue violaciones críticas (botones de acción con ancho y alto < 44px, o < 32px en móvil) de avisos informativos (píldoras sugeridas o enlaces de texto inline amparados por la excepción de texto corrido de WCAG 2.5.8).
   - Respeta delegación de hit-target en etiquetas de formulario (`<label>` envolvente).
4. **Visibilidad de Expectativas de Ruta**:
   - Verifica que los contenedores y elementos semánticos principales de cada vista se encuentren montados y visibles en el DOM, confirmando que el enrutador por hash resolvió y renderizó la vista sin errores de JavaScript.

---

## 6. Interpretación de Reportes y Capturas

### Salida en Terminal

Cada combinación ruta × ancho produce una línea de estado inmediata:

```text
[Ruta] Catálogo de Presentaciones (#/presentations)
  ✅ PASS |  320px | docWidth=320px (0px overflow) | 1 touch <44px (0 críticos) | 📸 01-catalog-320px.png
  ✅ PASS |  360px | docWidth=360px (0px overflow) | 2 touch <44px (0 críticos) | 📸 01-catalog-360px.png
  ...
```

- **`✅ PASS`**: Todos los criterios de layout se cumplieron sin desbordes ni fallos de expectativas.
- **`❌ FAIL`**: Se detectó una violación real (desborde horizontal no scrolleable, control cortado o elemento principal ausente). El script imprime el selector, dimensiones, texto del elemento afectado y finaliza con código de salida `1`.

### Capturas PNG

Las capturas se almacenan en `target/responsive-screenshots/` con la nomenclatura:

```text
<indice>-<nombre-ruta>-<ancho>px.png
```

Ejemplos:
- `01-catalog-320px.png`
- `02-presentation-detail-390px.png`
- `04-event-detail-640px.png`
- `05-public-ticket-320px.png`

Estas imágenes permiten una auditoría visual directa y facilitan la detección de problemas estéticos de wrapping, legibilidad o espaciado tipográfico.

### Reporte JSON Estructurado

En `target/responsive-screenshots/responsive-report.json` se exporta el detalle completo:

```json
{
  "timestamp": "2026-09-19T11:25:00.000Z",
  "baseUrl": "http://127.0.0.1:39541",
  "totalChecks": 54,
  "passedChecks": 54,
  "failedChecks": 0,
  "totalViolations": 0,
  "widths": [320, 360, 390, 414, 480, 640, 768, 1024, 1440],
  "results": [ ... ]
}
```

---

## 7. Mantenimiento y Extensibilidad

- Para agregar nuevas rutas, añada un objeto a la función `buildRoutesList()` en `scripts/verify-responsive.mjs`.
- Para ajustar o incorporar nuevos breakpoints de verificación, modifique el array `VIEWPORT_WIDTHS` en `scripts/verify-responsive.mjs`.
- Los datos sembrados para pruebas pueden ajustarse en la sección de llamadas API de `scripts/verify-responsive.sh`.
