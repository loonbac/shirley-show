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
#    El comando importa: rutas ABSOLUTAS y RELATIVAS producen hashes distintos
#    sobre la misma imagen (la ruta entra en el hash). Usar siempre esta forma
#    relativa, que no depende del prefijo /app, y no mezclar métodos entre
#    sesiones: hacerlo crea un falso positivo de divergencia.
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

# 4. redesplegar.
#    Usar la API del panel, no "docker rm -f" y luego "docker run". Ese orden
#    destruye producción antes de crear el reemplazo: si el run falla (imagen
#    ausente, env_file ilegible, puerto ocupado, límite de memoria) el servicio
#    queda caído. El panel hace un swap en tres fases y nunca borra primero:
#      a) arranca el reemplazo al lado, como panel-app-shir-next-<pid>
#      b) si falla, retira el reemplazo y REARRANCA el anterior
#      c) solo si el reemplazo está en pie, borra el viejo y lo renombra
#    Ver src/docker.rs en koro-panel. Es la ruta que usan los despliegues reales.
#
#    La URL no es loopback ni el puerto por defecto. El panel se ata a la IP de
#    Tailscale y al puerto del unit, no a 127.0.0.1: comprobado en el VPS,
#    koro-panel escucha en 100.82.97.118:9090 (PANEL_BIND + PANEL_PORT en
#    /etc/koro-panel.env y el unit). Y 127.0.0.1:9090 rehúsa la conexión,
#    porque el panel no escucha en loopback.
#
#    Cuidado con 127.0.0.1:3000: NO es el panel. Lo publica el contenedor
#    panel-korosoft-lead, la landing de korosoft.site. Apuntar ahí no falla con
#    "connection refused" y ahí está el peligro:
#      POST /api/v1/projects/app-shir/acciones/up  -> 405  (Allow: GET, HEAD)
#      GET  /api/v1/projects/app-shir/acciones/up  -> 200  (fallback HTML)
#    La landing sirve su SPA para cualquier ruta, así que un GET de
#    "verificación" devuelve 200 y da el paso por bueno mientras se golpea otro
#    sitio en producción.
#
#    Ni GET ni -f resuelven esto: -f solo convierte 4xx/5xx en fallo, y ante el
#    200 de la landing devuelve exit 0 como si todo fuera bien. Y usar el POST
#    que despliega como "verificación" confunde la prueba con la acción: un
#    verde no distingue "el panel respondió" de "el panel redesplegó".
#
#    Una sonda no debe tener efectos. La prueba que discrimina es el
#    Content-Type sobre una ruta de LECTURA, sin token y sin efectos:
#      curl -sS -o /dev/null -w '%{content_type}\n' "$PANEL_URL/api/v1/projects"
#    El panel responde application/json (401 sin token); la landing responde
#    text/html con 200. Medido: el panel nunca sirve HTML bajo /api/v1.
#    Si imprime text/html, la URL es otro servicio: no desplegar.
#    El POST de /acciones/up va después, y es la operación, no la verificación.
#
#    Que un puerto lo "posea" dockerd en ss -ltnp no dice qué lo sirve: dockerd
#    figura como dueño de todo puerto publicado por un contenedor. Cruzar con
#    docker ps --format '{{.Names}} {{.Ports}}' y con el contenido servido.
PANEL_URL=http://100.82.97.118:9090   # PANEL_BIND + PANEL_PORT del unit; loopback cerrado
#    -f hace que un 4xx salga como exit != 0 en vez de pasar silenciosamente.
curl -sSf -X POST "$PANEL_URL/api/v1/projects/app-shir/acciones/up" \
  -H 'Authorization: Bearer <token>' | tee /dev/stderr | grep -q '"ok":true'

#    Si hay que hacerlo a mano sin el panel, reproducir las tres fases. El
#    "||" y el "&&" tienen que cortar el script de verdad, o el rename borra
#    producción aunque el reemplazo no haya arrancado:
#      set -e
#      NEXT=panel-app-shir-next
#      docker rm -f "$NEXT" 2>/dev/null || true
#      docker run -d --name "$NEXT" --label panel.project=app-shir \
#        --restart unless-stopped --memory 768m \
#        --env-file /srv/panel/projects/app-shir.env \
#        -v /srv/shirley/data:/app/data \
#        -v /srv/shirley/secrets/colab-token.json:/home/shirley/.config/colab/token.json \
#        -p 127.0.0.1:65529:3001 shirley-show:<commit>
#      # Solo se llega aquí si el run anterior tuvo éxito (set -e).
#      docker rm -f panel-app-shir
#      docker rename "$NEXT" panel-app-shir
#
#    Sobre la memoria: pasar solo --memory. Docker deriva --memory-swap al doble
#    por defecto, igual que hace el panel. Escribir los dos a mano deja el swap
#    obsoleto en silencio si alguien cambia "memory" en el TOML. Verificado en el
#    contenedor vivo: Memory=805306368, MemorySwap=1610612736.

# 5. verificar de verdad.
#    El 200 de "/" no prueba nada: index.html es estático y una web en blanco
#    también devuelve 200. El healthcheck tampoco sirve, porque solo consulta
#    /api/presentations. Hay que comprobar el contenido del grafo y abrir la
#    aplicación en un navegador.
#
#    Ojo con cómo se audita la red. Los listeners de Playwright
#    (page.on('request'), context.on('response')) NO son la lista completa de
#    lo que pide el navegador: filtran peticiones internas. Medido sobre el
#    mismo render, buscando /favicon.ico:
#      page.on('request') / context.on('request')     -> 136, favicon NO
#      page.on('response') / context.on('response')   -> 136, favicon NO
#      CDP Network.requestWillBeSent                  -> 137, favicon SÍ
#      CDP Network.responseReceived                   -> 137, "404 .../favicon.ico"
#      page.on('console') con location.url            -> el error del 404
#    Para auditar de verdad hay que usar CDP (context.newCDPSession + Network.enable).
#    Y un error de consola con location.url es evidencia de una petición aunque no
#    haya evento de red de alto nivel asociado.
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

## 10. Guardas dentro del contenedor: tres reglas

Las guardas de despliegue comprueban que la imagen es la correcta. Tres detalles las
rompen en silencio, y los tres se descubrieron midiendo, no leyendo.

### 10.1 Un binario ausente se parece a un fallo

La imagen de runtime **no trae `strings`**. Un `docker exec … strings` devuelve
`sh: 1: strings: not found` — y ese silencio es indistinguible de un símbolo
ausente, que es justo lo que la guarda existe para detectar.

Comprobar antes de usar, o usar la alternativa que sí está:

```sh
docker exec panel-app-shir sh -c 'command -v strings || echo AUSENTE'
docker exec panel-app-shir sh -c 'grep -ac transfer_failed /app/shirley-show'
```

Presentes en la imagen: `grep`, `tr`, `od`, `awk`, `sed`, `curl`, `colab`,
`ffprobe`, `openssl`, `ffmpeg`. Ausentes: `strings`, `wget`.

### 10.2 La guarda usa un literal, no un nombre de módulo

Los nombres de módulo y de tipo **no sobreviven al binario**: los elimina el
compilador o los ofusca el enlazador. Los literales de cadena sí, porque están en
los datos del ejecutable.

```sh
# NO sirve: grep -ac tool_routing /app/shirley-show        -> 0
# NO sirve: grep -ac ExistingTrack /app/shirley-show       -> 0
# SI sirve: un literal del código
docker exec panel-app-shir sh -c \
  'grep -ac "El audio esta adjunto en esta solicitud" /app/shirley-show'
```

### 10.3 `grep -c` devuelve exit 1 al contar cero

Con `set -e` el script muere **sin decir por qué**: cero coincidencias es un
fallo de `grep`, no un error de la guarda. Hay que absorberlo explícitamente:

```sh
set -e
count=$(docker exec panel-app-shir sh -c \
  'grep -ac transfer_failed /app/shirley-show' || true)
if [ "${count:-0}" -lt 1 ]; then
  echo "FALLO: la imagen no contiene el literal esperado"; exit 1
fi
```

### 10.4 Comprobar el apagado de la GPU

`colab status` **no es prueba de nada por sí solo**. Dos formas de leerlo mal:

- El campo `Idle` puede pertenecer a una sesión **anterior**: ver
  `Connected: true` con horas de inactividad no significa que la GPU de este turno
  siga viva.
- Una GPU recién arrancada **sin trabajo** la recoge Colab sola, así que responde
  `No active runtime` aunque nadie la haya apagado.

La evidencia válida es el log del release con su generación, o un `colab exec`
que funciona antes del `stop` y falla después:

```sh
colab exec --session <id> -c 'print("VIVA")'   # funciona si está viva
colab stop
colab exec --session <id> -c 'print("VIVA")'   # "no active runtime with endpoint"
```

Con `SHIRLEY_COLAB_DEBUG_OUTPUT=1` el apagado diferido se registra:

```text
[release] generation 1 owns=true
[release] stop gpu-g4-... ok=true
```

## 11. Apagado de la GPU: quién la apaga y cuándo

La GPU de Colab cuesta mientras existe. Hay tres mecanismos, y conviene saber
cuál actúa en cada caso porque el log los distingue.

### 11.1 Liberación al terminar, con ventana de reutilización

Al acabar una separación la máquina **no** se apaga: se queda 180 s para que una
segunda petición cercana la reutilice. Una tarea diferida la apaga al vencer la
ventana. Verificado en producción: `Connected: true` durante 120 s,
`No active runtime` a los 150 s.

### 11.2 Barrido al arrancar

Si el proceso muere por SIGKILL, OOM o reinicio duro, **ningún destructor corre**
(se muere con el proceso) y la GPU seguiría facturando hasta que Colab la expira.
El proceso nuevo es el primero capaz de limpiarlo, y al arrancar no hay ningún
turno en curso:

```text
Barrido de GPU al arrancar: runtimes huérfanos liberados=true
```

### 11.3 El barrido respeta un arrendamiento

`colab stop` **sin `--session` libera todos los runtimes de la cuenta**, no solo
los propios. Un barrido ciego mataría el trabajo de otra instancia viva.

Por eso el arranque consulta `<data>/colab-runtime-owner.pid`:

| pid del arrendamiento | significado | acción |
| --- | --- | --- |
| no existe | nadie reclamó | barrer |
| pid muerto | lo dejó un proceso que ya no está | barrer |
| pid vivo y no somos nosotros | otra instancia trabaja | **omitir** |
| pid vivo y somos nosotros | es nuestro propio arranque | barrer |

Al omitir, el log dice:

```text
Barrido de GPU al arrancar: omitido, otra instancia sigue viva
```

**Eso es un resultado correcto, no un fallo.** Significa que el arrendamiento
está protegiendo trabajo ajeno.

### 11.4 Límites conocidos

- **Dentro de un contenedor el pid propio es siempre 1**, así que un contenedor
  nuevo ve `pid == self` y reclama el arrendamiento. Es lo correcto para
  reemplazar al anterior, que es el caso de un redespliegue, pero **no distingue
  dos contenedores en máquinas distintas**.
- **No hay vigilante periódico.** Existió uno cada 60 s y se retiró: apagaba GPUs
  ajenas a mitad de vuelo, y como todas las instancias comparten cuenta de Colab,
  también las propias. Su rastro era invisible porque solo registraba con
  `SHIRLEY_COLAB_DEBUG_OUTPUT`, que en producción no está definida.
- Si un release falla en silencio y no hay redespliegue, la GPU vive hasta que
  Colab la reclama por inactividad. Es el hueco que queda, y es más barato que
  apagar el trabajo de otro.

### 11.5 Cómo comprobarlo sin engañarse

`colab status` **no basta**: el `Idle` puede ser de una sesión anterior y una GPU
recién arrancada sin trabajo la recoge Colab sola. La prueba real es preguntar por
la sesión concreta:

```sh
colab exec --session <id> -c 'print("VIVA")'   # funciona si está viva
colab stop
colab exec --session <id> -c 'print("VIVA")'   # "no active runtime with endpoint"
```
---

## 12. Verificar un despliegue sin confundir estados viejos

Cuatro veces en una sola sesión, el agente que despliega reportó una imagen y el
agente que verifica midió otra. Ninguna de las dos partes se equivocó al medir;
se midieron **momentos distintos**.

### La regla que no basta

«Un pedido, un deploy; gana el último mensaje» ordena los pedidos, pero no dice
nada del estado. El problema real es que se verifica un estado que ya cambió.

### La que sí

Al leer un despliegue, no basta la etiqueta de la imagen. Hay que leer también el
arranque:

```bash
docker inspect panel-app-shir --format 'imagen={{.Config.Image}} arranque={{.State.StartedAt}}'
```

Si el `StartedAt` es **posterior** a tu propio pedido, la imagen que estás mirando
ya es la que pediste y el reporte que esperabas se produjo. Comprobar la etiqueta
sola deja los dos casos indistinguibles.

```
imagen=shirley-show:14290bf arranque=2026-11-05T14:11:15.59558965Z
estado=Up 11 minutes (healthy)
```

### Discriminadores que no funcionan

Tres instrumentos mintieron en esa sesión, siempre en la misma dirección: daban
un resultado limpio sobre un sistema sano.

| Instrumento | Qué dio | Realidad |
| --- | --- | --- |
| `docker exec … sh -c "grep …"` con acentos | `0` | el `ó` se rompe entre capas de comillas |
| `strings` sobre el binario, buscando un campo serde | `0` | el nombre del campo no es una cadena garantizada |
| Copiar solo `shirley-show.sqlite3` | tabla vacía | falta el `-wal`: en modo WAL los datos recientes viven ahí |

Para acentos: descargar por HTTP o usar Python, nunca `grep` anidado en
`docker exec`.

Para campos de un DTO: el JSON real del endpoint o el fichero fuente. Nunca las
cadenas del binario.

Para un snapshot de base en WAL: copiar los tres ficheros (`.sqlite3`, `-wal`,
`-shm`) o usar `VACUUM INTO`.

### La señal de alarma

Las tres trampas produjeron una respuesta **demasiado limpia**: `0`, `404`, tabla
vacía. Cuando el instrumento da un resultado perfectamente nítido sobre algo que
debería tener matices, hay que sospechar del instrumento antes que del sistema.

### Invariantes: siempre con su comando

Un invariante sin unidad no se puede verificar: dos personas pueden medir cosas
distintas y ambas tener razón.

```
mal:   js/data = 14
bien:  ls /app/frontend/js/data | wc -l = 14
```

En esa sesión, `js/data = 14` se midió como ficheros en disco (14, correcto) y
como etiquetas `<script src=` en el HTML servido (0, también correcto: la landing
no carga ningún JS externo, su script es inline).

### Una medición de layout sin el CSS aplicado no mide nada

Quinta trampa de la misma familia, y la más silenciosa: un número de layout
parece una medición y puede ser el resultado de los estilos por defecto del
navegador.

En Playwright, `page.setContent()` deja el documento en `about:blank`, así que
**un `<link href="file://…">` no carga**. El `getBoundingClientRect()` devuelve
entonces cifras medidas con el CSS del navegador, no con el del proyecto. La
conclusión puede incluso ser correcta, con evidencia inválida: el peor caso.

Y los números discrepan por una razón concreta: `.text-input` tiene
`width: 100%` **más `padding` y `border`**. Sin el `box-sizing: border-box` de
`base/reset.css`, el input desborda su envoltorio; con él, lo contiene. El mismo
markup da 314, 337, 363 o 364 según qué hojas se hayan cargado.

**La comprobación que invalida o valida la medición**, antes de creerla:

```js
const g = getComputedStyle(el).boxSizing;   // 'border-box' sólo con el reset
document.styleSheets.length;                // 0 con setContent + file://
```

Si el valor que se lee es el del navegador por defecto, no se aplicó nada.

**Cómo cargar el CSS real**: leer `frontend/styles/index.css` y resolver sus
`@import` a CSS plano —así entra el reset—, o navegar a la página servida. Un
`<link>` a `file://` desde `setContent` nunca funciona.

**Segunda comprobación, y es la que faltaba**: que el elemento medido sea el que
crees, y que el contenedor esté completo.

Y una tercera, que salió de cuatro mediciones fallidas del mismo atributo: **el
contenedor no es solo el padre directo.** Añadir o quitar el `<section>` que
envuelve la fila cambia el ancho del input:

```
con .events-types-section     fila 364  envoltorio 294  input 294
sin la seccion                fila 390  envoltorio 320  input 320
```

Los dos son «el mismo componente» y miden distinto por 26 px. Un número sin el
árbol completo que lo rodea no es comparable con otro número, ni siquiera propio.

Y una cuarta comprobación, que salió de la anterior: **copiar el markup del código
fuente, no escribirlo de memoria.** Las clases de un átomo traen propiedades
invisibles en el nombre del componente. Medido en la misma fila:

```
botón sin clases (inventado)      input 314
botón real (btn btn-ghost btn-sm) input 294
```

La diferencia son los 20 px de `min-width: var(--touch-target-min)` que `.btn`
declara (`atoms/button.css:7`), y en una columna `auto` son 20 px menos para las
otras dos. **Un componente inventado mide distinto que el componente que existe.**

Y hay un tercer escenario que nadie había medido, porque no se ve en una captura:
`forms.js` termina la fila con `.filter(Boolean)`, así que **con un solo tipo de
entrada el botón no se renderiza**:

```
2+ tipos  4 hijos  botón 44px  input 294
1 tipo    3 hijos  sin botón   input 338
```

44 px exactos, que es el `min-width` del botón. La app real pasa la mayor parte del
tiempo en el segundo caso. **El estado por defecto también es un escenario.**

Y es el escenario que **el usuario ve al crear un evento**, porque el formulario
nace con un solo tipo (`forms.js:126-128`, `[{nombre:'General',…}]`). Medido con la
sección completa —etiqueta, lista y botón de añadir— y con la hoja aplicada
(`border-box`, `styleSheets.length` = 1):

```
1 tipo  (DEFECTO)  3 hijos  input 338  botón 0
2+ tipos           4 hijos  input 294  botón 44
```

**Los siete números de la cadena se midieron con el botón presente**, es decir, con
el caso que no es el de partida.


Un `querySelector` con un nombre que suena bien puede devolver otro nodo, o
`null`. Y medir un solo hijo de una rejilla mide una rejilla distinta:

```
medido con la fila completa (nombre, precio, capacidad, botón)
  fila 390 · envoltorio 320 · input 320 · precio 156 · capacidad 156 · sin desborde

medido con el campo de nombre solo
  input 364 · envoltorio 364
```

El segundo número no es el input en su sitio: es el ancho que ocupa el campo
cuando es el único hijo. **Un número que coincide exactamente con otro elemento
del mismo contenedor suele ser el elemento equivocado.** Si el input mide lo mismo
que la fila, se midió la fila.

**Regla general**: un número de layout sin `box-sizing` documentado no significa
nada. Citar una comparación —«idéntico con y sin la línea»— sobrevive a que el
número absoluto sea irreproducible; citar el píxel no.

#### El estado es la quinta dimensión

Las cuatro comprobaciones cubren hoja, elemento, ancestros y markup. **Ninguna
cubre el estado del componente.** Y el sesgo es el mismo en las dos direcciones:
los probes se construyen con el caso más rico —con el botón, con los hermanos, con
el ancestro— porque es el que hace lucir la medición, y el que se usa es el más
pobre. Es el mismo sesgo que verificar con datos de prueba que no se parecen a los
reales.

Un número necesita, para significar algo:

```
la hoja que lo aplica
el elemento correcto
los hermanos presentes
los ancestros completos
el estado del componente que lo produjo
```

### Verificar una lectura no se hace con una escritura

`POST /api/entradas/checkin` **marca la entrada**. Comprobar con esa llamada que
un código se puede leer gasta la entrada en producción. Para verificar lecturas:
`GET`, SQL, o un contenedor aislado con copia de la base.

Y si se prueba una venta o un evento, **nunca contra los datos del dueño**:

```bash
docker run -v /tmp/prueba-data:/app/data …    # directorio vacío
```

Un solo `POST /api/eventos` contra la base real crea un evento que hay que borrar
después, y el borrado es una operación sobre datos ajenos.

### Borrar datos: el paso que el resto del runbook no cubre

Todo lo anterior enseña a **medir**. Lo único que este runbook deja pendiente y
que **borra** es la limpieza de datos de prueba, así que merece su propia regla:
un runbook que enseña a verificar y calla sobre la operación destructiva deja el
paso peligroso sin cubrir.

#### Las guardas que protegen contra borrar de más

Seleccionar por **nombre exacto** (nunca `LIKE '%prueba%'`: borraría a un
comprador real que se llame así más adelante), abortar si el número de filas no
es el esperado, y verificar las entradas reales antes y después.

#### La guarda que protege contra borrar algo que ya hizo su trabajo

Las anteriores cubren el riesgo de borrar **de más**. Falta el contrario, y es el
único cuya pérdida es irreversible: borrar algo que **ya produjo un efecto**.

```sql
SELECT nombre_comprador, estado_checkin FROM entradas
 WHERE evento_id = ? AND nombre_comprador IN (?, …) AND estado_checkin != 'No ingresó';
```

Si devuelve una sola fila, **no se borra nada**: esa entrada registró un ingreso
real y el `DELETE` lo destruye sin vuelta atrás. Una entrada de prueba sin usar
se puede recrear; un ingreso registrado, no.

#### Y la que no depende de la lista estar completa

La consulta anterior filtra por los nombres que ya crees conocer. Si una entrada
de prueba tiene otro nombre —un typo, una creada después— pasa el filtro y se
borra igual, aunque haya ingresado a alguien.

La condición que no admite listas incompletas es la complementaria: **ninguna
entrada ingresada del evento puede desaparecer con ese `DELETE`**.

```sql
SELECT nombre_comprador, estado_checkin FROM entradas
 WHERE evento_id = ? AND estado_checkin != 'No ingresó';
```

Quédate con la segunda: una lista protege contra borrar de más **si está
completa**; esta protege aunque no lo esté.

#### Y la que no depende de la etiqueta, sino del hecho

`estado_checkin` es el estado **declarado**. El esquema guarda además el hecho:

```sql
CHECK (
  (estado_checkin = 'No ingresó' AND hora_ingreso IS NULL)
  OR
  (estado_checkin = 'Ingresó' AND hora_ingreso IS NOT NULL)
)
```

Hoy las dos fuentes coinciden por construcción, así que cualquiera de las dos
detecta un ingreso. Pero si una migración futura relajara ese `CHECK`, una fila
podría quedar con `estado_checkin = 'No ingresó'` y `hora_ingreso` puesta: la
guarda por estado la dejaría pasar, y borrarla destruiría un ingreso real.

Comprueba las dos, con `OR`:

```sql
SELECT nombre_comprador, estado_checkin, hora_ingreso FROM entradas
 WHERE evento_id = ? AND (estado_checkin != 'No ingresó' OR hora_ingreso IS NOT NULL);
```

La etiqueta puede estar mal escrita o desactualizada; la hora de ingreso, si
existe, es porque alguien entró.

#### Antes de ejecutar

- `SELECT` que cuenta las filas del evento antes y después.
- Una FK ausente no se supone: se comprueba con `PRAGMA foreign_key_list` sobre
  todas las tablas, para saber si el borrado deja huérfanos.
- `DRY_RUN=1` por defecto, `DRY_RUN=0` para ejecutar. Contra la base real, la
  primera ejecución es siempre en seco.
- Probarlo antes contra una base sintética **con el caso peligroso**, no solo con
  el normal: el normal ya se sabe que funciona; el peligroso hay que verlo fallar.

#### La regla general

Antes de borrar, no basta preguntarse «¿es la fila que quiero?». Hay que
preguntarse además «¿ha producido ya un efecto que no puedo reconstruir?», y
«¿depende mi lista de que esté completa?».

#### Procedimiento del borrado: por tubería, sin copias

Una copia en `/tmp` no tiene dueño ni historial, y una copia vieja es **más
peligrosa que ninguna**: parece el script y no tiene las guardas. La solución no
es nombrar mejor la copia, es **no crear copia**.

`docker cp` no acepta stdin (responde «destination must be a directory»), pero
`python3 -` sí. El script entra completo por tubería:

```bash
# En seco, siempre, sobre el contenido exacto del commit
git show <commit>:scripts/purge-test-entries.py \
  | ssh root@host 'docker exec -i panel-app-shir python3 -'

# Y solo si el seco dijo lo esperado
git show <commit>:scripts/purge-test-entries.py \
  | ssh root@host 'docker exec -i -e DRY_RUN=0 panel-app-shir python3 -'
```

`DRY_RUN` va por entorno (`-e`), igual que antes. No hay fichero intermedio, así
que no hay nada que limpiar después.

Y tiene una propiedad que ninguna copia tiene: **el contenido y la procedencia
viajan juntos en la misma tubería.** No hay ventana entre «copié» y «ejecuté»
donde el fichero pueda cambiar. Esa ventana es justo lo que produjo las citas
desfasadas y las limpiezas incompletas de esta sesión.

**Un límite de una herramienta no es un límite del problema.** Que `docker cp` no
lea stdin es cierto; que no haya forma de alimentar al contenedor sin escribir un
fichero, no. Antes de aceptar un compromiso, probar la alternativa obvia: aquí,
`python3 -` o una redirección.

#### El acta, con dos hashes y tres sitios

```bash
git show <commit>:scripts/purge-test-entries.py | md5sum   # anotar
```

```
fuente   scripts/purge-test-entries.py @ <commit>
md5      921b2ead…  (remedido del commit, no de una copia)
copias   ls /tmp/*.py en LOCAL y VPS HOST -> 0 (el contenedor no recibe fichero)
antes    N filas · M ingresadas (estado u hora)
en seco  salida esperada
después  N' filas · las dos reales confirmadas
borradas nombre, estado previo, hora_ingreso
```

**El commit responde «¿de dónde salió?»; el md5 responde «¿qué se ejecutó?».** Un
commit envejece cuando la rama avanza; un hash de contenido no. Sólo con el commit,
el acta caduca en minutos. Sólo con el hash, se pierde la procedencia.

**«Host» es ambiguo**: puede ser la máquina donde hablas o el host del VPS fuera
del contenedor. Son dos sistemas de ficheros, y una copia vivió en el segundo
mientras el primero se declaraba limpio. Se nombran los sitios.

**Y se enumeran, no se recuerdan**: `ls /tmp/*.py`, no `rm` de nombres conocidos.
`rm -f` de algo inexistente devuelve 0 y refuerza la falsa sensación de limpieza.


#### Contar sentencias no es contar texto

En el mismo fichero, `grep -c 'raise SystemExit'` daba **5** y hay **4**: el quinto
era la frase de un comentario que explicaba el recuento. Para contar sentencias,
el AST:

```bash
python3 -c "import ast,sys; print(len([n for n in ast.walk(ast.parse(open(sys.argv[1]).read())) if isinstance(n, ast.Raise)]))" fichero.py
```

Y el error simétrico: comparar dos módulos enteros para probar «solo cambió la
cabecera» da distinto **porque el docstring es un nodo**. Ahí lo correcto es
excluirlo y comparar el resto. **Las dos son la misma falta: medir el artefacto
entero cuando importa una parte, o medir el texto cuando importa el código.**

#### Un commit envejece; el contenido no

Medido en esta rama, el mismo fichero a través de cuatro commits:

```
7cf5932  921b2ead1ebfb8f2e5e0cb5b6583a3b7
5fd4501  921b2ead1ebfb8f2e5e0cb5b6583a3b7
9d1784e  921b2ead1ebfb8f2e5e0cb5b6583a3b7
911ddc7  921b2ead1ebfb8f2e5e0cb5b6583a3b7
```

`HEAD` cambió cuatro veces; el contenido, ninguna.


#### Medir con el método bueno, no con el que está a mano

Una consulta con un literal acentuado no significa lo mismo en todas las capas por
las que pasa. Medido sobre la base real:

```
literal correcto    "No ingresó"   ->  0   (correcto: ninguna ingresada)
literal sin tilde   "No ingreso"   ->  8   (TODAS figuran ingresadas)
literal mojibake    "No ingresÃ³"   ->  8   (TODAS figuran ingresadas)
```

**Un `!=` con el literal corrupto no falla con un error: devuelve todas las filas.**
Y la dirección del engaño es la peor posible, porque se lee como «todo el mundo ya
entró». Un informe hecho así dice lo contrario de la verdad con aspecto de dato.

**Y hay dos causas distintas que producen el mismo síntoma.** Distinguirlas importa,
porque se arreglan de forma diferente:

- **acentos en el canal**: el literal se corrompe al atravesar capas. Se arregla
  leyendo por HTTP o por Python, sin pasar el texto por `sh -c`.
- **escape en el emisor**: el literal llega intacto pero **ya escapado**.
  `json.dumps` con su `ensure_ascii=True` por defecto convierte `ó` en la secuencia
  de seis caracteres `\u00f3`, y SQLite **no interpreta `\u` en literales**: compara
  contra esos seis caracteres, que nunca igualan al carácter real.

Medido sobre la base real, aislando las dos:

```
literal escapado (\u00f3 como 6 chars)  hex 4E6F...5C7530306633   != -> 8   (todas)
dato almacenado                          hex 4E6F...73C3B3         == real
literal real                             hex 4E6F...73C3B3         != -> 0   (correcto)
SQLite interpreta \u en un literal?                                   0   (no)
```

El síntoma es idéntico al de la corrupción de canal; la causa no. **Documentar sólo
una de las dos habría dejado la otra circulando como «la trampa de acentos».**

Y una advertencia sobre el control: si la comprobación se construye **por la misma
vía** que el defecto, comparte el defecto. Con `json.dumps` en dos pasos,
`ensure_ascii=True` y `False` dan el mismo resultado —`No ingres\\u00f3`—, así que
la comparación no distingue nada. **Un control que comparte la vía del defecto no
controla.**

La regla: **abrir SQLite con Python y comparar valores en Python**, nunca un
literal acentuado pasado por capas de comillas, `ssh`, `sh -c` y `json.dumps`. El
script lo hace bien por construcción: los literales van como **parámetro** `?`
(`sys.argv`-style binding), no interpolados en el SQL.

Y en una guarda, la dirección importa más que la detección:

```
guarda:  estado_checkin != ?        literal corrupto -> devuelve todas -> ABORTA
```

Falla **cerrado**. Una guarda que fallara abierto —dejando pasar el borrado porque
la comparación no coincide con nada— sería peor que no tenerla. Pero un fallo
cerrado permanente también es un problema: **aborta siempre y parece protección**.
Por eso la guarda necesita su control negativo, el caso en que se la ve **no**
abortar y borrar correctamente.

#### «Host» es ambiguo, y por eso se enumeran tres sitios

Decir «el host no tiene copias» es ambiguo: puede ser la máquina donde hablas o el
host del VPS fuera del contenedor. Son dos sistemas de ficheros distintos, y una
copia vivió en el segundo mientras el primero se declaraba limpio.

Se enumeran **tres**, con nombre completo:

```
local        la máquina donde se prepara
VPS host     el host del VPS, fuera del contenedor
contenedor   el sistema de ficheros del contenedor
```

Y se enumeran, no se recuerdan: `ls /tmp/*.py` en cada uno, no `rm` de nombres
conocidos. Un `rm -f` de algo inexistente devuelve 0 y refuerza la falsa sensación
de limpieza.

#### Una medición limpia dos veces, o no es una medición

Un número que sale limpio en la primera medición y no en la segunda **no es una
medición: es una coincidencia**. Y el caso peligroso no es el que falla, es el que
**sale limpio en la primera y nadie repite**:

```
314 / 340   salieron limpios, eran un botón sin clases     (markup inventado)
9           salió limpio, era el separador del comando      (instrumento roto)
8           salió limpio, era json.dumps escapando          (emisor roto)
```

Los tres eran plausibles, los tres se publicaron, y **ninguno describía la app**.

De ahí una regla barata: **una cifra que decide algo se mide dos veces, por vías
distintas.** No para confirmarla, sino para descubrir que la primera vía era el
instrumento. Si las dos vías coinciden, hay algo; si no, lo que se estaba midiendo
era la vía.

Y una variante de lo mismo, verificada en esta sesión: **un delimitador dentro de
capas de shell es tan frágil como un literal acentuado**, y **las llaves de una
plantilla compiten con las del shell**. No son trampas nuevas: son «suponer que el
canal transporta lo que escribo», la misma familia.

Conviene saber además que **no se reproducen siempre**. Probado con dos y tres capas
de `sh -c`, con `sudo` en medio, y con `@@`, `|` y `$` como separadores: en esta
máquina llegan intactos. **Un mecanismo plausible no demuestra que ocurriera** — la
atribución honesta es «puede pasar por esta vía», no «pasó por esta vía».

#### `ssh` concatena argv: el caso que sí se reproduce

El límite anterior tenía una causa concreta y reproducible, y no era ni el separador
ni las llaves de plantilla. Estaba una capa más arriba, en **cómo se invocaba `ssh`**:

```python
subprocess.run(["ssh", host, "sh", "-c", cmd])        # mal: argv separado
subprocess.run(["ssh", host, f"sh -c '{cmd}'"])       # bien: una sola cadena
```

**`ssh` no pasa argv al remoto: los concatena con espacios.** Con la forma separada,
el remoto recibe `sh -c sudo docker exec …`, y `sh -c` toma **sólo la palabra
siguiente** (`sudo`) como comando, dejando el resto en `$0 $1 …`. Medido:

```
["ssh", host, "sh", "-c", cmd]   ->  rc=1  9 líneas  "usage: sudo -h | -K | -k | -V"
["ssh", host, "sh -c '<cmd>'"]   ->  rc=0  "hola"
```

**Nueve líneas.** Un parser que contaba líneas como filas de datos reportó
`total=9, ingresadas=9` sobre el texto de ayuda de `sudo`.

De ahí la lección, que es la parte general:

> **Un parser que no comprueba la forma de la entrada convierte un mensaje de error
> en un resultado.**

Y la de atribución, que es la misma de antes: **un síntoma con dos causas posibles se
atribuye midiendo, no eligiendo.** Las dos explicaciones publicadas antes —«el
separador se rompió» y «las llaves compiten con el shell»— eran ficción: probadas
con dos y tres capas de `sh -c`, con `sudo` en medio y con `@@`, `|` y `$` como
separadores, llegan intactas.

Antes de interpretar una salida, **mirar su forma**. Un comando que devuelve algo que
parece de otro programa está diciendo que no se ejecutó lo que se creía.

## 16. Purgar la caché de Cloudflare después de cada despliegue

Los assets estáticos se sirven con `public, max-age=60, must-revalidate`
(`src/router.rs`, `ASSET_CACHE_CONTROL`). Antes salían con `no-store`, y eso
hacía que cada visita re-descargara 1,2 MB en 85 módulos con una cascada de
imports de 6 niveles: el panel tardaba segundos en aparecer.

Cloudflare **reescribe** ese `max-age` con su propio *Browser Cache TTL* (hoy
4 h) para las extensiones que cachea, así que un despliegue que cambie JS o CSS
puede quedarse invisible hasta cuatro horas. Por eso el despliegue termina con
una purga:

```bash
TOK=$(grep '^CLOUDFLARE_API_TOKEN=' .env | cut -d= -f2-)
curl -s -X POST -H "Authorization: Bearer $TOK" \
     -H "Content-Type: application/json" \
     -d '{"purge_everything":true}' \
     "https://api.cloudflare.com/client/v4/zones/83854199272ff369ab7cfbc6cafa2eb8/purge_cache"
```

Se comprueba así: la primera petición responde `cf-cache-status: MISS` y la
segunda `HIT` con `age: 0`.

**No se purga el HTML**: se sirve con `no-cache` a propósito, porque es el
documento que apunta a los puntos de entrada versionados y una copia vieja
fijaría el bundle entero.

**El `no-store` del endpoint de ticket se queda.** Ahí protege un secreto y no
es caché de assets.

## 17. Si el panel vuelve a parecer lento: la cascada, no la cabecera

Antes de tocar `Cache-Control` otra vez, leer esto. La caché ya se arregló y
**no es el problema que queda.**

### Lo que se arregló y lo que no

`ASSET_CACHE_CONTROL` (`src/router.rs`) pasó de `no-store` a
`public, max-age=60, must-revalidate`. Eso eliminó la re-descarga de 1,2 MB en
cada visita. **Lo que no elimina es la latencia del primer load**, porque el
bundle son **85 módulos alcanzables en una cascada de imports de 6 niveles**.

### Los números, medidos

```
peso del bundle        1,2 MB   (1076 KB js + 137 KB css) en 145 archivos
módulos alcanzables    85
profundidad de cascada 6 niveles
TTFB en el origen      1,5 ms
TTFB por el túnel      ~0,8 s
```

Un navegador descubre los imports nivel por nivel: hasta no parsear el nivel N
no sabe qué pedir del nivel N+1. Con 6 niveles y ~0,8 s por viaje, el primer
load paga del orden de **6 × 0,8 s solo en espera de red**, antes de ejecutar
nada.

### Dónde se cachea de verdad, y dónde no

```
landing  /assets/js/main.js   public, max-age=14400 (lo reescribe Cloudflare)
                              cf-cache-status: HIT con age creciente
landing  /                    no-cache · cf-cache-status: DYNAMIC
panel    /assets/js/main.js   302 a Access; el panel NUNCA se cachea en el borde
```

**El panel va detrás de Cloudflare Access, y Access corta antes de la caché.**
Ahí el navegador ve los 60 s del origen, no las 4 h que Cloudflare impone a la
landing. Por eso subir el `max-age` del origen no arregla el panel: solo alarga
la vida en el navegador de cada usuario.

### Qué mirar, en orden

1. **Reducir la profundidad de la cascada.** Es la palanca real. Agrupar
   módulos o aplanar los imports de los niveles 3-6 baja el número de viajes
   secuenciales; ninguna cabecera lo hace.
2. **Comprobar que no volvió el `no-store`.** Si alguien lo reintroduce «por
   seguridad», vuelve la re-descarga de 1,2 MB. El test
   `test_static_assets_are_cacheable_while_html_is_not` lo detecta.
3. **Subir `max-age` solo si el problema es de visitas repetidas**, no del
   primer load. Y recordar que el despliegue termina con purga (§16), así que
   un `max-age` largo no deja el deploy invisible en el borde, pero sí en el
   navegador de quien ya tenía la copia.

### Un señuelo que existe en producción

`/app/data/shirley.db` — **0 bytes, 0 tablas**. No lo crea la aplicación: no
hay ninguna referencia a ese nombre en `src/`, `compose.yaml` ni la
configuración. La base real es `/app/data/shirley-show.sqlite3`.

Está al lado de la real, y un fichero de base vacío es exactamente lo que
alguien abre un día creyendo que es la base. No se borró desde el runbook
porque tocar el volumen de datos no es tarea de una nota de despliegue; queda
anotado para quien pueda hacerlo con criterio.
