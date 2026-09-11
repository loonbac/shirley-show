CREATE TABLE IF NOT EXISTS eventos (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL CHECK (length(trim(nombre)) > 0),
  fecha_hora TEXT NOT NULL,
  lugar TEXT NOT NULL CHECK (length(trim(lugar)) > 0),
  tipos_de_entrada TEXT NOT NULL CHECK (json_valid(tipos_de_entrada)),
  finalizado INTEGER NOT NULL DEFAULT 0 CHECK (finalizado IN (0, 1)),
  imagen TEXT NULL,
  fecha_creacion TEXT NOT NULL,
  fecha_actualizacion TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_eventos_estado_fecha
  ON eventos(finalizado, fecha_hora);

CREATE INDEX IF NOT EXISTS idx_eventos_fecha
  ON eventos(fecha_hora);

CREATE TABLE IF NOT EXISTS interesados (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL
    REFERENCES eventos(id) ON DELETE CASCADE,
  nombre TEXT NOT NULL CHECK (length(trim(nombre)) > 0),
  contacto TEXT NOT NULL CHECK (length(trim(contacto)) > 0),
  canal TEXT NOT NULL CHECK (length(trim(canal)) > 0),
  promotora TEXT NOT NULL CHECK (length(trim(promotora)) > 0),
  personas INTEGER NOT NULL CHECK (personas >= 1),
  estado TEXT NOT NULL CHECK (estado IN ('Pendiente', 'Confirmado', 'Cancelado')),
  convertido_at TEXT NULL,
  fecha_creacion TEXT NOT NULL,
  fecha_actualizacion TEXT NOT NULL,
  CHECK (convertido_at IS NULL OR estado = 'Confirmado')
);

CREATE INDEX IF NOT EXISTS idx_interesados_evento_estado
  ON interesados(evento_id, estado);

CREATE INDEX IF NOT EXISTS idx_interesados_evento_creacion
  ON interesados(evento_id, fecha_creacion);

CREATE TABLE IF NOT EXISTS entradas (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL
    REFERENCES eventos(id) ON DELETE CASCADE,
  interesado_id TEXT NULL
    REFERENCES interesados(id) ON DELETE SET NULL,
  nombre_comprador TEXT NOT NULL CHECK (length(trim(nombre_comprador)) > 0),
  contacto TEXT NOT NULL CHECK (length(trim(contacto)) > 0),
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

CREATE UNIQUE INDEX IF NOT EXISTS ux_entradas_codigo_qr
  ON entradas(codigo_qr);

CREATE UNIQUE INDEX IF NOT EXISTS ux_entradas_scan_token
  ON entradas(scan_token);

CREATE INDEX IF NOT EXISTS idx_entradas_evento_creacion
  ON entradas(evento_id, fecha_creacion);

CREATE INDEX IF NOT EXISTS idx_entradas_evento_pago
  ON entradas(evento_id, estado_pago);

CREATE INDEX IF NOT EXISTS idx_entradas_evento_tipo
  ON entradas(evento_id, tipo_entrada);

CREATE TABLE IF NOT EXISTS gastos (
  id TEXT PRIMARY KEY,
  evento_id TEXT NOT NULL
    REFERENCES eventos(id) ON DELETE CASCADE,
  categoria TEXT NOT NULL CHECK (
    categoria IN (
      'Local/Alquiler',
      'Sonido/Luces',
      'Músicos/Personal',
      'Publicidad/Marketing',
      'Bebidas/Catering',
      'Transporte',
      'Otros'
    )
  ),
  monto_centimos INTEGER NOT NULL CHECK (monto_centimos >= 0),
  descripcion TEXT NOT NULL CHECK (length(trim(descripcion)) > 0),
  fecha TEXT NOT NULL CHECK (length(trim(fecha)) > 0),
  fecha_creacion TEXT NOT NULL,
  fecha_actualizacion TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gastos_evento_fecha
  ON gastos(evento_id, fecha);

CREATE INDEX IF NOT EXISTS idx_gastos_evento_categoria
  ON gastos(evento_id, categoria);
