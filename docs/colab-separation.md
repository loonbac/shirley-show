# Separación vocal con Google Colab

La separación vocal corre en una GPU de Google Colab, no en la PC de Joshua. La GPU se enciende cuando llega una separación y se apaga en cuanto termina.

## Requisitos

1. El binario colab (google-colab-cli oficial) viene instalado en la imagen por el Dockerfile.
2. Un token OAuth en SHIRLEY_COLAB_TOKEN con access_token, refresh_token y expiry. El refresh_token permite renovarlo sin navegador. Se monta como archivo de lectura-escritura en /home/shirley/.config/colab/token.json (ver compose.yaml). El adaptador copia ese archivo a ~/.config/colab-cli/token.json (la vía heredada que lee el CLI oficial) antes de cada uso.

Sin token, la separación falla con colab_token_missing **antes** de encender la GPU.

## Variables

| Variable | Default | Para qué |
|---|---|---|
| SHIRLEY_COLAB_BIN | colab | Binario de colab-cli |
| SHIRLEY_COLAB_TOKEN | ~/.config/colab/token.json | Credenciales |
| SHIRLEY_COLAB_GPU | g4 | Tipo de GPU. g4 = RTX PRO 6000 Blackwell, 102 GB |
| SHIRLEY_COLAB_OVERLAP | 12 | Overlap de los modelos Roformer |
| SHIRLEY_COLAB_TIMEOUT_SECS | 1800 | Tiempo máximo por separación |
| SHIRLEY_COLAB_WORK_DIR | directorio de la app | Archivos temporales |

## Ciclo de vida

1. Se valida el token y la duración. Si algo falla, no se enciende nada.
2. colab new -s <nombre> --gpu <tipo> solo cuando hay trabajo.
3. Subida del audio con colab upload -s <nombre> <local> <remoto>, ejecución del
   ensemble con colab exec -s <nombre> --timeout <segundos> -f <script>, y
   descarga del resultado con colab download -s <nombre> <remoto> <local> (el
   download propio del CLI).
4. Verificación de SHA-256 y de duración contra lo que reportó el runtime.
5. colab stop -s <nombre> siempre: éxito, error o timeout.

`--timeout` es el silencio que el CLI tolera del kernel, no el límite del
proceso: el CLI aborta la llamada cuando el kernel no imprime nada durante ese
número de segundos, y su valor por defecto son 30. Una separación real está
callada más tiempo mientras instala paquetes y carga los modelos, así que cada
`colab exec` recibe el presupuesto entero que le queda al paso, en segundos
enteros redondeados hacia arriba y nunca `0` (`0` significa «sin límite»). El
límite que corta de verdad sigue siendo el timeout de tokio del adaptador.

Un guard con Drop lanza colab stop si el proceso entra en pánico o se cancela, para que la GPU no quede encendida.

## Diagnóstico de un fallo

Cuando `colab exec` falla, el adaptador escribe **siempre** una línea en stderr
(no depende de `SHIRLEY_COLAB_DEBUG_OUTPUT`), con la sesión, el código de salida
o el límite que lo cortó, y la última línea del stderr del CLI recortada a ~300
caracteres. No incluye el token. El error que ve el usuario no cambia:

    [colab exec] fallo session=shirley-1a2b3c4d exit=1 stderr=TimeoutError: Timeout waiting for output: no message from the kernel for 30s. Use --timeout to allow longer silences.

Así el siguiente fallo en producción se diagnostica con `docker logs`.

## Coste medido

- Burn rate de una G4: 8.9 CCU/h.
- Canción de 2:39 con ensemble de 4 modelos y overlap 12: 5m 11s, aproximadamente 1 CCU.
- Con 300 CCU: del orden de 290 canciones.

## Límites conocidos

- Se usa el colab download propio del CLI oficial; el resultado viaja sin el
  workaround de catbox/curl. Se mantiene la verificación de SHA-256 y de
  duración contra lo reportado por el runtime.
- colab exec descarta la salida de más de ~16 KB, así que el script remoto imprime una sola línea JSON.
- El silencio del kernel lo controla `colab exec --timeout`; el adaptador le pasa
  el presupuesto del paso. Si se invoca el CLI a mano, sin ese flag, vuelve el
  corte a los 30s.
- El puerto 8080 del runtime lo ocupa el proxy del CLI.
- Un SIGKILL, un OOM del contenedor o un abort del proceso saltan los destructores, así que el guard de Drop no llega a ejecutarse y el runtime puede seguir vivo hasta que Colab lo expire por inactividad. No hay forma de evitarlo desde el cliente. Tras un reinicio duro del contenedor, conviene ejecutar colab stop -s <nombre> (o revisar colab usage) para no dejar la GPU consumiendo.

## Renovar el token

    colab auth

Necesita un navegador y un túnel al puerto que imprima el comando. El refresh_token evita repetirlo mientras siga vigente.

## Dónde viven el token y las sesiones

El CLI oficial lee su token heredado de `~/.config/colab-cli/token.json` y guarda
las sesiones en `~/.config/colab-cli/sessions.json` (directorio creado por el
Dockerfile, propiedad de UID 1000). El token del host se monta en
`/home/shirley/.config/colab/token.json` y el adaptador lo copia a la vía
heredada antes de cada uso. El montaje sigue siendo de lectura-escritura para que
el refresco del access_token (que reescribe el archivo) no rompa la separación a
la hora. El archivo del host debe pertenecer a UID 1000 para que el contenedor
pueda escribirlo.
