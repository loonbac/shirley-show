# Arquitectura y convenciones

El código está pensado para que una IA (o cualquier persona) pueda seguir trabajándolo sin perderse.
**Respeta estas reglas al agregar o modificar código.**

## Principios

1. **Modularidad total.** Archivos cortos (~150 líneas máx). Si crece, divídelo en subcomponentes,
   hooks o utilidades.
2. **Una responsabilidad por archivo.** Presentación en `components/`, estado/lógica en `hooks/`,
   acceso a datos en `services/`. No los mezcles.
3. **Reutiliza, no dupliques.** Si algo se usa en dos lugares, conviértelo en componente o hook compartido.
4. **Features autocontenidas.** Cada funcionalidad vive en su carpeta bajo `src/features/`.
5. **Carga diferida.** Cada feature se importa con `React.lazy` desde `src/App.jsx`.

## Estructura

```
server/                 # API local (Express + node:sqlite)
  config.js             # carga .env y valores por defecto
  index.js              # app Express: monta rutas y sirve dist/ en producción
  db/                   # connection.js, schema.sql, seed.js
  lib/                  # codigos (QR únicos), jwt, errores
  middleware/           # requireAuth (protege el panel)
  services/             # lógica de negocio por entidad (eventos, entradas, checkin, resumen, auth)
  routes/               # endpoints HTTP (delgados; delegan en services)

src/
  components/           # UI genérica y reutilizable (Boton, Input, Select, Modal, Tarjeta,
                        #   Badge, Campo, Estado, qr/VisorQR…) — sin lógica de negocio
  hooks/                # hooks globales reutilizables (usePeticion)
  lib/                  # api (cliente HTTP), format (moneda/fechas)
  routes/               # RutaProtegida
  features/
    auth/               # login y sesión (ProveedorSesion, useSesion)
    eventos/            # lista, crear/editar, detalle (pantalla principal)
    entradas/           # registrar venta, lista, ver/compartir, vista pública
    checkin/            # modo puerta (escáner de cámara)
    resumen/            # métricas y desglose por medio de pago
  App.jsx               # router con carga diferida de cada feature
  main.jsx              # punto de entrada
```

Cada feature sigue el mismo molde:

```
features/<nombre>/
  components/   # piezas de UI propias de la feature
  hooks/        # estado y lógica
  services/     # llamadas al API (usan src/lib/api.js)
  pages/        # pantallas enrutables (export default)
  index.js      # punto de entrada: re-exporta lo público de la feature
```

## Flujo de datos

`components` → llaman a `hooks` → usan `services` → usan `src/lib/api.js` → API Express → `services` (server) → SQLite.

Para refrescar datos relacionados (p. ej. al confirmar un pago se actualizan lista **y** resumen),
`DetalleEventoPage` mantiene un contador `version` que se pasa a los hooks como dependencia.

## Cómo agregar una feature nueva (ej. "comida para artistas")

1. Crea `src/features/comida/` con `components/`, `hooks/`, `services/`, `pages/`, `index.js`.
2. En el backend, añade `server/services/comidaService.js` y `server/routes/comida.js`, y móntalo
   en `server/index.js` (protégelo con `requireAuth` si es del panel).
3. Engancha la página en `src/App.jsx` con `lazy(() => import('./features/comida/pages/…'))`.

No hace falta tocar las demás features.

## Modelo de datos

- **eventos**: `nombre`, `fecha_hora`, `lugar`, `aforo?`, `tipos_de_entrada` (JSON `[{nombre, precio}]`).
  `estado` (Próximo/Finalizado) se deriva de la fecha, no se guarda.
- **entradas**: una por persona. `codigo_qr` único e irrepetible (16 bytes aleatorios → 32 hex,
  no adivinable). `estado_pago` (Pagado/Pendiente), `estado_checkin` (Pendiente/Usada), `hora_ingreso`.
