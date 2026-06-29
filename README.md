# Shirley Show — Gestión de Eventos

Aplicación web **de uso interno** para que la cantante gestione las ventas de entradas de sus
eventos: registrar ventas, generar entradas con **QR musical único**, controlar pagos, ver el
resumen recaudado y validar entradas en la puerta escaneando el QR.

> Solo para la administradora. El público compra por WhatsApp y ella registra cada venta.

## Stack

- **Frontend:** React + Vite (mobile-first, español, letras grandes, QR con notas musicales).
- **Backend:** **Go** (net/http, enrutado de Go 1.22) — un solo binario que sirve el API y el sitio.
- **Base de datos:** **SQLite** con `modernc.org/sqlite` (Go puro, **sin CGO**).
- **Auth:** JWT + bcrypt.
- **WhatsApp directo (opcional):** `whatsmeow` para enviar la entrada sin pasos manuales.
- **Despliegue:** Docker (imagen única) → Coolify y/o Cloudflare Tunnel en `shirleyshow.site`.

## Desarrollo local (sin Docker)

Requisitos: **Go 1.25+**, **Node 20+**, **pnpm**.

```bash
cp .env.example .env
pnpm install        # frontend
pnpm seed           # administradora + evento de ejemplo (Go)
pnpm dev            # backend Go (:3001) + Vite (:5173) juntos
```

Abre la URL de Vite. Usuario/clave por defecto: `admin` / `admin123`.

## Producción (Docker)

```bash
docker build -t shirley-show .
docker run -p 3001:3001 -e JWT_SECRET=... -e ADMIN_PASSWORD=... -v shirley_datos:/app/data shirley-show
```

La administradora se crea sola al arrancar. Ver **[DESPLIEGUE.md](./DESPLIEGUE.md)** para
Coolify, Cloudflare Tunnel y el emparejamiento de WhatsApp.

## Estructura

```
cmd/server/     # punto de entrada del servidor Go
cmd/seed/       # carga datos de ejemplo
internal/
  config/       # configuración por entorno (.env)
  store/        # capa de datos (SQLite) + modelos
  auth/         # JWT + bcrypt
  api/          # handlers HTTP, router, estáticos (SPA)
  whatsapp/     # cliente whatsmeow (envío directo)
src/            # frontend React (features autocontenidas, lazy loading)
Dockerfile      # build de React + binario Go en una imagen mínima
```

Ver **[ARQUITECTURA.md](./ARQUITECTURA.md)** para las convenciones del frontend.
