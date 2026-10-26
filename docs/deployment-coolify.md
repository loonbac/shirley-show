# Despliegue en Coolify (eje-desarrollo)

Shirley Show corre en Coolify **sin GPU**. La separación vocal se delega por SSH
a la PC de Joshua (`nixos-pc`, Tailscale `100.73.247.39`, RTX 3060).

## 1. Requisitos en eje-desarrollo

- Docker + Compose (los provee Coolify).
- Tailscale activo y logueado en la misma tailnet (`tailscale status` debe mostrar `nixos-pc`).
- Conectividad: `ping 100.73.247.39` y `ssh loonbac@100.73.247.39 true`.

## 2. Clave SSH (una vez)

En **nixos-pc**, autoriza la clave que usará el contenedor:

```bash
cat clave-publica-del-servidor.pub >> ~/.ssh/authorized_keys
```

En **eje-desarrollo**, guarda la privada con permisos estrictos, dueño UID 1000:

```bash
mkdir -p /opt/shirley/secrets
cp id_ed25519 /opt/shirley/secrets/
chmod 600 /opt/shirley/secrets/id_ed25519
chown 1000:1000 /opt/shirley/secrets/id_ed25519
```

Apunta `SSH_KEY_HOST_PATH=/opt/shirley/secrets/id_ed25519` en el entorno de Coolify.

## 3. Sudo sin password en nixos-pc (una vez)

Para el apagado automático por inactividad, en `configuration.nix`:

```nix
security.sudo.extraRules = [{
  users = [ "loonbac" ];
  commands = [{
    command = "/run/current-system/sw/bin/shutdown";
    options = [ "NOPASSWD" ];
  }];
}];
```

Verifica desde el servidor: `ssh loonbac@100.73.247.39 "sudo -n /run/current-system/sw/bin/shutdown -c"`.

## 4. Variables en Coolify

| Variable | Ejemplo | Notas |
|---|---|---|
| `CMD_API_KEY` | key de Command Code Provider API | Requerida. Guarda como secreto. |
| `CMD_MODEL` | `stealth/space-bunny-alpha` | Opcional (por defecto Space Bunny Alpha). |
| `CMD_BASE_URL` | `https://api.commandcode.ai/provider/v1` | Opcional. |
| `GROQ_API_KEY` / `GROQ_MODEL` | `gsk_...` | Opcional, solo como fallback. |
| `JOSHUA_PC_HOST` | `100.73.247.39` | Vacío = modo local (sin GPU remota). |
| `JOSHUA_PC_USER` / `JOSHUA_PC_PORT` | `loonbac` / `22` | Opcional. |
| `JOSHUA_PC_SSH_KEY` | `/home/shirley/.ssh/id_ed25519` | Ruta **dentro** del contenedor. |
| `SSH_KEY_HOST_PATH` | `/opt/shirley/secrets/id_ed25519` | Ruta en el **host**. |
| `JOSHUA_PC_WAKE_TIMEOUT_MINS` | `10` | Espera a que enciendan la PC. |
| `SHIRLEY_PUBLIC_BASE_URL` | `https://shirley.example.com` | Base del enlace del ticket que se envía por WhatsApp. Sin ella el enlace sale relativo y el comprador no puede abrirlo. |

> `JOSHUA_PC_IDLE_SHUTDOWN_MINS` ya **no** existe: el código nunca la leyó. El
> apagado por inactividad lo programa la propia app tras cada separación exitosa,
> y el tiempo lo fija `JOSHUA_PC_WAKE_TIMEOUT_MINS` como margen de espera.

## 5. Red: bridge con publicación en loopback

El `compose.yaml` usa **bridge** y publica `127.0.0.1:65529:3001`. El contenedor
sale a `tailscale0` del host por NAT de Docker, que es el caso normal y no
necesita nada extra.

Si el contenedor no alcanza `100.73.247.39`:

- **Verifica primero el host**: `ssh loonbac@100.73.247.39 true` desde el servidor.
  Si el host no llega, el contenedor tampoco.
- **Forwarding**: `sysctl -w net.ipv4.ip_forward=1` e iptables MASQUERADE entre
  `docker0` y `tailscale0` (ver wiki de Tailscale + Docker).
- **Sidecar**: añade `tailscale/tailscale` con `TS_AUTHKEY` y usa
  `network_mode: service:tailscale` en el servicio de la app. El sidecar debe
  publicar el puerto, no la app.

## 6. Proxy y timeouts largos

Separar voces con la PC apagada puede tomar 10+ minutos de espera.
Sube los timeouts del proxy (Traefik: `respondingTimeouts.readTimeout: 1500s`
o equivalente en Nginx `proxy_read_timeout 1500s`).

## 7. Primer arranque

```bash
docker compose up -d --build
curl http://127.0.0.1:65529/api/presentations
```

Datos en el volumen `shirley-data` (`/app/data`: SQLite, audios, posters).
Healthcheck: `GET /api/presentations`.

El puerto publicado es **65529** en el host hacia **3001** dentro del contenedor.
El 3100 de `.env.example` es el valor de desarrollo local, no el del contenedor.

Para desplegar con **koro-panel** en lugar de Compose, ver
`docs/deployment-koro-panel.md`.

## 8. Sin GPU remota

Si `JOSHUA_PC_HOST` está vacío o la PC no responde, la app sigue 100%
operativa; Ressy avisa amablemente y pide encender la PC de Joshua.
