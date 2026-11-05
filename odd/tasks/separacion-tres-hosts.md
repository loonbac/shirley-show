# Separación en tres hosts

Estado: **propuesta con evidencia**, pendiente de ejecutar. Nada desplegado todavía.

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

## Pendiente de confirmar con el dueño

1. **Los dos emails** exactos que pueden entrar.
2. Si la sesión de 30 días de la puerta es aceptable, o prefiere 8 h con código
   cada turno.

## Lo que no hará esta separación

- No invalida entradas impresas ni enlaces ya compartidos: el QR no lleva host y
  el `ticket_path` se reescribe con el host actual al generarse.
- No elimina la falta de autenticación de `/api`: la mitiga por posición de red y
  por `Host`. Un login propio en la API sigue siendo trabajo aparte, y el dueño
  decidió no hacerlo ahora.
- No arregla el bloqueo del comprador por sí sola: necesita la página pública de
  ticket, que está escrita como prueba en rojo (`tests/public_ticket_page_tests.rs`)
  y pendiente de implementar.
