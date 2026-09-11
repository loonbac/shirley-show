# Proposal: Despliegue de Shirley Show con separación vocal remota

**Change:** `shirley-show-deploy`  
**Status:** Proposed  
**Execution mode:** Auto  
**Artifact store:** OpenSpec  
**Delivery strategy:** Ask on risk  
**Review budget:** 400 líneas cambiadas por unidad de revisión  
**Planned slices:** S1 empaquetado y operación; S2 ejecución SSH; S3 energía y UX  
**Discovery status:** El pre-proposal fue confirmado por el orquestador; esta fase no reabre sus decisiones de producto.

## Intent

Desplegar Shirley Show en Coolify sobre `eje-desarrollo`, conservando allí la aplicación y sus datos persistentes, y delegar únicamente la separación vocal que requiere GPU a `nixos-pc` por SSH sobre Tailscale.

El resultado debe permitir que Shirley use el flujo existente de Ressy desde PC o teléfono sin administrar comandos, transferencias ni el ciclo de energía de la máquina GPU. Si la PC de Joshua está apagada, Ressy debe pedir que se encienda, esperar hasta diez minutos y continuar automáticamente cuando esté disponible. Tras un trabajo exitoso, la PC quedará programada para apagarse después de una hora de inactividad.

El despliegue local actual seguirá funcionando sin cambios cuando `JOSHUA_PC_HOST` no esté configurado.

## Product problem

`eje-desarrollo` puede alojar Shirley Show de forma continua, pero no dispone de GPU para ejecutar los pipelines de separación vocal existentes. `nixos-pc` sí tiene una RTX 3060, checkpoints, `karaoke-separator` y `ffmpeg`, aunque no debe permanecer encendida permanentemente.

Sin esta integración, desplegar la aplicación obliga a elegir entre perder la separación vocal, mantener encendida una estación personal o trasladar audio y ejecutar comandos manualmente. Esto rompe el flujo conversacional de Ressy, aumenta el coste operativo y hace que una función ya disponible localmente no sea viable desde la instancia desplegada.

## Target user and situations

La usuaria principal es Shirley, que prepara presentaciones desde una instancia de Shirley Show accesible por PC o teléfono.

Los momentos previstos son:

1. Usar normalmente la biblioteca, las presentaciones y Ressy desde el servidor Coolify, sin depender de que `nixos-pc` esté encendida.
2. Solicitar una separación vocal cuando la PC GPU ya está disponible.
3. Solicitarla cuando la PC está apagada, encenderla manualmente y dejar que la operación continúe sin repetir la petición.
4. Recibir una salida clara si la PC no aparece dentro de diez minutos.
5. Dejar que la PC se apague una hora después del último trabajo exitoso, cancelando ese apagado si llega un trabajo nuevo.
6. Ejecutar desarrollo local sin configurar infraestructura remota.

## Product outcome

Después del cambio:

- Shirley Show corre en una imagen reproducible, no privilegiada y desplegable por Coolify.
- SQLite, audio definitivo y staging sobreviven recreaciones del contenedor mediante un volumen persistente.
- La aplicación desplegada alcanza `nixos-pc` por Tailscale y delega allí los pipelines GPU a través del puerto `VocalSeparator` existente.
- El audio fuente viaja a la PC, el resultado vuelve como MP3 y la aplicación local lo valida antes de tratar la operación como exitosa.
- Una PC temporalmente apagada no causa un fallo inmediato: se sondea con espera acotada y backoff durante un máximo predeterminado de diez minutos.
- Cada conexión disponible cancela un apagado pendiente; cada separación exitosa programa un nuevo apagado a 60 minutos.
- Ressy explica el estado y la política de energía con copy estable, sin exponer IPs, rutas, comandos, claves ni stderr remoto.
- Sin `JOSHUA_PC_HOST`, la composición sigue eligiendo `ProcessVocalSeparator` y conserva el comportamiento local existente.

## Scope

### S1 — Empaquetado Docker y operación en Coolify

- Crear un `Dockerfile` multi-stage con builder Rust y runtime Debian slim.
- Incluir sólo las dependencias necesarias en runtime: `ffmpeg`/`ffprobe`, `yt-dlp`, cliente OpenSSH, certificados, `curl` y `sqlite3`.
- Ejecutar como usuario no root `shirley` con UID/GID 1000.
- Copiar el binario y los assets frontend a `/app`, usando `/app/data` como raíz persistente.
- Incorporar healthcheck HTTP contra `/api/presentations` en el puerto 3000.
- Añadir `.dockerignore` para excluir artefactos locales, datos, secretos y material innecesario del contexto de build.
- Crear `compose.yaml` con volumen `shirley-data:/app/data`, variables de aplicación/Groq/GPU remota y clave SSH montada en sólo lectura.
- Documentar el despliegue en Coolify y la conectividad Tailscale. La ruta preferida es `network_mode: host` cuando el host Coolify pertenece al tailnet; bridge con forwarding o sidecar de Tailscale quedan como alternativas operativas si host networking no está disponible.
- Mantener claves SSH y `GROQ_API_KEY` fuera de la imagen y del repositorio.

### S2 — Adapter de separación vocal por SSH

- Añadir configuración `JOSHUA_PC_*` y seleccionar el adapter remoto sólo cuando `JOSHUA_PC_HOST` tenga valor.
- Implementar `SshVocalSeparator` detrás del puerto `VocalSeparator`; el dominio y el caso de uso no dependerán de SSH.
- Sondear presencia con `ssh ... true`, usando ejecución no interactiva, timeout corto y una espera con backoff acotada a diez minutos por defecto.
- Construir invocaciones mediante argumentos de proceso, no mediante interpolación de entradas en un shell local.
- Crear por operación un directorio remoto aislado bajo `/tmp/shirley-vocal/<op_id>`.
- Transferir el audio fuente con `scp`, ejecutar en `nixos-pc` el pipeline correspondiente de `karaoke-separator` y producir allí un MP3 de 320 kbps.
- Transferir de vuelta sólo el MP3 final, reduciendo el tráfico frente a devolver stems sin comprimir.
- Limpiar el directorio remoto en éxito, error, timeout o cancelación mediante un propietario de ciclo de vida explícito.
- Validar localmente con `ffprobe`, tamaño no nulo y firma de MP3 antes de devolver `SeparatedAudio` exitoso.
- Conservar errores con significado y traducirlos a mensajes seguros para Ressy, sin filtrar detalles de infraestructura.
- Verificar el adapter con binarios stub de `ssh` y `scp`; las pruebas automatizadas no dependerán de Tailscale, la PC real ni una GPU.

### S3 — Ciclo de energía y feedback de Ressy

- Ejecutar `sudo shutdown -c` al conectar para cancelar cualquier apagado pendiente antes de iniciar trabajo.
- Ejecutar `sudo shutdown +60 "Shirley Show idle shutdown"` únicamente después de completar y validar una separación.
- Documentar la regla NixOS `NOPASSWD` limitada al binario `shutdown` para el usuario SSH configurado. Su aplicación en `configuration.nix` corresponde al operador de `nixos-pc`, no al contenedor.
- Añadir al prompt de Ressy las reglas de espera, abandono y apagado automático.
- Mostrar el indicador frontend exacto `esperando la PC de Joshua…` sólo cuando la fuente de verdad indique que una separación remota está esperando disponibilidad.
- Cubrir transiciones de estado para que el indicador pendiente vuelva a su valor normal en éxito, error, timeout y desmontaje.

## Product rules and invariants

### Selección y disponibilidad

1. `JOSHUA_PC_HOST` configurado selecciona `SshVocalSeparator`; ausente selecciona `ProcessVocalSeparator`.
2. La aplicación no intenta Wake-on-LAN. El encendido de `nixos-pc` siempre es manual.
3. Un probe individual no debe bloquear indefinidamente; la espera total de disponibilidad es de diez minutos por defecto.
4. Si la PC aparece antes del límite, la misma operación continúa sin pedir a Shirley que repita la solicitud.
5. Si no aparece, la operación termina como fallo explícito y no anuncia una pista creada.
6. El presupuesto HTTP/agente debe abarcar de forma coherente la espera de disponibilidad y la posterior separación. El diseño no puede permitir que el deadline actual de 600 segundos invalide silenciosamente la promesa de esperar diez minutos.

### Transferencia y resultado

1. Cada operación usa un identificador remoto opaco y aislado; no reutiliza directorios de otras solicitudes.
2. Sólo se transfieren los archivos necesarios y nunca claves, configuración o paths internos al modelo o al cliente.
3. El resultado remoto no es éxito hasta que el MP3 copiado haya sido validado localmente.
4. Ressy sólo confirma éxito cuando el caso de uso existente ha recibido un resultado válido.
5. Los artefactos remotos se limpian en todas las salidas razonablemente recuperables.
6. Los pipelines, modos, límites de duración, serialización de GPU y persistencia de pista definidos por la separación vocal existente permanecen sin cambios.

### Energía

1. Toda conexión aceptada para un trabajo cancela primero el apagado pendiente.
2. Sólo un trabajo exitoso y validado programa `shutdown +60`.
3. Un trabajo posterior vuelve a cancelar y reprogramar el temporizador, por lo que la hora se cuenta desde el último éxito.
4. La automatización no recibe privilegios generales: `NOPASSWD` se limita a `shutdown`.
5. Fallar al gestionar energía no puede convertir un audio inválido en éxito ni ocultar el resultado principal; el error y su tratamiento deberán quedar explícitos en diseño.

### Copy contractual de Ressy

Los siguientes textos se tratan como copy contractual y deberán verificarse literalmente:

- **Espera:** `Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo`.
- **Indicador:** `esperando la PC de Joshua…`.
- **Aviso de energía:** `Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.`
- **Abandono a los diez minutos:** `No pude conectarme con la PC de Joshua después de 10 minutos. Cuando esté encendida, vuelve a pedirme la separación.`

El copy puede ir acompañado de contexto conversacional, pero no debe prometer Wake-on-LAN, progreso porcentual ni éxito antes de validar el archivo.

## Affected areas

| Área | Impacto previsto |
|---|---|
| Raíz de despliegue | `Dockerfile`, `.dockerignore`, `compose.yaml` y documentación operativa. |
| Configuración | Variables `JOSHUA_PC_*`, rutas de datos/frontend y composición local/remota. |
| `src/infra/vocal/` | Nuevo adapter SSH, ejecución de procesos, transferencia, presencia, validación y cleanup remoto. |
| `src/state/app_state.rs` | Selección explícita de `ProcessVocalSeparator` o `SshVocalSeparator`. |
| Puerto/errores de separación | Sólo los deltas mínimos necesarios para representar indisponibilidad remota sin filtrar detalles. |
| Agente Ressy | Prompt y traducción segura de espera, timeout y política de energía. |
| Frontend chat | Estado/label cualitativo para espera remota y restauración del indicador. |
| Pruebas Rust y JS | Configuración, composición, adapter hermético, energía, copy exacto y transiciones UI. |
| `nixos-pc` | Cambio manual documentado de sudoers en `configuration.nix`; no forma parte del repositorio desplegado. |

No se proponen migraciones de SQLite ni cambios al contrato persistido de presentaciones o pistas.

## Non-goals

- Encender `nixos-pc` mediante Wake-on-LAN, Tailscale u otro mecanismo remoto.
- Soportar múltiples PCs, balanceo de workers, múltiples GPUs o colas distribuidas.
- Procesar o transferir video.
- Mover Groq, SQLite, la API o el frontend a la PC GPU.
- Reescribir los pipelines de separación vocal ya aprobados.
- Añadir progreso porcentual o una infraestructura general de streaming sólo para esta espera.
- Guardar secretos en la imagen, el repositorio, logs o respuestas del agente.
- Automatizar desde este repositorio el rebuild de NixOS o ampliar privilegios sudo.

## Delivery slices and review budget

| Slice | Contenido | Forecast de exploración | Gate |
|---|---|---:|---|
| S1 | Dockerfile, compose, ignore y guía de despliegue/Tailscale | ~220 líneas | Unidad independiente. |
| S2 | Config, adapter SSH, presencia, ida/vuelta, cleanup, validación y pruebas | ~380 líneas | Cercana al límite; recalcular antes de aplicar. |
| S3 | Cancelación/programación de apagado, sudoers, prompt Ressy y UX | ~190 líneas | Unidad independiente. |

Cada slice debe mantener juntos código, pruebas y documentación de su comportamiento. Con estrategia `ask-on-risk`, si el forecast real de una unidad supera 400 líneas se pausa para que el usuario elija una cadena más fina; no se infiere una excepción de tamaño.

## Risks and mitigations

| Riesgo | Impacto | Mitigación propuesta |
|---|---|---|
| El deadline de separación actual es 600 s y la espera puede consumir 600 s antes de empezar GPU. | La aplicación abandona antes de cumplir el flujo prometido o corta un trabajo ya iniciado. | Resolver en diseño un deadline total explícito y probar el caso “enciende cerca del límite”; no reutilizar ciegamente un único timeout. |
| El frontend no tiene hoy una señal de streaming sobre el probe SSH. | El indicador podría afirmar que la PC está apagada cuando no lo está. | Definir una fuente de verdad mínima para el estado remoto; no inferirlo sólo por intención ni introducir SSE sin necesidad. |
| `network_mode: host` interactúa con el routing y publicación de puertos de Coolify. | Healthcheck accesible pero proxy/ruta pública incorrectos, o conflicto de puerto. | Validar host networking en `eje-desarrollo`; mantener bridge forwarding y sidecar como rutas documentadas de contingencia. |
| `StrictHostKeyChecking=accept-new` confía en la primera clave observada. | Riesgo de suplantación durante el primer contacto. | Preaprovisionar o verificar `known_hosts` en operación cuando sea viable y documentar la huella esperada. |
| Clave SSH o variables terminan en build, logs o respuestas. | Compromiso de `nixos-pc` o Groq. | Montajes read-only, BuildKit/variables de runtime, permisos mínimos y redacción de errores. |
| Cleanup remoto falla por desconexión. | Acumulación en `/tmp` y consumo de disco. | Guard best-effort, directorios por UUID y limpieza operativa de residuos antiguos documentada. |
| Un timer de apagado compite con un nuevo trabajo. | La PC se apaga durante transferencia o GPU. | `shutdown -c` después de presencia y antes de transferir; pruebas de orden exacto. |
| UID 1000 no coincide con ownership del volumen o secreto. | El contenedor no puede leer la clave o escribir SQLite/audio. | Documentar permisos y validar escritura/lectura como usuario `shirley` antes del rollout. |
| S2 rebasa el presupuesto por adapter y pruebas. | Revisión fatigante y mayor probabilidad de defectos. | Reforecast antes de apply y activar `ask-on-risk` para dividir, sin aceptar `size:exception` implícitamente. |

## Rollback plan

1. **Aplicación:** volver a la última imagen conocida en Coolify y conservar el volumen `shirley-data`; el cambio no requiere migración de esquema.
2. **Modo remoto:** eliminar o dejar vacío `JOSHUA_PC_HOST` y reiniciar para recuperar `ProcessVocalSeparator` sin retirar el resto del despliegue.
3. **Red:** retirar host networking y usar la configuración previa del servicio; la GPU remota puede permanecer deshabilitada mientras se corrige Tailscale.
4. **Credenciales:** desmontar y revocar la clave SSH dedicada si existe sospecha de exposición.
5. **Energía:** retirar la regla `NOPASSWD` de `nixos-pc` después de desactivar el adapter remoto; cancelar manualmente cualquier timer pendiente con autorización operativa.
6. **Slices:** S3 puede revertirse sin retirar el empaquetado; S2 puede revertirse preservando S1. Los cambios deberán evitar contratos acoplados que impidan este rollback gradual.
7. **Datos:** antes de una recreación destructiva del volumen, respaldar `shirley-show.sqlite3` y `/app/data/audio`; nunca eliminar el volumen como paso rutinario de rollback.

## Success criteria

1. La imagen se construye en multi-stage, arranca como UID 1000 y sirve frontend/API desde el runtime slim.
2. El healthcheck de `/api/presentations` pasa y Coolify puede enrutar tráfico a la aplicación.
3. Presentaciones, SQLite, audio y staging persisten después de recrear el contenedor con el mismo volumen.
4. La imagen y su historial no contienen `GROQ_API_KEY`, claves SSH ni archivos de datos locales.
5. Sin `JOSHUA_PC_HOST`, las pruebas existentes y una separación local conservan el comportamiento previo.
6. Con configuración remota y PC disponible, una separación completa transfiere entrada, ejecuta el pipeline correcto, devuelve MP3, valida localmente y limpia el directorio remoto.
7. Si la PC aparece durante la ventana de diez minutos, la operación continúa automáticamente; si no aparece, termina con el copy contractual de abandono y sin crear pista.
8. El copy de espera, aviso de una hora, abandono e indicador coincide literalmente con esta propuesta.
9. Un nuevo trabajo ejecuta `shutdown -c` antes de transferir; un éxito programa `shutdown +60`; un fallo de separación no programa un nuevo apagado.
10. Las pruebas herméticas cubren conexión inmediata, varios probes antes de conexión, timeout, orden de energía, fallos de `scp`/SSH, cleanup y resultado MP3 inválido.
11. Los errores visibles no revelan IPs, usuario SSH, paths remotos, comandos, claves ni stderr crudo.
12. La documentación permite al operador configurar Coolify, Tailscale, permisos del volumen, clave SSH y sudoers sin incorporar secretos al repositorio.
13. Cada unidad entregada permanece dentro de 400 líneas o pasa por el gate `ask-on-risk` antes de excederlo.

## Next phase

Crear specs delta en Given/When/Then para despliegue/configuración, ejecución SSH y chat/feedback remoto. El diseño posterior deberá cerrar especialmente la composición de deadlines y la fuente de verdad del indicador de espera antes de producir tareas de implementación.
