-- Esquema de la base de datos (SQLite). Igual al de la versión Node (datos compatibles).

CREATE TABLE IF NOT EXISTS usuarios (
  id       INTEGER PRIMARY KEY AUTOINCREMENT,
  usuario  TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS eventos (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  nombre           TEXT NOT NULL,
  fecha_hora       TEXT NOT NULL,
  lugar            TEXT NOT NULL,
  aforo            INTEGER,
  tipos_de_entrada TEXT NOT NULL DEFAULT '[]',
  finalizado       INTEGER NOT NULL DEFAULT 0,  -- 1 = finalizado manualmente (archivado)
  imagen           TEXT,                        -- nombre del archivo del afiche (en MEDIA_DIR)
  fecha_creacion   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS entradas (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  evento_id        INTEGER NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
  nombre_comprador TEXT NOT NULL,
  contacto         TEXT,
  tipo_entrada     TEXT NOT NULL,
  precio           REAL NOT NULL,
  medio_pago       TEXT NOT NULL,
  estado_pago      TEXT NOT NULL DEFAULT 'Pendiente',
  codigo_qr        TEXT NOT NULL UNIQUE,
  estado_checkin   TEXT NOT NULL DEFAULT 'Pendiente',
  hora_ingreso     TEXT,
  fecha_registro   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entradas_evento ON entradas(evento_id);
CREATE INDEX IF NOT EXISTS idx_entradas_codigo ON entradas(codigo_qr);
