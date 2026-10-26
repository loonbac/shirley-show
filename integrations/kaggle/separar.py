"""
Kernel de Kaggle: separación de voces en máxima calidad.

Corre en background con `kaggle kernels push`. No hace falta tener el notebook
abierto: se lanza, corre solo, y el MP3 queda en los outputs del kernel.

Lee el audio de un dataset de Kaggle (entrada) y escribe output.mp3 en
/kaggle/working, que es lo que `kaggle kernels output` descarga.

Modo se elige con la variable KAGGLE_MODE ("chorus" o "instrumental").
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

WORK = Path("/kaggle/working")
INPUT_DIR = Path("/kaggle/input")
MODELS_DIR = WORK / "models"

# Ensemble por pipeline. Nombres exactos del catálogo de audio-separator.
# Para "coros" se promedia el stem NO vocal (instrumental + coros).
# Para "instrumental" se promedia el stem instrumental de modelos afinados
# para no arrastrar bleed de voz.
MODELS = {
    "chorus": [
        "bs_roformer_karaoke_frazer_becruily.ckpt",
        "bs_roformer_karaoke_anvuew.ckpt",
        "mel_band_roformer_karaoke_gabox_v2.ckpt",
        "mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt",
    ],
    "instrumental": [
        # El mejor instrumental medido del catálogo: 17.0 dB SDR.
        "model_bs_roformer_ep_317_sdr_12.9755.ckpt",
        "melband_roformer_inst_v1e.ckpt",
        "mel_band_roformer_instrumental_becruily.ckpt",
    ],
}

# Numero de ventanas superpuestas. Mas overlap = menos artefactos de costura
# entre ventanas, a cambio de 4-6x mas pasadas. Es la palanca de calidad que
# SI depende de la GPU: en 16 GB conviene 8; en 40-96 GB se puede subir.
DEFAULT_OVERLAP = 8


def log(msg):
    print(msg, flush=True)


def run(cmd, **kw):
    log("$ " + " ".join(str(c) for c in cmd))
    subprocess.run([str(c) for c in cmd], check=True, **kw)


def ensure_audio_separator():
    """
    Kaggle no trae audio-separator. Se instala el extra [cpu]:
    los modelos Roformer corren con PyTorch en CUDA, y onnxruntime solo
    necesita estar importable. El extra [gpu] anade onnxruntime-gpu, que
    suele romper con CUDA 13 + Python 3.13.
    """
    try:
        import audio_separator  # noqa: F401
        log("audio-separator ya disponible")
        return
    except ImportError:
        pass
    log("Instalando audio-separator[cpu] ...")
    run([sys.executable, "-m", "pip", "install", "-q", "audio-separator[cpu]"])


def find_input_audio():
    exts = {".mp3", ".wav", ".flac", ".m4a", ".ogg", ".opus", ".aac"}
    candidates = [
        p for p in INPUT_DIR.rglob("*")
        if p.is_file() and p.suffix.lower() in exts and p.stat().st_size > 10_000
    ]
    if not candidates:
        sys.exit(f"No encontré audio en {INPUT_DIR}")
    if len(candidates) > 1:
        log(f"Aviso: {len(candidates)} audios en el dataset, uso el primero: {candidates[0]}")
    return candidates[0]


def separate_one(model, audio, out_dir):
    """Corre un modelo y devuelve el stem no vocal."""
    out_dir.mkdir(parents=True, exist_ok=True)
    run([
        "audio-separator", audio,
        "--model_filename", model,
        "--model_file_dir", MODELS_DIR,
        "--output_dir", out_dir,
        "--output_format", "WAV",
        "--mdxc_overlap", str(OVERLAP),
        "--log_level", "WARNING",
    ])
    wavs = sorted(out_dir.glob("*.wav"))
    if not wavs:
        sys.exit(f"{model} no produjo ningún WAV en {out_dir}")

    # El stem que conservamos es el que NO es voz. Los modelos de 2 stems
    # escriben "Vocals" en el nombre del stem vocal; el resto es el que sirve.
    non_vocal = [w for w in wavs if "vocal" not in w.name.lower()]
    if len(non_vocal) != 1:
        sys.exit(
            f"{model}: esperaba un stem no vocal, encontré {[w.name for w in wavs]}"
        )
    return non_vocal[0]


def average_and_encode(stems, output):
    """Promedio de N stems con ffmpeg y encode MP3 CBR 320k."""
    n = len(stems)
    inputs = []
    for s in stems:
        inputs += ["-i", str(s)]

    # amix normaliza por defecto (divide por N): es exactamente el promedio.
    filt = f"amix=inputs={n}:duration=first:dropout_transition=0"
    run([
        "ffmpeg", "-y", "-v", "error", *inputs,
        "-filter_complex", filt,
        "-map_metadata", "-1",
        "-codec:a", "libmp3lame", "-b:a", "320k",
        str(output),
    ])


def verify(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries",
         "stream=codec_name,bit_rate,duration",
         "-of", "default=noprint_wrappers=1", str(path)],
        capture_output=True, text=True,
    )
    log("ffprobe:\n" + out.stdout.strip())
    if path.stat().st_size < 50_000:
        sys.exit(f"El MP3 quedó demasiado pequeño: {path.stat().st_size} bytes")


def resolve_overlap():
    """
    Overlap de ventanas para los modelos RoFormer (--mdxc_overlap, 2-50).

    Se puede subir con un `overlap.txt` en el dataset, sin tocar el codigo.
    En una GPU de 16 GB, subirlo mucho puede terminar en OOM; 8 es el valor
    que ya usan los YAML de karaoke y el punto de partida sensato.
    """
    for p in INPUT_DIR.rglob("overlap.txt"):
        raw = p.read_text().strip()
        try:
            value = int(raw)
        except ValueError:
            sys.exit(f"{p} debe contener un entero entre 2 y 50; contiene {raw!r}")
        if not 2 <= value <= 50:
            sys.exit(f"overlap fuera de rango en {p}: {value}")
        log(f"Overlap tomado de {p}: {value}")
        return value
    return DEFAULT_OVERLAP


def resolve_mode():
    """
    El modo se elige con un `mode.txt` dentro del dataset de entrada.

    `kaggle kernels push` no permite pasar variables de entorno, así que el
    dataset es el único canal para decirle al kernel qué pipeline correr.
    """
    for p in INPUT_DIR.rglob("mode.txt"):
        value = p.read_text().strip().lower()
        if value in MODELS:
            log(f"Modo tomado de {p}: {value}")
            return value
        sys.exit(f"{p} debe contener uno de {list(MODELS)}; contiene {value!r}")
    env = os.environ.get("KAGGLE_MODE", "instrumental").strip().lower()
    log("Aviso: no hay mode.txt en el dataset, uso el valor por defecto")
    return env


def main():
    global OVERLAP
    OVERLAP = resolve_overlap()
    mode = resolve_mode()
    if mode not in MODELS:
        sys.exit(f"Modo inválido: debe ser {list(MODELS)}; recibí {mode!r}")

    ensure_audio_separator()
    audio = find_input_audio()
    log(f"Modo: {mode}")
    log(f"Modelos: {len(MODELS[mode])} (overlap {OVERLAP})")
    log(f"Entrada: {audio}")

    # audio-separator descarga los checkpoints aquí (--model_file_dir). No hay
    # caché entre ejecuciones: cada kernel vuelve a bajarlos.
    MODELS_DIR.mkdir(parents=True, exist_ok=True)

    stems = []
    for i, model in enumerate(MODELS[mode], 1):
        log(f"=== [{i}/{len(MODELS[mode])}] {model} ===")
        try:
            stems.append(separate_one(model, audio, WORK / f"stem_{i}"))
        except subprocess.CalledProcessError as e:
            # Un modelo que falla no debe tirar el ensemble entero si ya hay
            # al menos dos stems: con dos se puede promediar igual.
            log(f"AVISO: {model} falló ({e}). Sigo con los demás.")
            if len(stems) >= 2:
                continue
            raise

    if len(stems) < 2:
        sys.exit(f"Solo {len(stems)} stem(s) válido(s); hacen falta 2 para promediar")

    log(f"Promediando {len(stems)} stems")
    output = WORK / "output.mp3"
    average_and_encode(stems, output)
    verify(output)
    log(f"Listo: {output} ({output.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
