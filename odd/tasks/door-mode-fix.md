# Modo Puerta — Escáner real + Venta rápida

## Goal
El modo puerta (#/puerta/:id) debe servir en un evento en vivo: (1) escanear QR de entradas con la cámara del celular en cualquier navegador moderno, y (2) vender entradas en puerta sin pedir nombre, con cantidad, medio de pago (Efectivo/Yape), total automático según precio General del evento, mostrando el QR de Yape si corresponde, y registrando la venta en la web.

## Acceptance criteria
- La cámara escanea QR reales en Chrome/Safari Android/iOS: BarcodeDetector donde exista, fallback jsQR con canvas en el resto. Sin jsQR el escáner queda en la mitad de los teléfonos.
- Siempre visible el fallback manual (selectores `.door-manual-input`, `.door-manual-form button` intactos — los exige `responsive_mobile_tests.rs`).
- Pestaña "Vender": cantidad (stepper 1+), medio Efectivo/Yape, total = precio General × cantidad calculado en vivo, QR de Yape visible al elegir Yape, venta registrada vía `sellTickets` con nombre interno "Venta en puerta" (backend exige nombre no vacío; contacto vacío permitido), confirmación inline y contador.
- Sin cambios de backend. Sin cambios de contrato API.
- Tests: `node --test frontend/test/door-controller.test.js` y demás suites de `frontend/test/` en verde.

## Tasks
1. [x] **DOOR-1 — Fallback jsQR + UX de escaneo.** vendor jsQR local, `camera-qr-scanner.js` con captura a canvas y `jsQR()`, botón Abrir/Cerrar cámara con manejo de errores de permiso, deduplicación conservada.
2. [x] **DOOR-2 — Pestaña Vender en puerta.** tabs Escanear/Vender en `door-controller.js`, venta sin nombre, total automático desde tipos del evento, QR Yape (`/assets/public/yape-qr.jpg`), estilos nuevos en `frontend/styles/events/index.css`, tests que cubren pestañas, total y registro vía `sellTickets`.
3. [x] **DOOR-3 — Verificación.** `node --test` de `frontend/test/door-controller.test.js` + resto de `frontend/test/`, `git diff --check`. Sin commit salvo autorización.

## Evidence
- `node --test frontend/test/door-controller.test.js` — pass 1, fail 0.
- `node --test frontend/test/*.test.js` (6 suites incl. events-ui, public-ticket, router, whatsapp, dialog-dismiss) — pass 6, fail 0.
- `cargo test --test responsive_mobile_tests` (vía writer) — 15 passed, 0 failed.
- `git diff --check` — limpio.
- Archivos: `frontend/js/modules/events/camera-qr-scanner.js`, `frontend/js/modules/events/door-controller.js`, `frontend/index.html`, `frontend/styles/events/index.css`, `frontend/test/door-controller.test.js` + nuevos sin commit `frontend/js/vendor/jsqr.min.js`, `frontend/public/yape-qr.jpg`.
- Sin commit (sin autorización).
