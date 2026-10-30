# Colab separation backend

Reemplazar el backend de separación vocal (SSH a la PC de Joshua) por Google Colab, encendiendo la GPU solo mientras hay trabajo y apagándola en cuanto termina.

## Specs

S1. El backend de separación vocal en producción es Google Colab. No se usa la PC de Joshua ni ningún fallback local para separar.

S2. La GPU se enciende al iniciar la primera separación en curso y se apaga cuando termina la última. Nunca queda encendida después de un éxito, un error, un timeout o un pánico. Si el proceso muere a mitad, la sesión remota se apaga igual.

S3. El resultado se transfiere cifrado y se verifica por SHA-256 y por duración antes de apagar la GPU. Si la verificación falla, la separación falla y la GPU se apaga igual.

S4. Las credenciales viven en un token.json con access_token, refresh_token y expiry, en una ruta configurable. Si falta el archivo o el token no se puede renovar, la separación falla con un error claro sin encender la GPU.

S5. El modo instrumental usa el ensemble de cuatro modelos: model_bs_roformer_ep_317_sdr_12.9755.ckpt, model_bs_roformer_ep_368_sdr_12.9628.ckpt, melband_roformer_inst_v1e.ckpt y mel_band_roformer_instrumental_becruily.ckpt, con overlap 12 y promedio por muestra. El modo coros usa el ensemble de karaoke ya definido en integrations/colab/separar.ipynb.

S6. Si Colab no está disponible o la cuota está agotada, la separación falla con un error claro. No hay fallback a la PC de Joshua.

S7. Se mantienen el límite de 10 minutos de audio y la exclusión mutua existente: una segunda separación concurrente devuelve GpuBusy.

## Tasks

- T1 (S4) — Configuración y token de Colab. src/infra/vocal/colab_vocal_separator_config.rs
- T2 (S1, S2) — Adapter ColabVocalSeparator con ciclo de vida encendido/apagado garantizado. src/infra/vocal/colab_vocal_separator.rs
- T3 (S3, S5) — Transporte cifrado y script remoto de ensemble. src/infra/vocal/colab_remote_script.rs
- T4 (S1, S6, S7) — Composición en app_state y retirada del camino SSH. src/state/app_state.rs
- T5 (S1–S7) — Tests. tests/colab_vocal_separator_tests.rs
- T6 (S4) — Documentación y .env.example.

## Log

L1 (petición del usuario, literal): "si, el backend de colab si o si debe hacer la separacion, asegurarse de que se prenda y se apague justo cuando se necesite y ya no se necesite"

L2 (hallazgo verificado): colab download trunca o devuelve 0 bytes con exit code 0; colab exec descarta el output de más de ~16 KB; colab upload funciona con archivos grandes. El puerto 8080 del runtime lo ocupa el proxy propio de colab-cli. El transporte que funciona es cifrar en el runtime (AES-256-CBC, PBKDF2), subir a litterbox.catbox.moe, descargar con curl y descifrar, verificando SHA-256.

L3 (medición): 2:38.8 de audio con el ensemble de 4 modelos y overlap 12 tardó 5m 11s en una G4. Coste aproximado 1 CCU por canción.
