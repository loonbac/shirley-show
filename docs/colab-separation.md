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
| SHIRLEY_COLAB_GPU | g4 | GPU preferida. g4 = RTX PRO 6000 Blackwell, 102 GB |
| SHIRLEY_COLAB_GPU_FALLBACK | a100,l4 | GPUs adicionales elegibles, separadas por coma |

La configuración decide **qué** GPUs son elegibles, nunca el orden: los intentos
van siempre por rendimiento, la más rápida primero. La cadena por defecto es
**G4 → A100 → L4**.

- **G4** (RTX PRO 6000 Blackwell, 102 GB): la más rápida medida en este proyecto.
- **A100**: la GPU sobre la que se midieron los modelos; más lenta que G4 y más
  rápida que el resto.
- **L4**: el piso fiable, la que siempre respondió cuando se le pidió.
- **T4**: excluida por defecto (no cabe en la reserva del pipeline); sólo se
  alcanza si la configuración la nombra, y siempre al final.
- **H100**: descartada. El CLI acepta el nombre, pero esta cuenta no tiene acceso
  (la interfaz de Colab la muestra en gris) y en producción toda asignación de
  H100 devolvía HTTP 400, así que cada intento gastaba una vuelta en una máquina
  imposible. Configurada o no, nunca se intenta: se trata igual que un valor que
  el CLI no acepta.
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
5. La GPU se conserva: si la separación termina bien, o falla DESPUÉS de tener
   GPU asignada (subida rechazada, exec distinto de cero o agotado, salida
   ilegible, descarga fallida), la sesión sigue viva y queda en caché durante 10
   minutos contados desde el final de ese intento. La siguiente separación la
   reutiliza en vez de volver a pelear con los 503 de Colab. Sólo se apaga en el
   acto cuando la sesión está probada muerta: el CLI responde que la sesión o el
   runtime no existe, o la sonda de salud (un `colab exec` trivial) falla. Una
   sesión rota nunca queda cacheada, y el reaper de huérfanos y la expiración de
   10 minutos siguen aplicándose.
6. colab stop -s <nombre> en el apagado diferido, o de inmediato si la sesión
   está muerta.

`--timeout` es el silencio que el CLI tolera del kernel, no el límite del
proceso: el CLI aborta la llamada cuando el kernel no imprime nada durante ese
número de segundos, y su valor por defecto son 30. Una separación real está
callada más tiempo mientras instala paquetes y carga los modelos, así que cada
`colab exec` recibe el presupuesto entero que le queda al paso, en segundos
enteros redondeados hacia arriba y nunca `0` (`0` significa «sin límite»). El
límite que corta de verdad sigue siendo el timeout de tokio del adaptador.

Un guard con Drop lanza colab stop si el proceso entra en pánico o se cancela, para que la GPU no quede encendida. Se desactiva cuando la sesión queda deliberadamente viva (éxito, o fallo con la sesión sana), porque entonces el apagado lo programa la tarea de retención.

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

## Fallback a Vast.ai (T2/T3)

Colab sigue siendo el proveedor principal y **se prueba primero**: por decisión
del propietario, Vast no se alquila en paralelo, sino sólo cuando Colab no ha
entregado GPU en su ventana. Evidencia del despliegue `4155c11`: dos alquileres
RTX 3090 (Noruega y Argentina) de la imagen anterior siguieron en `loading` más
de 4,5 min y el turno de chat expiró a los 600 s sin separación; en cambio las
sondas con `pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime` estuvieron `running`
en 36 s (CN) y 175 s (PL). Colab sigue respondiendo 503 y renuncia en ~60 s. La
imagen y los filtros de Vast se mejoraron para que, cuando llega su turno,
alquile rápido y no gaste el deadline del turno descargando.

Si hay `VAST_API_KEY`, el adaptador compuesto (`FallbackVocalSeparator`)
aplica estas reglas:

1. Si ya existe una instancia Vast caliente (rentada en los últimos 10 minutos y
   todavía `running`), se usa directamente: ni se intenta Colab ni se vuelve a
   alquilar.
2. Si no, se limita **sólo la fase de asignación** de Colab a
   `SHIRLEY_VAST_FALLBACK_AFTER_SECS` (60 s, S4). Si Colab entrega GPU, conserva
   el deadline completo para subida/ejecución/descarga y Vast no se alquila. Si
   Colab no entrega GPU en esa ventana, Vast empieza a rentar **entonces** y
   termina la separación con el tiempo que quede del deadline original.
3. Sin `VAST_API_KEY` el comportamiento es el de siempre: sólo Colab, con el
   timeout completo y sin pedir nada a Vast.

| Variable | Default | Para qué |
|---|---|---|
| VAST_API_KEY | — | Habilita el fallback. Nunca se escribe en logs. |
| SHIRLEY_VAST_IMAGE | ghcr.io/loonbac/shirley-vast-worker:latest | Imagen a rentar; **fijar un tag `:sha` en producción**. |
| SHIRLEY_VAST_MAX_DPH | 0.6 | Precio máximo por hora de la oferta. |
| SHIRLEY_VAST_FALLBACK_AFTER_SECS | 60 | Ventana de asignación de GPU concedida a Colab. |
| SHIRLEY_VAST_LAUNCH_TIMEOUT_SECS | 150 | Tiempo máximo que se espera a que **una oferta** llegue a `running`; al vencer se destruye y se prueba la siguiente. |

El adapter pide primero ofertas con **≥ 1000 Mbps de bajada** y las ordena por
precio; si hay menos de 5 aceptables repite la consulta con el piso anterior de
**≥ 500 Mbps**. El resto de filtros no cambia (verificada, rentable, no rentada,
fiabilidad ≥ 0.99, 1 GPU, ≥ 20 GB de VRAM, CUDA ≥ 12.4, ≥ 40 GB de disco,
on-demand), excluye hosts CN/China/HK y sólo considera RTX 3090/4090/A5000/
A6000, L4, A10 y RTX 4080S. Prueba la siguiente oferta si el alquiler falla o
se queda en `loading` más de 150 s. Genera una clave ed25519 bajo
`<data>/vast/ssh` (modo 600), la registra con `POST /ssh/` si falta, y usa
`ssh`/`scp` con timeouts estrictos. El script remoto es el mismo de Colab; la
verificación SHA-256 y de duración es idéntica. La instancia se conserva 10
minutos tras la última separación (éxito o fallo con máquina sana) y se destruye
antes si está muerta; al arrancar la app se destruyen las instancias etiquetadas
`shirley-show` que hubiera dejado un reinicio anterior, y un reaper de seguridad
hace lo mismo cada 5 minutos. El alquiler y el pipeline viven en una tarea
independiente: si se cancela la petición (deadline del agente o desconexión), el
apagado o el temporizador de retención se ejecutan igual, y una instancia que
se alquila después de la cancelación se destruye de inmediato.

Cuando ningún proveedor da GPU, el error mantiene el código `colab_unavailable`
pero el mensaje es honesto: «No hay GPU disponible ahora mismo en los
proveedores; inténtalo de nuevo en unos minutos.» No se menciona una cuota
agotada porque los créditos están disponibles.

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
