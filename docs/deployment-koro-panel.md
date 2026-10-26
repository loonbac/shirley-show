# Despliegue en koro-panel

koro-panel no lee `compose.yaml`: construye un Dockerfile y ejecuta un `docker run`
propio. Todo lo que necesita saber está en esta página y en el TOML del proyecto.

Para Coolify, que sí usa Compose, ver `docs/deployment-coolify.md`.

## 1. Lo que la imagen ya cumple

| Requisito del panel | Cómo lo cumple este repo |
|---|---|
| Escuchar en `0.0.0.0` | `SHIRLEY_BIND=0.0.0.0:3001` viene fijado en el Dockerfile |
| Configuración por entorno | `src/config.rs` lee todo de variables; nada está compilado dentro |
| Estado fuera del contenedor | `SHIRLEY_DATA_DIR=/app/data` es el único directorio con escritura |
| Logs por stdout/stderr | La app solo usa `println!`/`eprintln!` |
| Dockerfile en la raíz | `./Dockerfile`, dos etapas, usuario final no root (UID 1000) |
| `.dockerignore` | Excluye `.git`, `target/`, `data/`, `production-data/`, `audio/`, `.env` |
| SIGTERM | `src/main.rs` instala un handler de `terminate` con apagado ordenado |

## 2. Declaración

`/srv/panel/projects/<id>.toml`:

```toml
id = "app-shir"
label = "Shirley Show"
description = "Escenario, setlist y ticketing con asistente Ressy"

build = "/srv/repos/shirley-show"   # ruta absoluta EN EL SERVIDOR, con el Dockerfile
port = 3001                          # puerto interno; el de SHIRLEY_BIND
host_port = 65529                    # opcional: solo si nginx debe alcanzarlo

memory = "1g"                        # opcional; ver §5

volumes = [
  "/srv/shirley/data:/app/data",
  "/srv/shirley/secrets/id_ed25519:/home/shirley/.ssh/id_ed25519:ro",
]
```

Dos advertencias sobre los volúmenes:

- **El directorio del host debe existir y pertenecer a UID 1000.** Docker crea
  el directorio faltante como root y entonces la app no puede escribir. Créalo
  antes con `install -d -o 1000 -g 1000 /srv/shirley/data`.
- **La clave SSH tiene que ser un archivo, no un directorio.** Si
  `/srv/shirley/secrets/id_ed25519` no existe, Docker crea un *directorio* en esa
  ruta y el contenedor arranca con una clave inservible; el fallo aparece recién
  al separar voces. Si no vas a usar la PC remota, elimina esa línea del TOML y
  deja `JOSHUA_PC_HOST` vacío.

## 3. Variables de entorno

El panel escribe `/srv/panel/projects/<id>.env` (modo 0600) y lo pasa con
`--env-file`. Lista completa en `.env.example`. Las que importan:

| Variable | Ejemplo | Efecto si falta |
|---|---|---|
| `CMD_API_KEY` | key de Command Code | Ressy responde "no configurado"; el resto funciona |
| `CMD_MODEL` | `stealth/space-bunny-alpha` | usa el valor por defecto |
| `SHIRLEY_PUBLIC_BASE_URL` | `https://shirley.example.com` | **el enlace del ticket sale relativo** y el comprador no puede abrirlo |
| `JOSHUA_PC_HOST` | `100.73.247.39` | separación vocal local, sin GPU remota |
| `SHIRLEY_DATA_DIR` | `/app/data` | ya viene del Dockerfile; no lo cambies |
| `SHIRLEY_BIND` | `0.0.0.0:3001` | ya viene del Dockerfile; no lo cambies |

`SHIRLEY_PUBLIC_BASE_URL` es la única que el panel no puede inferir y cuya
ausencia rompe una función visible para el público.

## 4. Puertos

Hay tres números distintos en juego y conviene no confundirlos:

| Número | Dónde vive | Qué es |
|---|---|---|
| `3001` | `port` del TOML y `SHIRLEY_BIND` | el puerto **dentro** del contenedor |
| `host_port` | TOML | se publica como `127.0.0.1:<host_port>:3001` |
| `65529` | `compose.yaml` | solo para Compose/Coolify; el panel lo ignora |

Publica `host_port` únicamente si nginx en el host tiene que alcanzar la app. El
panel no conecta proyectos en una red compartida: un contenedor proxy no llega a
`panel-<id>` por nombre, necesita el puerto de loopback.

## 5. Memoria

La app mantiene SQLite, los audios en staging y las descargas de yt-dlp dentro del
contenedor. Un tope bajo la mata en silencio: el kernel la termina y el panel
muestra "detenido" sin línea de log. Con 2 GB de VPS, `memory = "1g"` es un punto
de partida razonable; si ves reinicios con audios grandes, súbelo.

## 6. Comprobación a mano antes de confiar en la interfaz

El panel solo reporta la primera línea del error de `docker run`, así que conviene
reproducir su invocación exacta una vez:

```sh
cd /srv/repos/shirley-show
docker build -t panel-app-shir:latest .
docker run -d --name prueba --restart unless-stopped \
  --label panel.project=app-shir \
  --memory 1g \
  --env-file /srv/panel/projects/app-shir.env \
  -v /srv/shirley/data:/app/data \
  -v /srv/shirley/secrets/id_ed25519:/home/shirley/.ssh/id_ed25519:ro \
  -p 127.0.0.1:65529:3001 \
  panel-app-shir:latest

curl -sS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:65529/api/presentations
docker logs --tail 20 prueba
docker rm -f prueba
```

`200` significa que la app responde. `502` desde el proxy con el contenedor
"en marcha" casi siempre es `SHIRLEY_BIND` atado a `127.0.0.1` o un `port` que no
coincide con el real.

## 7. Lo que el panel no puede verificar

- Que la app responda: el estado es `docker inspect .State.Status`, no un healthcheck.
- Que el enlace del ticket funcione: depende de `SHIRLEY_PUBLIC_BASE_URL`.
- Que la separación vocal remota funcione: depende de Tailscale y de la clave SSH.
- Que los datos sobrevivan: haz un segundo despliegue y comprueba que los shows
  siguen ahí. Un dato escrito dentro del contenedor se pierde.

## 8. Red hacia la PC de Joshua

El contenedor sale a `tailscale0` del host por NAT de Docker. No requiere
`network_mode: host`; basta con que el host tenga Tailscale arriba y alcance
`100.73.247.39`. Para diagnosticarlo desde el contenedor:

```sh
docker exec -it panel-app-shir ssh -i /home/shirley/.ssh/id_ed25519 \
  -o BatchMode=yes loonbac@100.73.247.39 true && echo OK
```
