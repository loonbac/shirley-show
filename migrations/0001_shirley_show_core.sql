CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS presentations (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS show_items (
  id TEXT PRIMARY KEY,
  presentation_id TEXT NOT NULL
    REFERENCES presentations(id) ON DELETE CASCADE,
  position INTEGER NOT NULL CHECK (position >= 0),
  kind TEXT NOT NULL CHECK (kind IN ('track', 'lyrics', 'note')),
  title TEXT,
  body TEXT,
  audio_key TEXT UNIQUE,
  media_type TEXT,
  byte_size INTEGER CHECK (byte_size IS NULL OR byte_size >= 0),
  created_at TEXT NOT NULL,
  UNIQUE (presentation_id, position),
  CHECK (
    (kind = 'track' AND length(trim(title)) > 0 AND body IS NULL
      AND audio_key IS NOT NULL AND media_type IS NOT NULL AND byte_size IS NOT NULL)
    OR
    (kind IN ('lyrics', 'note') AND title IS NULL AND length(trim(body)) > 0
      AND audio_key IS NULL AND media_type IS NULL AND byte_size IS NULL)
  )
);

CREATE INDEX IF NOT EXISTS show_items_by_presentation
  ON show_items(presentation_id, position);

CREATE TABLE IF NOT EXISTS pending_audio_deletions (
  audio_key TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT
);
