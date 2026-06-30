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
  scan_token       TEXT,  -- token corto que lleva el QR (se resuelve solo en el servidor)
  estado_checkin   TEXT NOT NULL DEFAULT 'Pendiente',
  hora_ingreso     TEXT,
  fecha_registro   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entradas_evento ON entradas(evento_id);
CREATE INDEX IF NOT EXISTS idx_entradas_codigo ON entradas(codigo_qr);
-- El índice de scan_token se crea en migrar() (tras añadir la columna en bases existentes).

-- Fase 1 (invitación): personas contactadas que dijeron que sí de palabra. Sirve para
-- estimar la asistencia (sumar "personas") y planear sillas/aforo antes de cobrar nada.
-- Al cobrarle a una se vuelve una entrada normal (fase 2) y aquí queda como 'Confirmado'.
CREATE TABLE IF NOT EXISTS interesados (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  evento_id       INTEGER NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
  nombre          TEXT NOT NULL,
  contacto        TEXT,
  canal           TEXT,                              -- por dónde se contactó: 'WhatsApp' | 'Messenger'
  personas        INTEGER NOT NULL DEFAULT 1,        -- cuántos vendrían (titular + familia)
  estado          TEXT NOT NULL DEFAULT 'Interesado', -- 'Interesado' | 'Confirmado'
  fecha_creacion  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_interesados_evento ON interesados(evento_id);
