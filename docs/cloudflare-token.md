# El token de Cloudflare para activar los tres hosts

## Qué falta y por qué no lo puedo hacer yo

Hoy los tres hosts están configurados en el código y **nada más**. Medido en el
VPS:

```
/etc/cloudflared/            a6975516-….json, shirleyshow.env, tunnel.env
/etc/cloudflared/config.yml  no existe → el túnel es remoto
cloudflared                 /usr/local/bin/cloudflared
~/.cloudflared/             no existe → sin cert.pem
```

Un túnel gestionado de forma remota se configura **por API**, con un token. Sin
token no hay DNS, ni hostnames nuevos, ni Access. No es una limitación del
repositorio: es que esas tres cosas viven en tu cuenta.

## El token que sirve

Dashboard → My Profile → API Tokens → Create Custom Token.

**Permissions**, las cuatro:

| Tipo | Recurso | Nivel |
| --- | --- | --- |
| Zone | DNS | Edit |
| Zone | Zone | Read |
| Account | Cloudflare Tunnel | Edit |
| Account | Access: Apps and Policies | Edit |

**Zone Resources**: Include → Specific zone → `shirleyshow.site`.

**Account Resources**: Include → tu cuenta.

**TTL**: 24 h basta. Es un trabajo de una tarde, no una credencial permanente.

## Dónde ponerlo

```
/root/.cloudflare/access-api-token    permiso 600
```

Y nada más. No hace falta exportarlo en el entorno del contenedor ni del servicio:
lo lee quien ejecuta la configuración, una vez.

## Qué hará con él, en este orden

El orden no es preferencia: cada paso depende de que exista el anterior.

1. **CNAME** de `panel` y `puerta` al túnel actual.
2. **Public hostnames** en el túnel: `panel.shirleyshow.site` y
   `puerta.shirleyshow.site` → `http://127.0.0.1:65529`.
3. **Dos aplicaciones de Access**, ambas con One-time PIN, las mismas dos
   direcciones de correo autorizadas:
   - `panel.shirleyshow.site` — sesión de **8 h**.
   - `puerta.shirleyshow.site` — sesión de **30 días**. El doorman abre la puerta
     en un móvil y no puede estar reautenticándose en mitad del evento.
4. **Recién entonces** las variables `SHIRLEY_PANEL_BASE_URL` y
   `SHIRLEY_DOOR_BASE_URL` en el contenedor, y reinicio.

Las variables van al final a propósito. Ponerlas antes de que los CNAME existan no
rompe nada —`classify` mira primero si hay división de hosts y, si falta la mitad,
el despliegue queda **inerte**, no cerrado— pero deja el sistema hablando de unos
hosts que todavía no resuelven.

## Verificación, cuando esté hecho

```bash
curl -sI https://panel.shirleyshow.site/   # 302 a Access
curl -sI https://puerta.shirleyshow.site/  # 302 a Access
curl -s  https://shirleyshow.site/         # 200, sin Access
```

Para la fila de contrato, el codigo HTTP **no basta**: sobre una base vacia
`Landing` responde 404 antes de llegar al handler, asi que un host desconocido y
un token inexistente se ven igual. El discriminador es la cabecera:

```bash
curl -s -o /dev/null -D- -H 'Host: desconocido.test' \
  http://127.0.0.1:3001/api/public/ticket/<token> | grep -i x-surface-refused
# esperado: 1   (lo refuso la superficie, no el handler)
```

Y con sesión iniciada, que `panel.shirleyshow.site/` sirva el panel (contiene
`main.js`, no `pres__hero`) y `puerta.shirleyshow.site/` el modo puerta.

## Lo que este token NO autoriza

Es de alcance de zona y de túnel. No toca otras zonas, no cambia el plan, no
publica nada, no borra registros. Si algo pide más permisos que estos cuatro, es
que el plan cambió y hay que hablarlo.
