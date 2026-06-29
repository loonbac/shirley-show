# Despliegue — Docker, Coolify y Cloudflare Tunnel (shirleyshow.site)

El backend es **Go** y el frontend **React**; ambos quedan en **un solo contenedor** (el
servidor Go sirve el API y el sitio compilado). La base es **SQLite** en un volumen.

---

## Modo desarrollo (local, sin Docker)

Requisitos: **Go 1.25+**, **Node 20+** y **pnpm**.

```bash
cp .env.example .env        # ajusta JWT_SECRET y credenciales si quieres
pnpm install                # dependencias del frontend
pnpm seed                   # crea la administradora y un evento de ejemplo (usa Go)
pnpm dev                    # levanta el backend Go (:3001) y Vite (:5173) a la vez
```

Abre la URL que imprime Vite. El backend Go corre en `:3001` y Vite le redirige `/api`.
Usuario/clave por defecto: `admin` / `admin123`.

> Solo backend: `pnpm dev:api` (= `go run ./cmd/server`). Solo frontend: `pnpm dev:web`.

---

## Modo producción (Docker)

Construye una imagen única (build de React + binario Go estático):

```bash
docker build -t shirley-show .
docker run -p 3001:3001 \
  -e JWT_SECRET="pon-un-secreto-largo" \
  -e ADMIN_PASSWORD="tu-clave" \
  -v shirley_datos:/app/data \
  shirley-show
```

La administradora se crea sola al arrancar (idempotente). Abre `http://localhost:3001`.

---

## Opción A — Coolify (recomendado)

1. Sube este repo a GitHub (privado).
2. En Coolify: **New Resource → Application → desde tu repo** (build pack: **Dockerfile**).
3. **Puerto**: 3001.
4. **Variables de entorno**:
   - `JWT_SECRET` = (un secreto largo y aleatorio)
   - `ADMIN_USUARIO`, `ADMIN_PASSWORD` = credenciales de la cantante
   - `WHATSAPP_ON` = `true` si usarás WhatsApp directo (ver abajo)
5. **Almacenamiento persistente**: monta un volumen en `/app/data` (¡importante! ahí vive la base).
6. **Dominio**: pon `shirleyshow.site`. Coolify gestiona HTTPS automáticamente, o úsalo detrás
   de Cloudflare Tunnel (Opción B).

Cada `git push` puede redeployar solo si activas el auto-deploy.

## Opción B — Cloudflare Tunnel (con docker-compose)

1. En Cloudflare **Zero Trust → Networks → Tunnels**, crea un túnel y copia su **token**.
2. En el túnel, agrega un *public hostname*: `shirleyshow.site` → servicio `http://app:3001`.
3. Crea un `.env` junto al `docker-compose.yml`:
   ```
   JWT_SECRET=un-secreto-largo
   ADMIN_PASSWORD=tu-clave
   CLOUDFLARE_TUNNEL_TOKEN=el-token-del-tunel
   WHATSAPP_ON=false
   ```
4. `docker compose up -d --build`

Cloudflare pone el HTTPS y el dominio; no hay que abrir puertos en el servidor.

---

## WhatsApp directo (whatsmeow) — opcional

Con `WHATSAPP_ON=true`, el servidor puede **enviar la entrada directamente** por WhatsApp
(en vez del enlace `wa.me`). Requiere emparejar **una vez** el WhatsApp de la cantante:

1. Inicia sesión en el panel y abre el estado de WhatsApp (`POST /api/whatsapp/conectar`).
2. El servidor genera un **código QR**; escanéalo desde *WhatsApp → Dispositivos vinculados*.
3. La sesión queda guardada en `/app/data/whatsapp.db` (por eso el volumen es importante).

> El emparejamiento es un trámite de una sola vez; luego el envío es automático.

---

## Respaldo

Todo vive en el volumen `/app/data` (archivos `eventos.db` y `whatsapp.db`). Respáldalo
copiando esos archivos.
