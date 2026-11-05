# Despliegue en Coolify (eje-desarrollo)

Shirley Show corre en Coolify **sin GPU**. La separación vocal se delega a
Google Colab (`colab` CLI), que se coordina desde el contenedor.

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
| `OPENCODE_API_KEY` | key de OpenCode Go (plan Go) | Requerida. Guarda como secreto. |
| `OPENCODE_MODEL` | `deepseek-v4.1-flash` | Opcional (por defecto DeepSeek V4.1 Flash). |
| `OPENCODE_BASE_URL` | `https://opencode.ai/inference/go/openai/v1` | Opcional. |
| `GROQ_API_KEY` / `GROQ_MODEL` | `gsk_...` | Opcional, solo como fallback. |
| `SSH_KEY_HOST_PATH` | `/opt/shirley/secrets/id_ed25519` | Ruta en el **host**. |
| `SHIRLEY_PUBLIC_BASE_URL` | `https://shirley.example.com` | Base del enlace del ticket que se envía por WhatsApp. Sin ella el enlace sale relativo y el comprador no puede abrirlo. |

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

Si la sesión de Colab no está disponible o falla, la app sigue 100%
operativa; Ressy avisa amablemente y pide reintentar la separación.
