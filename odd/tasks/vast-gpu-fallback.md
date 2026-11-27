# Vast.ai GPU fallback for vocal separation

Colab's GPU selection can stall indefinitely; the client needs separations to keep running. Add Vast.ai as a cheap-GPU fallback behind the same separation contract, with a worker image that separates without installing anything at runtime.

## Specs

S1. "si parece que demora mucho la seleccion de gpus de google, otra opcion para obtener gpus?, el cliente lo necesita"
S2. "pero en todo caso mejor busca una grafica economica en ves de una blackwell, que corra bien los modelos del sistema"
S3. "claro, y hay que aprovechar en dejar viva una grafica por 10 minutos si no se hace mas separaciones en esos 10 minutos recien se apaga"
S4. "claro por eso es que si google nos da una grafica hay que quedarnos con esa hasta que el usuario ya no la vaya a usar realmente"

## Tasks

T1 (S1, S2) — Worker image: `docker/vast-worker/Dockerfile` with a CUDA PyTorch base on drivers CUDA >= 12.4, ffmpeg, a pinned `audio-separator` with GPU support, and every model of both modes pre-downloaded into the image's model dir, so a Vast instance separates with no install and no model download. The existing generated script (`generate_remote_script`) must run on it unchanged: `import audio_separator` succeeds and `/content` exists. Also `.github/workflows/vast-worker-image.yml` (build+push `ghcr.io/loonbac/shirley-vast-worker:<sha>` and `:latest`) and `docs/vast-worker.md`. Route: gentle-ai-worker. Commit: pending.
T2 (S1) — Vast adapter plus fallback: a Vast.ai backend implementing the same vocal-separator port, used only when Colab does not hand over a GPU in time (~60s), reusing the hot-session retention rules. Route: pending. Commit: pending.
T3 (S1, S2) — Honest "no GPU" message: when neither Colab nor Vast can provide a GPU, report that plainly instead of the current "cuota agotada". Route: pending. Commit: pending.
T4 (S1, S2) — Delivery and live test: branch, PR, merge and deploy through the repository's flow, then a real separation on production through whichever GPU the fallback provided. Route: parent. Commit: pending.

## Log

L1 (decisión del usuario): se elige Vast.ai como proveedor de GPU alternativo. Evidencia aportada por el usuario: captura de Vast.ai con **$9.32 de crédito** disponible.
L2 (evidencia, sonda en vivo del padre sobre Vast.ai con la imagen `pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime`): GPUs de 24 GB baratas y fiables — RTX 3090 ~$0.17-0.26/h, A5000 ~$0.21/h, 4090 ~$0.38-0.46/h. Instancia lista + SSH en **67s** (host CN) y **177s** (host PL). `apt-get install ffmpeg` tardó **220-480s**. `pip install -q audio-separator[cpu]` FALLÓ en ~16s en ambos hosts (CN y PL, python 3.11 de esa imagen) con stderr suprimido y causa desconocida; se sospechaba una dependencia que necesita compilador (por ejemplo `diffq`) ausente en la imagen runtime.
L3 (causa real del fallo de pip, reproducida localmente, T1): sobre `pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime`, `pip install "audio-separator[cpu]"` falla al construir el wheel de `diffq`, que es dependencia obligatoria (no opcional) de `audio-separator`:

```
Building wheels for collected packages: diffq
  × Building wheel for diffq (pyproject.toml) did not run successfully.
  ╰─> building 'diffq.bitpack' extension
      gcc -pthread ... -c bitpack.c -o build/temp.linux-x86_64-cpython-311/bitpack.o
      error: [Errno 2] No such file or directory: 'gcc'
ERROR: Failed building wheel for diffq
```

La imagen `pytorch/pytorch:*-runtime` no trae toolchain C (`gcc` ausente), así que la causa no es la red ni la cuota sino la compilación de `diffq`. `audio-separator[gpu]` no evita ese paso: no hay wheel de `diffq` para linux. En Vast el `-q` del script ocultaba exactamente este stderr.
L4 (instrucción del usuario, literal): "dile que no compile en instabox, que compile en local" — T1 no compila ni ejecuta nada en instabox; la reproducción del fallo de pip, el build de la imagen y el smoke run en CPU se ejecutan en esta máquina.
L5 (instrucción del usuario, literal, ampliación del encargo de T1): "dile que no compile en instabox, que compile en local". Nada de T1 se construyó ni ejecutó en instabox. Lo único que se había llegado a crear allí fue la descarga de `pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime`, que se borró con `docker image rm` y se limpió con `docker builder prune -af`; `/data` quedó en 3.0G libres, igual que al empezar, y `/` pasó de 16G usados al 100% a 6.6G (45%). La reproducción, el build y el smoke run se hicieron en local.
L6 (T1, decisión de diseño): la imagen parte de `python:3.10-slim-bookworm` con los wheels `torch==2.5.1` cu124 de `download.pytorch.org` en vez de una base `nvidia/cuda:*-runtime`. Motivo: los wheels cu124 traen el runtime CUDA completo, así que el host solo aporta el driver (CUDA >= 12.4) y la imagen no arrastra una base de ~9.6 GB (el `pytorch/pytorch` runtime medido) antes de ffmpeg ni de los modelos.
L7 (T1, dos fallos reales del build, encontrados y corregidos):
 (a) `audio-separator --download_model_only` aborta con `FileNotFoundError: 'ffmpeg'`: el CLI valida ffmpeg ANTES de descargar, incluso cuando no va a separar nada. La etapa builder instala `ffmpeg` junto a `build-essential`.
 (b) `audio-separator` solo declara `torch>=2.3,<3`, así que pip REEMPLAZÓ el build cu124 fijado por un `2.14.1+cu130` del índice por defecto: ese build exige un driver más nuevo que el objetivo del encargo y habría fallado en hosts Vast válidos para CUDA 12.4. Se fija con un archivo de constraints y el build ahora falla ruidosamente si `torch.__version__` o `torch.version.cuda` se desvían (`assert` en la etapa builder).
L8 (T1, verificación ejecutada, RED): `pip install "audio-separator[cpu]"` sobre `pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime`, con red y DNS funcionando y sin `-q`, falla en ~29s con `error: [Errno 2] No such file or directory: 'gcc'` al construir el wheel de `diffq`. Causa raíz exacta del fallo que la sonda en vivo vio en Vast.
L9 (T1, verificación ejecutada, GREEN): build local de `docker/vast-worker/` con `--network host` (solo por el DNS del bridge de esta máquina). Imagen `shirley-vast-worker:t1`, **13.5 GB** en disco (`docker image inspect` → 13.516.915.871 bytes), `torch 2.5.1+cu124` / `torch.version.cuda == "12.4"`, `audio-separator 0.48.0`, ffmpeg y ffprobe en `/usr/bin`, ambos modelos más sus YAML y `download_checks.json` en `/opt/models`, y `/tmp/audio-separator-models` resolviendo a `/opt/models`. Capas más grandes: `COPY /opt/venv` 6.47 GB, `COPY /opt/models` 1.55 GB, ffmpeg+openssh 475 MB.
L10 (T1, verificación ejecutada, smoke run de aceptación): el script de coros EXACTO generado por `generate_remote_script` (sha256 `a72461316fbc63ce95cf22517b805a23cd4fc74195f633bb510dad01096807ab`), sobre un MP3 sintético de 20s de ffmpeg lavfi, ejecutado con `--network none`:

```
docker run --rm --network none -v <scratch>:/work --entrypoint bash shirley-vast-worker:t1 \
  -lc 'cp /work/input_audio /content/input_audio; cd /content && python /work/chorus_script.py'
{"url": "", "sha256": "b0cafb665f4d3dce62b7c025a203735040d8ebb825c503324d12662e208366b4", "duration": 20.035918, "size": 802525}
```

`output.mp3` quedó en `/content/colab_sep_work/output.mp3` y su `ffprobe` independiente confirma `duration=20.035918`, `size=802525` (802.525 bytes), es decir los mismos valores del JSON. Sin red: `import audio_separator` resolvió desde `/opt/venv` y los modelos desde `/opt/models`, sin descargas.
L11 (T1, límite encontrado y corregido): un shell de login (`bash -lc`, que es lo que abre SSH) reinicia `PATH` desde `/etc/profile` y perdía el venv, con lo que el script caía en su rama de `pip install` y fallaba igual que en la sonda. La imagen ahora escribe `/etc/profile.d/venv.sh` además de `/etc/environment`; sin ese archivo, un operador entrando por SSH habría visto el fallo de pip otra vez.
L12 (T1, decisión sobre el modo de lanzamiento de Vast): con launch mode `ssh` o `jupyter`, Vast REEMPLAZA el entrypoint de la imagen y arranca su propio sshd, y no propaga las variables de entorno de la plantilla a la sesión SSH. Por eso la imagen trae `openssh-server` y escribe el entorno en `/etc/environment` y `/etc/profile.d/`, y el symlink `/tmp/audio-separator-models -> /opt/models` cubre el caso de una sesión sin entorno alguno.
L13 (T1, riesgo declarado, punto 5 del encargo): se añade un workflow de CI nuevo (`.github/workflows/vast-worker-image.yml`) que publica en GHCR con `GITHUB_TOKEN` y `packages: write`, y una dependencia nueva de terceros fijada (`audio-separator[gpu]==0.48.0` sobre wheels `torch==2.5.1` cu124). Consecuencias: (a) el paquete GHCR nace privado y **el propietario debe hacerlo público** (paso documentado en `docs/vast-worker.md`) o Vast no podrá descargar la imagen; (b) subir `audio-separator` de versión exige revisar que los nombres de modelo sigan existiendo y que el pin de torch siga verificado; (c) el workflow publica `:latest` en cada cambio de `docker/vast-worker/**`, así que producción debe fijar el tag `:<sha>`, no `:latest`.
L14 (T1, pendiente fuera de alcance): la imagen es solo `linux/amd64`; no se probó en una GPU real (la verificación fue en CPU, sin GPU en esta máquina), y T2 (adaptador Vast + fallback), T3 (mensaje honesto de "no hay GPU") y T4 (entrega + prueba en vivo) siguen pendientes.
