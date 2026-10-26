# Separación de voces en Kaggle (GPU gratis, sin navegador)

Corre un ensemble de modelos Roformer en la GPU gratuita de Kaggle, en
**background**, sin tener ningún notebook abierto. El resultado es un MP3 320k
que se sube a Shirley Show como pista.

## Por qué Kaggle y no Colab

Colab prohíbe en sus términos el control remoto por SSH, los proxies remotos y
ofrecer servicios web sobre sus runtimes. Kaggle es distinto: el uso previsto es
justamente lanzar notebooks que corren solos en background, y su API oficial
(`kaggle kernels push`) hace exactamente eso. No hay SSH ni túnel.

| | Colab | Kaggle |
|---|---|---|
| Notebook abierto | obligatorio | **no** |
| Automatizable por API | no | **sí** (`kernels push`) |
| GPU gratis | limitada y no headless | **30 h/semana** (45 con Google AI Pro) |
| Añade gasto | no | no |

## Si ya pagas Google AI Pro

Desde septiembre de 2026 Colab está incluido en los planes Google AI. Ese plan da
**200 compute units al mes** y "expanded access to powerful accelerators and longer
runtimes" (texto literal de la página de planes de Google One).

Lo que **no** incluye es la ejecución en segundo plano. El blog oficial de Google
lo dice solo para Ultra: *"Google AI Ultra subscribers also unlock uninterrupted
background execution and Premium GPU access, so long training runs can finish
without keeping a browser tab open."*

Consecuencia práctica: con Google AI Pro puedes usar GPUs grandes (T4, L4, A100,
G4) pero cerrando el navegador se corta la sesión. Para eso está este kernel de
Kaggle, y para lo demás está `integrations/colab/`, que usa esa cuota con más
hardware pero con la pestaña abierta.

## Cuota y límites

- **30 h de GPU por semana** (45 si la cuenta tiene Google AI Pro / Colab de pago).
- Sesión: hasta 12 h en CPU/GPU y 9 h en TPU. Una separación tarda minutos.
- Requiere **verificación de teléfono** en la cuenta (Profile > Settings).
- Aceleradores gratis: T4 x2 (16 GB cada una) o P100 (16 GB). **No hay G4.**
- Los checkpoints **se vuelven a descargar en cada ejecución**: no hay caché entre
  kernels. Son varios cientos de MB por modelo.
- Un kernel privado por modo. El modo se elige con `mode.txt` dentro del dataset.

## Flujo completo, por línea de comandos

```sh
pip install kaggle
# token en ~/.kaggle/kaggle.json (API token de kaggle.com/settings)

# 1. Preparar el audio de entrada. Un directorio por trabajo.
mkdir -p trabajo/entrada
cp cancion.mp3 trabajo/entrada/
echo instrumental > trabajo/entrada/mode.txt   # o: echo chorus
#    dataset-metadata.json va en la misma carpeta (plantilla abajo).

# 2. Publicar el audio como dataset privado.
#    El `id` debe coincidir con dataset_sources de kernel-metadata.json.
kaggle datasets create -p trabajo/entrada            # primera vez
kaggle datasets version -p trabajo/entrada -m "otra cancion"   # siguientes

# 3. Lanzar el kernel (corre solo, puedes cerrar la terminal).
cp integrations/kaggle/kernel-metadata.json trabajo/
cp integrations/kaggle/separar.py trabajo/
kaggle kernels push -p trabajo

# 4. Esperar y descargar el MP3.
kaggle kernels status TU_USUARIO_KAGGLE/shirley-separar
kaggle kernels output TU_USUARIO_KAGGLE/shirley-separar -p salida
```

El MP3 queda en `salida/output.mp3`. Se sube a la app con la UI normal de pistas
(`POST /api/presentations/{id}/tracks`, hasta 101 MB), o arrastrándolo al show.

`dataset-metadata.json`, junto al audio:

```json
{
  "title": "Shirley Audio",
  "id": "TU_USUARIO_KAGGLE/shirley-audio",
  "licenses": [{ "name": "CC0-1.0" }]
}
```

Ventaja de actualizar el mismo dataset con `datasets version`: el `dataset_sources`
del kernel no cambia y no hay que tocar el metadata entre canciones.

## Qué modelos corre

Elegidos por las puntuaciones SDR del propio catálogo de `audio-separator`
(`models-scores.json`), no por recomendaciones de blog.

**Modo `instrumental`** (quitar toda la voz), promedio de 3:

| Archivo | SDR instrumental |
|---|---|
| `model_bs_roformer_ep_317_sdr_12.9755.ckpt` | **17.0** — el mejor medido |
| `melband_roformer_inst_v1e.ckpt` | 15.735 |
| `mel_band_roformer_instrumental_becruily.ckpt` | 15.342 |

Los dos primeros son la referencia del catálogo. `ep_317` se descarga por nombre de
archivo aunque no aparezca con nombre amigable en `models.json`.

**Overlap.** El parámetro que más mueve la calidad después del ensemble es
`--mdxc_overlap`: número de ventanas superpuestas. Más overlap = menos artefactos
de costura, a cambio de 4–6× más pasadas. Por defecto el kernel usa 8; se puede
cambiar con un `overlap.txt` en el dataset. En 16 GB no conviene subirlo mucho más
sin bajar el segmento; en 24 GB o más hay margen.

**Modo `chorus`** (quitar voz líder, conservar coros), promedio de 4:

| Archivo | Nota |
|---|---|
| `bs_roformer_karaoke_frazer_becruily.ckpt` | el que ya usa el pipeline actual |
| `bs_roformer_karaoke_anvuew.ckpt` | el que ya usa el pipeline actual |
| `mel_band_roformer_karaoke_gabox_v2.ckpt` | V2, más nueva que la V1 en uso |
| `mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt` | único karaoke con score (8.59 voc / 14.29 inst) |

Por qué ensemble y no un solo modelo: ningún modelo gana en todo. Promediar
stems reduce artefactos y bleed a costa de tiempo de GPU, y aquí el tiempo es
gratis. Es el mismo criterio que ya aplica `AVG_WAVE` en el pipeline de coros.

## Sobre "máxima calidad"

Los números de `models-scores.json` **no son todos comparables**: unas medias
salen de 40 pistas y otras de 3, 11 o 12. `Kim FT by unwa` marca 11.53 en vocals
con 12 pistas, mientras `Viperx-1296` marca 11.63 con 40. Dentro del conjunto de
40 pistas, el mejor vocals descargable por el CLI es `Duality V2` (10.62).

`PolarFormer` no existe en el catálogo (cero ocurrencias): por eso el modo
instrumental de Shirley Show, que invoca `polarformer`, está roto.

`BS-Roformer-Viperx-1296/1297` sí están disponibles, con los nombres
`model_bs_roformer_ep_368_sdr_12.9628.ckpt` y
`model_bs_roformer_ep_317_sdr_12.9755.ckpt`. No aparecen con nombre amigable en
`models.json`, pero `--model_filename` los descarga igual. Son los mejores:
12.9 dB voces / 17.0 dB instrumental.

## Alternativa si algún día quieres automatizarlo del todo

`python-audio-separator` trae `audio_separator/remote/deploy_modal.py` y
`Dockerfile.runpod-cuda12`. Modal da 30 USD/mes de cómputo gratis por segundo
(requiere tarjeta: si te pasas, cobra). RunPod cobra ~0.16 USD/h en A5000.
Ambos son legítimos para uso automatizado, a diferencia de Colab.
