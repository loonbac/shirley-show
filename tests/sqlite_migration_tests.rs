use shirley_show::infra::sqlite::{init_pool, migrate_pool};
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
