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
  "/srv/shirley/secrets/colab-token.json:/home/shirley/.config/colab/token.json",
]
```

Tres advertencias sobre los volúmenes:

- **El directorio del host debe existir y pertenecer a UID 1000.** Docker crea
  el directorio faltante como root y entonces la app no puede escribir. Créalo
  antes con `install -d -o 1000 -g 1000 /srv/shirley/data`.
- **El token de Colab tiene que ser un archivo, no un directorio.** Si
  `/srv/shirley/secrets/colab-token.json` no existe, Docker crea un *directorio*
  en esa ruta y el contenedor arranca con un token inservible; el fallo aparece
  recién al separar voces, con `colab_token_missing`.
- **El token se monta sin `:ro`.** `colab` reescribe ese archivo cuando refresca
  el `access_token` caducado. Con un montaje de solo lectura, la separación
  funciona una hora y después falla. El archivo del host debe pertenecer a UID
  1000 para que el contenedor pueda escribir en él.

## 3. Variables de entorno

El panel escribe `/srv/panel/projects/<id>.env` (modo 0600) y lo pasa con
`--env-file`. Lista completa en `.env.example`. Las que importan:

| Variable | Ejemplo | Efecto si falta |
|---|---|---|
| `OPENCODE_API_KEY` | key de OpenCode Go (plan Go) | Ressy responde "no configurado"; el resto funciona |
| `OPENCODE_MODEL` | `deepseek-v4.1-flash` | usa el valor por defecto: el modelo mas barato que completa un flujo de varias herramientas, y con el tope mensual mas alto del plan |
| `OPENCODE_BASE_URL` | `https://opencode.ai/inference/go/openai/v1` | usa el valor por defecto |
| `SHIRLEY_PUBLIC_BASE_URL` | `https://shirley.example.com` | **el enlace del ticket sale relativo** y el comprador no puede abrirlo |
| `SHIRLEY_COLAB_TOKEN` | `/home/shirley/.config/colab/token.json` | ya viene del Dockerfile; cámbialo solo si montas el token en otra ruta |
| `SHIRLEY_COLAB_GPU` | `g4` | usa el valor por defecto (RTX PRO 6000, 102 GB) |
| `SHIRLEY_COLAB_OVERLAP` | `12` | usa el valor por defecto; bajarlo acelera y baja calidad |
| `SHIRLEY_COLAB_TIMEOUT_SECS` | `1800` | usa el valor por defecto |
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
  -v /srv/shirley/secrets/colab-token.json:/home/shirley/.config/colab/token.json \
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
- Que la separación vocal remota funcione: depende del token de Colab y de que
  la cuota de la cuenta siga disponible.
- Que los datos sobrevivan: haz un segundo despliegue y comprueba que los shows
  siguen ahí. Un dato escrito dentro del contenedor se pierde.

## 8. Token de Colab

La separación vocal enciende una GPU en Google Colab, la usa y la apaga. El
contenedor solo necesita un token OAuth válido; el tráfico sale por la red
normal del host.

**De dónde sale el token.** En la máquina donde ejecutaste `colab login` una vez,
el CLI guarda el token en `~/.config/colab/token.json`:

```sh
# en tu máquina
colab quota        # si responde, el token sirve
ls -l ~/.config/colab/token.json
```

**Cómo llevarlo al servidor.** Cópialo y dale el propietario del contenedor:

```sh
# en el servidor
install -d -m 700 /srv/shirley/secrets
scp tu-usuario@tu-maquina:~/.config/colab/token.json /tmp/colab-token.json
install -o 1000 -g 1000 -m 600 /tmp/colab-token.json /srv/shirley/secrets/colab-token.json
rm /tmp/colab-token.json
chown 1000:1000 /srv/shirley/secrets/colab-token.json
```

**Cómo comprobarlo desde el contenedor:**

```sh
docker exec -it panel-app-shir colab quota
```

Si responde con tu cuota, la separación funcionará. Si dice que no hay token,
revisa la ruta: el contenedor lo busca en `/home/shirley/.config/colab/token.json`
y el usuario del contenedor es `shirley` (UID 1000).

**El token caduca y se renueva solo.** `access_token` dura una hora; `colab`
lo refresca con el `refresh_token` y reescribe el archivo. Por eso el montaje es
de lectura-escritura. Si el archivo del host es de `root`, el refresco falla con
un error de permisos que la app reporta como `colab_unavailable`.

## 9. Dónde se construye la imagen

**No en el servidor.** El VPS tiene 1,9 GB de RAM; un `cargo build --release` ahí
puede agotar la memoria y arrastrar a los otros contenedores. Por eso el TOML
declara `image` en lugar de `build`: la imagen se construye fuera y se carga en
el demonio del servidor.

### Entorno de build

Se usa una caja de instabox con la plantilla `universal` (trae git, Node, Python,
Go, Rust, toolchain C y Docker). Dos añadidos que no vienen de fábrica:

```sh
# ffmpeg: la plantilla no lo trae y lo necesita un test de MP3
sudo apt-get update && sudo apt-get install -y ffmpeg

# dependencias de test del frontend (pnpm es el único gestor permitido)
pnpm install --frozen-lockfile
```

Sin `ffmpeg`, `test_encode_mp3_320k_success` falla con `IoError(NotFound)`. Sin
`pnpm install`, `test_frontend_node_unit_tests` no encuentra `qrcode`.

### Despliegue, paso a paso

```sh
# 1. copiar el árbol desde un checkout limpio, sin build, sin dependencias ni datos.
#    git archive parte de lo que git rastrea, así que no puede arrastrar
#    target/, node_modules/ ni la base de datos. Preferirlo a un tar con
#    --exclude escritos a mano.
#
#    Cuidado con la semántica de los patrones, porque no coincide entre
#    herramientas y un mismo texto hace cosas distintas:
#      tar   --exclude='data'    -> casa a CUALQUIER profundidad: borra
#                                   frontend/js/data/ (esto causó la página en
#                                   blanco del commit 3414a23)
#      tar   --exclude='./data'  -> solo la raíz
#      rsync --exclude='data/'   -> casa a cualquier profundidad (mismo peligro)
#      rsync --exclude='/data/'  -> solo la raíz
#      docker .dockerignore data/ -> solo la raíz (Docker casa la ruta completa)
#      git   .gitignore    data/  -> cualquier profundidad; oculta módulos nuevos
#    Por eso el paso 1b no es opcional: comprueba el resultado, no la intención.
git archive HEAD | tar x -C /tmp/build-src
instabox ssh <caja> -- 'rm -rf /data/workspace/build && mkdir -p /data/workspace/build'
rsync -a --exclude='/target/' --exclude='/node_modules/' --exclude='/.git/' --exclude='/.env' \
  /tmp/build-src/ <caja>:/data/workspace/build/ 2>/dev/null \
  || tar czf - -C /tmp/build-src . | instabox ssh <caja> -- 'tar xzf - -C /data/workspace/build'

# 1b. comprobar el grafo de módulos ANTES de construir. El builder también lo
#     corre, pero verlo aquí evita esperar 10 minutos para descubrirlo.
instabox ssh <caja> -- 'cd /data/workspace/build && sh scripts/verify-frontend-graph.sh frontend'

# 2. construir en la caja
instabox ssh <caja> -- 'cd /data/workspace/build && docker build -t shirley-show:<commit> .'

# 3. transferir al servidor.
#    El image ID cambia al cruzar daemons y NO es corrupción: Docker recalcula
#    el ID sobre el manifiesto del daemon de destino, y la versión del motor
#    afecta a ese manifiesto. Un mismo artefacto puede aparecer como 948 MB en
#    un daemon y 665 MB en otro. Comparar IDs entre daemons es un falso
#    positivo de integridad; comparar contenido:
#      docker exec panel-app-shir sha256sum /app/shirley-show
#      docker exec panel-app-shir sh -c 'cd /app/frontend && find . -type f \
#        | LC_ALL=C sort | xargs sha256sum | sha256sum'
#    Ambos hashes deben coincidir con los de la imagen en la caja.
instabox ssh <caja> -- 'docker save shirley-show:<commit> | gzip -1' \
  | ssh -i ~/.ssh/id_elastika root@<vps> 'gunzip | docker load'

# 3b. confirmar el contenido EN EL DAEMON DE DESTINO antes de tocar producción.
#     Si esto no da 14, abortar: producción sigue con la imagen anterior.
ssh -i ~/.ssh/id_elastika root@<vps> \
  'docker run --rm --entrypoint sh shirley-show:<commit> -c "ls /app/frontend/js/data | wc -l"'

# 4. recrear el contenedor con la misma configuración que usa el panel
ssh -i ~/.ssh/id_elastika root@<vps> '
  docker rm -f panel-app-shir
  docker run -d --name panel-app-shir \
    --label panel.project=app-shir \
    --restart unless-stopped \
    --memory 768m --memory-swap 1536m \
    --env-file /srv/panel/projects/app-shir.env \
    -v /srv/shirley/data:/app/data \
    -v /srv/shirley/secrets/colab-token.json:/home/shirley/.config/colab/token.json \
    -p 127.0.0.1:65529:3001 \
    shirley-show:<commit>'

# 5. verificar de verdad.
#    El 200 de "/" no prueba nada: index.html es estático y una web en blanco
#    también devuelve 200. El healthcheck tampoco sirve, porque solo consulta
#    /api/presentations. Hay que comprobar el contenido del grafo y abrir la
#    aplicación en un navegador.
docker exec panel-app-shir ls /app/frontend/js/data | wc -l        # 14
curl -s -o /dev/null -w '%{http_code}\n' \
  http://127.0.0.1:65529/assets/js/data/list-presentations.js      # 200
curl -sS http://127.0.0.1:65529/api/agent/gpu-status   # {"mode":"remote","pc_online":true}
docker exec panel-app-shir colab quota
```

Nota sobre el runtime: la imagen final solo contiene `data`, `frontend` y el
binario. `scripts/verify-frontend-graph.sh` corre en el stage de build y no se
copia a `/app`, así que `/app/scripts` no existe dentro del contenedor. Es
correcto; no hay que buscarlo ahí.

Actualiza después `image` en el TOML; si no, la interfaz del panel muestra una
imagen distinta de la que está corriendo.

### Revertir

Antes de recrear, guarda el TOML y la configuración:

```sh
cp /srv/panel/projects/app-shir.toml /root/app-shir.toml.bak-$(date +%F-%H%M)
docker inspect panel-app-shir > /root/panel-app-shir.backup-$(date +%F-%H%M).json
```

Revertir es cargar la imagen anterior y repetir el paso 4 con ese tag.
