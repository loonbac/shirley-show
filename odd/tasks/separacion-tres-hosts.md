# Separación en tres hosts

Estado: **ejecutado y verificado en producción** el 2026-11-05. Ver «Acta» al final.

## Por qué tres y no dos

La decisión de Cloudflare Access como único login es correcta: el cliente no tiene
que aprender ninguna contraseña. Pero el panel y la puerta tienen **dueños,
horarios y niveles de riesgo distintos**, y meterlos en el mismo host obliga a una
sesión de ocho horas en la puerta.

| host | quién | auth | por qué así |
|---|---|---|---|
| `shirleyshow.site` | visitantes y compradores | ninguna | es la landing pública; el `scan_token` de 32/64 hex es la credencial de la entrada |
| `panel.shirleyshow.site` | Shirley y el dueño | Access + código al email | sesión de horas; nadie más entra |
| `puerta.shirleyshow.site` | el portero | Access + código, sesión de 30 días | si la sesión caduca a mitad del evento, la puerta deja de escanear |

## Lo que verificé antes de proponerlo

- **El QR impreso no lleva dominio.** Codifica 32 hex (`QrSecret`, `ticket.rs:41`);
  la URL pública usa el `scan_token` completo (`st_` + 64 hex). Son dos secretos
  distintos y ninguno menciona el host. **Separar hosts no invalida ninguna
  entrada impresa ni compartida.**
- **Dos entradas ya vendidas y pagadas** para *"shirley show y sus amigos sesión
  3"* (10 de enero de 2027, evento `6c6d16b3`), con `ticket_path` ya generado. Por
  eso la prioridad es no romper nada de ticketing.
- **El comprador está bloqueado hoy**, antes de cualquier cambio mío:
  `https://shirleyshow.site/#/ticket/<token>` devuelve **302** al login de Access,
  porque el hash nunca llega al servidor y la petición de `/` se intercepta. Hoy
  nadie puede abrir la entrada que pagó. Con Access como login único esto sigue
  igual de roto, así que es parte del trabajo, no un efecto secundario.
- **Un solo endpoint público** existe ya: `GET /api/public/ticket/{scan_token}`.
  Nada más necesita el comprador.
- **`/api/agent/chat` gasta tokens de pago** en cada llamada sin credencial, y las
  28 rutas `/api` no tienen capa de identidad. Verificado por el par con un POST
  real sin token: HTTP 200 y respuesta del modelo.

## Reglas de ruta por host

El aislamiento por `Host` ya está implementado y probado (10 pruebas, incluida la
petición exacta del `HEALTHCHECK`). Se extiende a tres superficies:

Medido desde el propio `door-controller.js` (no supuesto): el modo puerta usa
exactamente **cuatro** funciones de la API.

```
api.getEvent(id)          -> GET  /api/eventos/{id}
api.getEventSummary(id)   -> GET  /api/eventos/{id}/resumen
api.checkInTicket(...)    -> POST /api/entradas/checkin
api.sellTickets(...)      -> POST /api/eventos/{id}/entradas
```

Las dos últimas no son de solo lectura: la puerta **vende entradas en la puerta** y
marca el ingreso. Eso amplía la superficie real y hay que decirlo, porque una
primera versión de esta tabla la dejaba fuera.

| ruta | landing | panel | puerta |
|---|---|---|---|
| `/` | landing | panel | panel (entra directo al modo puerta) |
| `/api/public/ticket/*` | 200 | 200 | 200 |
| `GET /api/eventos/{id}` | 404 | 200 | 200 |
| `GET /api/eventos/{id}/resumen` | 404 | 200 | 200 |
| `POST /api/entradas/checkin` | 404 | 200 | 200 |
| `POST /api/eventos/{id}/entradas` | 404 | 200 | 200 |
| `/api/eventos/{id}/imagen` | 404 | 200 | 200 |
| demás `/api/*` | 404 | 200 | 404 |

La puerta puede escanear, vender en la puerta y cerrar el evento. No puede tocar
presentaciones, repertorio, gastos ni el chat.

**Riesgo declarado**: la puerta escribe (vende y marca ingresos). Una regla por
`Host` mal puesta la deja sin poder vender justo durante el evento, que es cuando
no hay tiempo de arreglarlo. Por eso la lista se escribe desde el código y se fija
con pruebas, no de memoria.

## Access

- `panel.shirleyshow.site` → política **Allow** para dos emails (Shirley y el
  dueño), login **One-time PIN**. Sesión de 8 h.
- `puerta.shirleyshow.site` → misma política, sesión de **30 días**, para que no
  caduque a mitad del evento.
- `shirleyshow.site` → **sin Access**: la landing y la página de ticket son
  públicas por diseño.
- `panel` y `puerta` conservan además el aislamiento por `Host` en la aplicación,
  porque Access es una regla de borde y una mala configuración del borde no debe
  publicar la API.

## Confirmado por el dueño

1. **Los dos emails**: `newlichi2011@gmail.com` y `joshuarosalesmoreno@hotmail.com`.
2. Sesión de la puerta: **30 días**, aceptada.
3. Sesión del panel: **8 h**.
4. `www` **no se necesita**: retirado (ver Acta).

## Lo que no hará esta separación

- No invalida entradas impresas ni enlaces ya compartidos: el QR no lleva host y
  el `ticket_path` se reescribe con el host actual al generarse.
- No elimina la falta de autenticación de `/api`: la mitiga por posición de red y
  por `Host`. Un login propio en la API sigue siendo trabajo aparte, y el dueño
  decidió no hacerlo ahora.
- No arregla el bloqueo del comprador por sí sola: necesita la página pública de
  ticket, que está escrita como prueba en rojo (`tests/public_ticket_page_tests.rs`)
  y pendiente de implementar.

## Acta — 2026-11-05

Ejecutado con el token de Cloudflare que proveyó el dueño. **Nada de esto es
propuesta: es lo que está corriendo.**

### Cloudflare

```
DNS    panel.shirleyshow.site    CNAME -> túnel ae3f41bc…  proxied
       puerta.shirleyshow.site   CNAME -> túnel ae3f41bc…  proxied
       www.shirleyshow.site      BORRADO a petición del dueño (d790179f…)
       media.shirleyshow.site    intacto (otro túnel, b90f923e…)

túnel  ingress -> http://127.0.0.1:65529
         shirleyshow.site · panel · puerta
         catch-all -> http://100.82.97.118:9090   (preservado)

Access app "Shirley Show — Panel"   2c7c714b-c8c9-4429-b218-26e3ac05592f   8h
       app "Shirley Show — Puerta"  213e2ec0-5a67-4250-8daf-7528c8960c8f   720h
       allow: newlichi2011@gmail.com · joshuarosalesmoreno@hotmail.com
       one_time_pin · auto_redirect_to_identity · launcher oculto
       RETIRADAS las dos apps que cubrían la landing:
         5072beeb… (shirleyshow.site, 720h) · 55a100c5… (www, 720h)
         respaldo previo en /tmp/cf-backup/{app,policies}-<id>.json
```

### Contenedor

`shirley-show:14290bf` — **sin build nuevo**: el binario ya soportaba las tres
variables (`config.rs:206 new_three`).

```
SHIRLEY_PUBLIC_BASE_URL=https://shirleyshow.site
SHIRLEY_PANEL_BASE_URL=https://panel.shirleyshow.site
SHIRLEY_DOOR_BASE_URL=https://puerta.shirleyshow.site
started 2026-11-05T15:54:27.197898194Z · restarts 0 · healthy · 10 variables
```

koro-panel las añadió al `env_file`, no con `-e`, para que sobrevivan al redeploy.

### Verificación

Desde internet:
```
https://shirleyshow.site/         200   landing pública, sin Access
https://www.shirleyshow.site/     NXDOMAIN (4 vías: 1.1.1.1 · 8.8.8.8 · 9.9.9.9 · DoH)
https://panel.shirleyshow.site/   302   -> cloudflareaccess.com
https://puerta.shirleyshow.site/  302   -> cloudflareaccess.com
GET /api/public/ticket/<real>     200   qr_svg=True · codigo_qr=null
GET /#/ticket/<codigo_qr>         200
```

Corte en origen contra `127.0.0.1:65529`:
```
shirleyshow.site        / 200 · /panel 404 refused=1 · checkin 404 refused=1
panel.shirleyshow.site  / 200 · /panel 200 · /api/presentations 200
puerta.shirleyshow.site / 200 · /panel 404 refused=1 · checkin permitido
desconocido.test        / 200 (landing estática) · todo lo demás 404 refused=1
```

**El comprador lee su entrada y no puede marcar ingresos.**

### Trampas medidas, para no repetirlas

- **Access se retira DESPUÉS del split, nunca antes.** Con `panel_host.is_empty()`
  el binario devuelve `Application` para todos los hosts (`config.rs:521`);
  retirar el borde primero habría abierto `/api/agent/chat` y `/panel`.
- **Un `HTTP 000` en un CNAME recién creado es «todavía no», no «roto».**
- **El DNS se mide antes que el HTTP.** Un `530`/`1016` sobre un nombre que ya no
  resuelve es el resolver, no Cloudflare. `--resolve` a una IP del borde **fabrica
  el `Host` que el DNS niega**: el borde contesta por cualquier nombre.
- **`getent hosts` mide la caché de esta máquina, no el DNS público.** Aquí el
  resolver es Tailscale (`100.100.100.100`). Un registro no cambia dos veces; una
  entrada de caché sí.
- **Un 404 por token inexistente no distingue «refusado» de «no encontrado».**
  La cabecera `x-surface-refused: 1` sí.

### Pendiente

Sólo el issue **#72**, después del evento.
