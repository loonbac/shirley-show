use rusqlite::Connection;
use shirley_show::infra::sqlite::{init_pool, migrate_db, migrate_pool};
use tempfile::NamedTempFile;

#[test]
fn test_sqlite_pragmas_and_migration_tables() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let conn = pool.get().expect("get conn from pool");

    let foreign_keys: i32 = conn
        .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
        .expect("query foreign_keys");
    assert_eq!(foreign_keys, 1, "foreign_keys should be ON (1)");

    let busy_timeout: i32 = conn
        .query_row("PRAGMA busy_timeout;", [], |row| row.get(0))
        .expect("query busy_timeout");
    assert_eq!(busy_timeout, 5000, "busy_timeout should be 5000 ms");

    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .expect("query journal_mode");
    assert_eq!(
        journal_mode.to_lowercase(),
        "wal",
        "journal_mode should be WAL"
    );

    // Check table creation
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;")
            .expect("prepare select");
        stmt.query_map([], |row| row.get(0))
            .expect("query_map")
            .map(|r| r.expect("row"))
            .collect()
    };

    assert!(tables.contains(&"schema_migrations".to_string()));
    assert!(tables.contains(&"presentations".to_string()));
    assert!(tables.contains(&"show_items".to_string()));
    assert!(tables.contains(&"pending_audio_deletions".to_string()));
}

#[test]
fn test_pragmas_persist_across_multiple_connections() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let conn1 = pool.get().expect("conn1");
    let conn2 = pool.get().expect("conn2");

    let fk1: i32 = conn1
        .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
        .unwrap();
    let fk2: i32 = conn2
        .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk1, 1);
    assert_eq!(fk2, 1);

    let bt1: i32 = conn1
        .query_row("PRAGMA busy_timeout;", [], |r| r.get(0))
        .unwrap();
    let bt2: i32 = conn2
        .query_row("PRAGMA busy_timeout;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(bt1, 5000);
    assert_eq!(bt2, 5000);
}

#[test]
fn test_migration_idempotency() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool first time");
    // Second migration run must succeed idempotently
    migrate_pool(&pool).expect("migrate pool second time");

    let conn = pool.get().expect("conn");
    let count: i32 = conn
        .query_row(
            "SELECT count(*) FROM schema_migrations WHERE version = 1;",
            [],
            |row| row.get(0),
        )
        .expect("schema version query");
    assert_eq!(count, 1);
}

#[test]
fn test_legacy_ticket_schema_is_upgraded_to_allow_empty_contact() {
    let mut conn = Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch(
        "
        PRAGMA foreign_keys=ON;
        CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
        INSERT INTO schema_migrations VALUES (1, '2026-01-01T00:00:00Z'), (2, '2026-01-01T00:00:00Z');
        CREATE TABLE eventos (id TEXT PRIMARY KEY);
        INSERT INTO eventos VALUES ('evento-1');
        CREATE TABLE interesados (id TEXT PRIMARY KEY);
        CREATE TABLE entradas (
          id TEXT PRIMARY KEY,
          evento_id TEXT NOT NULL REFERENCES eventos(id) ON DELETE CASCADE,
          interesado_id TEXT NULL REFERENCES interesados(id) ON DELETE SET NULL,
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
          CHECK ((estado_checkin = 'No ingresó' AND hora_ingreso IS NULL) OR (estado_checkin = 'Ingresó' AND hora_ingreso IS NOT NULL))
        );
        INSERT INTO entradas VALUES ('existing', 'evento-1', NULL, 'Ana', '999', 'General', 2000, 'Yape', 'Pagado', 'qr-existing', 'token-existing', 'No ingresó', NULL, '2026-01-01', '2026-01-01');
        ",
    )
    .expect("create legacy ticket schema");

    migrate_db(&mut conn).expect("upgrade legacy schema");

    conn.execute(
        "INSERT INTO entradas (id, evento_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, fecha_creacion, fecha_actualizacion)
         VALUES ('no-phone', 'evento-1', 'Sin telefono', '', 'General', 0, 'Cortesía', 'Pagado', 'qr-no-phone', 'token-no-phone', 'No ingresó', '2026-01-01', '2026-01-01')",
        [],
    )
    .expect("an empty ticket contact is accepted after migration");

    let existing: String = conn
        .query_row("SELECT contacto FROM entradas WHERE id = 'existing'", [], |row| row.get(0))
        .expect("existing ticket preserved");
    assert_eq!(existing, "999");
}
