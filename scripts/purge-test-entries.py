"""Borra las entradas de prueba del evento del dueno. NO las reales.

Uso, contra la base de produccion:

    docker cp scripts/purge-test-entries.py panel-app-shir:/tmp/purge.py
    docker exec panel-app-shir python3 /tmp/purge.py            # en seco (por defecto)
    docker exec -e DRY_RUN=0 panel-app-shir python3 /tmp/purge.py   # borra

`DRY_RUN` es una variable de entorno: va en `docker exec -e`, no en el `python3`.

Antes de ejecutar: anotar el md5 del fichero que se copia, y las filas antes y
despues. Una operacion irreversible sin acta no prueba que se ejecuto lo que se
creia ejecutar.

SEIS CONDICIONES, CUATRO PUNTOS DE ABORTO. Las guardas 5 y 6 comparten una sola
consulta y un solo `raise`, asi que contar `raise SystemExit` da 4 y contar
condiciones da 6. Las dos cifras son correctas con unidades distintas; decirlo
evita que alguien cuente y crea que falta una.

Selecciona por nombre exacto, no por patron: un LIKE con '%prueba%' borraria
tambien a cualquier comprador real que se llame igual en el futuro.

Uso:
    DRY_RUN=1  solo lista (por defecto)
    DRY_RUN=0  borra
"""
import os, sqlite3

DB = "/app/data/shirley-show.sqlite3"
EVENTO = "6c6d16b3-6672-4111-a00d-28c5ce0565b8"

# Exactos, medidos: los seis creados el 2026-11-05 entre 11:11:08 y 11:11:53.
# Las dos ventas reales son del 2026-11-01 y no aparecen aqui.
PRUEBA = ["prueba", "prueba2", "orueba3", "asdasdasd", "asdasdasdasdas", "12312312312qweqweqweq"]
REALES = ["Mayela Salinas", "Elizabeth Sallo Roman"]

dry = os.environ.get("DRY_RUN", "1") != "0"
c = sqlite3.connect(DB)

marcas = ",".join("?" * len(PRUEBA))
filas = c.execute(
    f"SELECT id, nombre_comprador, estado_checkin, fecha_creacion "
    f"FROM entradas WHERE evento_id=? AND nombre_comprador IN ({marcas})",
    [EVENTO] + PRUEBA,
).fetchall()

print(f"{'A BORRAR' if not dry else 'SECO'}: {len(filas)} entradas")
for f in filas:
    print("  ", f[1], "|", f[2], "|", f[3][:16])

# Guardas: si algo no cuadra, no se borra nada.
if len(filas) != len(PRUEBA):
    raise SystemExit(f"ABORTADO: esperaba {len(PRUEBA)} y encontre {len(filas)}")

for nombre in REALES:
    r = c.execute(
        "SELECT COUNT(*) FROM entradas WHERE evento_id=? AND nombre_comprador=?",
        (EVENTO, nombre),
    ).fetchone()[0]
    if r != 1:
        raise SystemExit(f"ABORTADO: la entrada real de {nombre} no esta donde deberia")

antes = c.execute("SELECT COUNT(*) FROM entradas WHERE evento_id=?", (EVENTO,)).fetchone()[0]
print(f"\nentradas del evento antes: {antes}")


# GUARDA EXTRA (auditoria de la otra sesion): si alguna de las seis ya se uso,
# borrarla destruye el registro de un ingreso real. Abortar sin tocar nada.
usadas = c.execute(
    f"SELECT nombre_comprador, estado_checkin FROM entradas "
    f"WHERE evento_id=? AND nombre_comprador IN ({marcas}) AND estado_checkin != ?",
    [EVENTO] + PRUEBA + ["No ingresó"],
).fetchall()
if usadas:
    raise SystemExit(f"ABORTADO: hay entradas de prueba YA USADAS: {usadas}. No se borra nada.")

# GUARDAS 5 Y 6: una sola consulta, dos condiciones.
#
# GUARDA 5 -- la lista puede estar incompleta.
# `usadas` solo mira los seis nombres de PRUEBA. Si una entrada de prueba tuviera
# un nombre que no esta en la lista (un typo, una creada despues), pasaria el
# filtro y se borraria igual, aunque ya hubiera ingresado a alguien. La condicion
# que de verdad protege es complementaria: NINGUNA entrada ingresada del evento
# puede desaparecer con este DELETE. Por eso esta consulta NO filtra por nombre.
#
# GUARDA 6 -- el hecho, no su etiqueta.
#
# El esquema declara hoy:
#   CHECK ((estado_checkin='No ingresó' AND hora_ingreso IS NULL)
#       OR (estado_checkin='Ingresó'   AND hora_ingreso IS NOT NULL))
# asi que una fila ingresada siempre trae `hora_ingreso`. Si una migracion futura
# relajara ese CHECK, una fila podria quedar con estado 'No ingresó' y hora puesta:
# la guarda por estado la dejaria pasar, y borrarla destruiria un ingreso real.
# Se comprueban LAS DOS fuentes: el estado declarado y el hecho registrado.
ingresadas = c.execute(
    "SELECT nombre_comprador, estado_checkin, hora_ingreso FROM entradas "
    "WHERE evento_id=? AND (estado_checkin != ? OR hora_ingreso IS NOT NULL)",
    (EVENTO, "No ingresó"),
).fetchall()
if ingresadas:
    raise SystemExit(
        f"ABORTADO: el evento tiene {len(ingresadas)} entrada(s) ya ingresada(s): "
        f"{ingresadas}. Este script solo borra entradas sin usar; si alguna de estas "
        f"es de prueba, borrala a mano y con acta."
    )

if dry:
    print("seco: ninguna entrada del evento esta ingresada.")
    print("seco: no se borro nada. DRY_RUN=0 para ejecutar.")
else:
    c.execute(
        f"DELETE FROM entradas WHERE evento_id=? AND nombre_comprador IN ({marcas})",
        [EVENTO] + PRUEBA,
    )
    c.commit()
    despues = c.execute("SELECT COUNT(*) FROM entradas WHERE evento_id=?", (EVENTO,)).fetchone()[0]
    print(f"entradas del evento despues: {despues}")
    for nombre in REALES:
        r = c.execute(
            "SELECT nombre_comprador, estado_checkin FROM entradas WHERE evento_id=? AND nombre_comprador=?",
            (EVENTO, nombre),
        ).fetchone()
        print("  intacta:", r)
