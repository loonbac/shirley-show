# Separación vocal con Google Colab

La separación vocal corre en una GPU de Google Colab, no en la PC de Joshua. La GPU se enciende cuando llega una separación y se apaga en cuanto termina.

## Requisitos

1. El binario colab (colab-cli) accesible en el PATH del contenedor.
2. Un token OAuth en SHIRLEY_COLAB_TOKEN con access_token, refresh_token y expiry. El refresh_token permite renovarlo sin navegador.

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
2. colab start --gpu <tipo> solo cuando hay trabajo.
3. Subida del audio, ejecución del ensemble, descarga cifrada del resultado.
4. Verificación de SHA-256 y de duración contra lo que reportó el runtime.
5. colab stop siempre: éxito, error o timeout.

Un guard con Drop lanza colab stop si el proceso entra en pánico o se cancela, para que la GPU no quede encendida.

## Coste medido

- Burn rate de una G4: 8.9 CCU/h.
- Canción de 2:39 con ensemble de 4 modelos y overlap 12: 5m 11s, aproximadamente 1 CCU.
- Con 300 CCU: del orden de 290 canciones.

## Límites conocidos

- colab download está roto: trunca o devuelve 0 bytes con exit code 0. No se usa. El resultado viaja cifrado a litterbox.catbox.moe y se descarga con curl.
- colab exec descarta la salida de más de ~16 KB, así que el script remoto imprime una sola línea JSON.
- El puerto 8080 del runtime lo ocupa el proxy de colab-cli.
- Si litterbox no responde desde el nodo de Colab, la separación falla y la GPU se apaga igual.

## Renovar el token

    colab auth

Necesita un navegador y un túnel al puerto que imprima el comando. El refresh_token evita repetirlo mientras siga vigente.
