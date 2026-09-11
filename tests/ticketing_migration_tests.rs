use tempfile::NamedTempFile;

use shirley_show::infra::sqlite::{init_pool, migrate_pool};

#[test]
fn test_migration_creates_ticketing_tables_and_indices() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let conn = pool.get().expect("get conn from pool");

    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;")
            .expect("prepare select tables");
        stmt.query_map([], |row| row.get(0))
            .expect("query_map")
            .map(|r| r.expect("row"))
            .collect()
    };

    assert!(tables.contains(&"eventos".to_string()), "eventos missing");
    assert!(
        tables.contains(&"interesados".to_string()),
        "interesados missing"
    );
    assert!(tables.contains(&"entradas".to_string()), "entradas missing");
    assert!(tables.contains(&"gastos".to_string()), "gastos missing");

    let indices: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' ORDER BY name;")
            .expect("prepare select indices");
        stmt.query_map([], |row| row.get(0))
            .expect("query_map")
            .map(|r| r.expect("row"))
            .collect()
    };

    assert!(indices.contains(&"idx_eventos_estado_fecha".to_string()));
    assert!(indices.contains(&"idx_eventos_fecha".to_string()));
    assert!(indices.contains(&"idx_interesados_evento_estado".to_string()));
    assert!(indices.contains(&"idx_interesados_evento_creacion".to_string()));
    assert!(indices.contains(&"ux_entradas_codigo_qr".to_string()));
    assert!(indices.contains(&"ux_entradas_scan_token".to_string()));
    assert!(indices.contains(&"idx_entradas_evento_creacion".to_string()));
    assert!(indices.contains(&"idx_entradas_evento_pago".to_string()));
    assert!(indices.contains(&"idx_entradas_evento_tipo".to_string()));
    assert!(indices.contains(&"idx_gastos_evento_fecha".to_string()));
    assert!(indices.contains(&"idx_gastos_evento_categoria".to_string()));
}

#[test]
fn test_migration_idempotency_and_preservation_of_v1() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("first migrate");
    migrate_pool(&pool).expect("second migrate idempotency");

    let conn = pool.get().expect("conn");

    let v1_count: i32 = conn
        .query_row(
            "SELECT count(*) FROM schema_migrations WHERE version = 1;",
            [],
            |row| row.get(0),
        )
        .expect("v1 count");
    let v2_count: i32 = conn
        .query_row(
            "SELECT count(*) FROM schema_migrations WHERE version = 2;",
            [],
            |row| row.get(0),
        )
        .expect("v2 count");

    assert_eq!(v1_count, 1);
    assert_eq!(v2_count, 1);

    // v1 tables work normally
    conn.execute(
        "INSERT INTO presentations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4);",
        rusqlite::params!["p1", "Show Titulo", "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"],
    )
    .expect("insert presentation");
}

#[test]
fn test_foreign_keys_and_cascading_deletes() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let conn = pool.get().expect("conn");

    // Inserting child without event fails
    let bad_expense = conn.execute(
        "INSERT INTO gastos (id, evento_id, categoria, monto_centimos, descripcion, fecha, fecha_creacion, fecha_actualizacion)
         VALUES ('g1', 'nonexistent', 'Transporte', 500, 'Taxi', '2026-10-10', '2026-10-10T00:00:00Z', '2026-10-10T00:00:00Z');",
        [],
    );
    assert!(bad_expense.is_err(), "FK constraint must reject invalid evento_id");

    // Insert valid event
    conn.execute(
        "INSERT INTO eventos (id, nombre, fecha_hora, lugar, tipos_de_entrada, finalizado, imagen, fecha_creacion, fecha_actualizacion)
         VALUES ('e1', 'Concierto', '2026-10-15T20:00:00Z', 'Teatro', '[]', 0, NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).expect("insert event");

    // Insert valid children
    conn.execute(
        "INSERT INTO interesados (id, evento_id, nombre, contacto, canal, promotora, personas, estado, convertido_at, fecha_creacion, fecha_actualizacion)
         VALUES ('i1', 'e1', 'Juan Perez', '999111222', 'WhatsApp', 'Promotora A', 2, 'Pendiente', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).expect("insert lead");

    conn.execute(
        "INSERT INTO entradas (id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion)
         VALUES ('t1', 'e1', 'i1', 'Juan Perez', '999111222', 'General', 2500, 'Yape', 'Pagado', 'qr123', 'st_abc123', 'No ingresó', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).expect("insert ticket");

    conn.execute(
        "INSERT INTO gastos (id, evento_id, categoria, monto_centimos, descripcion, fecha, fecha_creacion, fecha_actualizacion)
         VALUES ('g1', 'e1', 'Transporte', 500, 'Taxi', '2026-10-10', '2026-10-10T00:00:00Z', '2026-10-10T00:00:00Z');",
        [],
    ).expect("insert expense");

    // Deleting event cascades to all children
    conn.execute("DELETE FROM eventos WHERE id = 'e1';", []).expect("delete event");

    let leads_count: i32 = conn.query_row("SELECT count(*) FROM interesados WHERE evento_id = 'e1';", [], |r| r.get(0)).unwrap();
    let tickets_count: i32 = conn.query_row("SELECT count(*) FROM entradas WHERE evento_id = 'e1';", [], |r| r.get(0)).unwrap();
    let expenses_count: i32 = conn.query_row("SELECT count(*) FROM gastos WHERE evento_id = 'e1';", [], |r| r.get(0)).unwrap();

    assert_eq!(leads_count, 0, "cascaded delete for interesados");
    assert_eq!(tickets_count, 0, "cascaded delete for entradas");
    assert_eq!(expenses_count, 0, "cascaded delete for gastos");
}

#[test]
fn test_sqlite_checks_enforce_domain_invariants() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let db_path = temp_file.path().to_str().expect("valid path");

    let pool = init_pool(db_path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let conn = pool.get().expect("conn");

    // Invalid JSON in tipos_de_entrada must fail
    let bad_event = conn.execute(
        "INSERT INTO eventos (id, nombre, fecha_hora, lugar, tipos_de_entrada, finalizado, imagen, fecha_creacion, fecha_actualizacion)
         VALUES ('e_bad', 'Concierto', '2026-10-15T20:00:00Z', 'Teatro', 'invalid_json', 0, NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    );
    assert!(bad_event.is_err(), "json_valid check must reject invalid JSON");

    // Valid event
    conn.execute(
        "INSERT INTO eventos (id, nombre, fecha_hora, lugar, tipos_de_entrada, finalizado, imagen, fecha_creacion, fecha_actualizacion)
         VALUES ('e_chk', 'Concierto', '2026-10-15T20:00:00Z', 'Teatro', '[]', 0, NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).expect("insert event");

    // Cortesía with non-zero price must fail
    let bad_cortesia = conn.execute(
        "INSERT INTO entradas (id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion)
         VALUES ('t_bad', 'e_chk', NULL, 'Amigo', '999', 'VIP', 100, 'Cortesía', 'Pagado', 'qr_bad', 'st_bad', 'No ingresó', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    );
    assert!(bad_cortesia.is_err(), "Cortesia must have 0 price");

    // Gastos invalid category must fail
    let bad_category = conn.execute(
        "INSERT INTO gastos (id, evento_id, categoria, monto_centimos, descripcion, fecha, fecha_creacion, fecha_actualizacion)
         VALUES ('g_bad', 'e_chk', 'Inventada', 500, 'Desc', '2026-10-10', '2026-10-10T00:00:00Z', '2026-10-10T00:00:00Z');",
        [],
    );
    assert!(bad_category.is_err(), "Expense category must be in closed set");
}
