-- A ticket sale can be recorded without a telephone number. SQLite cannot drop
-- the old CHECK constraint in place, so rebuild only the entradas table while
-- preserving every existing ticket and its indexes.
CREATE TABLE entradas_nueva (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL
    REFERENCES eventos(id) ON DELETE CASCADE,
  interesado_id TEXT NULL
    REFERENCES interesados(id) ON DELETE SET NULL,
  nombre_comprador TEXT NOT NULL CHECK (length(trim(nombre_comprador)) > 0),
  contacto TEXT NOT NULL,
  tipo_entrada TEXT NOT NULL CHECK (length(trim(tipo_entrada)) > 0),
  precio_centimos INTEGER NOT NULL CHECK (precio_centimos >= 0),
  medio_pago TEXT NOT NULL CHECK (medio_pago IN ('Yape', 'Efectivo', 'Cortesía')),
  estado_pago TEXT NOT NULL CHECK (estado_pago IN ('Pagado', 'Pendiente')),
  codigo_qr TEXT NOT NULL UNIQUE CHECK (length(trim(codigo_qr)) > 0),
  scan_token TEXT NOT NULL UNIQUE CHECK (length(trim(scan_token)) > 0),
  estado_checkin TEXT NOT NULL DEFAULT 'No ingresó' CHECK (estado_checkin IN ('No ingresó', 'Ingresó')),
  hora_ingreso TEXT NULL,
  fecha_creacion TEXT NOT NULL,
  fecha_actualizacion TEXT NOT NULL,
  CHECK (medio_pago != 'Cortesía' OR (precio_centimos = 0 AND estado_pago = 'Pagado')),
  CHECK (
    (estado_checkin = 'No ingresó' AND hora_ingreso IS NULL)
    OR
    (estado_checkin = 'Ingresó' AND hora_ingreso IS NOT NULL)
  )
);

INSERT INTO entradas_nueva (
  id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada,
  precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token,
  estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion
)
SELECT
  id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada,
  precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token,
  estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion
FROM entradas;

DROP TABLE entradas;
ALTER TABLE entradas_nueva RENAME TO entradas;

CREATE UNIQUE INDEX ux_entradas_codigo_qr ON entradas(codigo_qr);
CREATE UNIQUE INDEX ux_entradas_scan_token ON entradas(scan_token);
CREATE INDEX idx_entradas_evento_creacion ON entradas(evento_id, fecha_creacion);
CREATE INDEX idx_entradas_evento_pago ON entradas(evento_id, estado_pago);
CREATE INDEX idx_entradas_evento_tipo ON entradas(evento_id, tipo_entrada);
