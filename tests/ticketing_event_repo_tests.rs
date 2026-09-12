use tempfile::NamedTempFile;

use shirley_show::domain::ticketing::ticket::{PaymentMethod, PaymentStatus};
use shirley_show::domain::ticketing::{
    Event, Expense, ExpenseCategory, Money, QrSecret, ScanToken, TicketType,
};
use shirley_show::infra::sqlite::ticketing_repository::{
    DEFAULT_TICKETS_LIMIT, SqliteTicketingRepository,
};
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::ticketing::{
    EventRepository, EventSummaryReader, ExpenseRepository, GeneratedCredentials, SellTicketsInput,
    TicketRepository, TicketingTransactions,
};

#[test]
fn test_event_repository_crud_and_status_toggle() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = SqliteTicketingRepository::new(pool);

    let types = vec![
        TicketType::new("General", Money::from_soles(30.0).unwrap(), 80).unwrap(),
        TicketType::new("VIP", Money::from_soles(50.0).unwrap(), 20).unwrap(),
    ];

    let event = Event::new(
        "evt-1".to_string(),
        "Concierto Acústico".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "La Noche de Barranco".to_string(),
        types,
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .unwrap();

    repo.create_event(&event).expect("create event");

    let fetched = repo
        .get_event("evt-1")
        .expect("get event")
        .expect("event found");
    assert_eq!(fetched.nombre, "Concierto Acústico");
    assert_eq!(fetched.total_capacity(), 100);
    assert!(!fetched.finalizado);

    // List events
    let list = repo.list_events().expect("list events");
    assert_eq!(list.len(), 1);

    // Update event
    let mut updated = fetched;
    updated.nombre = "Concierto Acústico 2026".to_string();
    updated.fecha_actualizacion = "2026-10-02T00:00:00Z".to_string();
    repo.update_event(&updated).expect("update event");

    let re_fetched = repo.get_event("evt-1").unwrap().unwrap();
    assert_eq!(re_fetched.nombre, "Concierto Acústico 2026");

    // Toggle finalizado
    repo.set_finalizado("evt-1", true).expect("finalize event");
    assert!(repo.get_event("evt-1").unwrap().unwrap().finalizado);

    repo.set_finalizado("evt-1", false).expect("reopen event");
    assert!(!repo.get_event("evt-1").unwrap().unwrap().finalizado);

    // Delete event
    let deleted = repo.delete_event("evt-1").expect("delete event");
    assert!(deleted);
    assert!(repo.get_event("evt-1").unwrap().is_none());
}

#[test]
fn test_expense_repository_crud() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = SqliteTicketingRepository::new(pool.clone());

    // Create parent event
    let event = Event::new(
        "evt-e1".to_string(),
        "Concierto".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        vec![TicketType::new("General", Money::from_soles(20.0).unwrap(), 50).unwrap()],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .unwrap();
    repo.create_event(&event).unwrap();

    let expense = Expense::new(
        "exp-1".to_string(),
        "evt-e1".to_string(),
        ExpenseCategory::SonidoLuces,
        Money::from_soles(450.0).unwrap(),
        "Consola y micrófonos".to_string(),
        "2026-10-14".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .unwrap();

    repo.create_expense(&expense).expect("create expense");

    let list = repo.list_expenses("evt-e1").expect("list expenses");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].monto.to_soles(), 450.0);

    let mut exp_upd = list[0].clone();
    exp_upd.monto = Money::from_soles(500.0).unwrap();
    repo.update_expense(&exp_upd).expect("update expense");

    let fetched = repo.get_expense("exp-1").unwrap().unwrap();
    assert_eq!(fetched.monto.to_soles(), 500.0);

    let deleted = repo.delete_expense("exp-1").expect("delete expense");
    assert!(deleted);
    assert!(repo.get_expense("exp-1").unwrap().is_none());
}

#[test]
fn test_event_summary_reader() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = SqliteTicketingRepository::new(pool.clone());

    let event = Event::new(
        "evt-sum".to_string(),
        "Gran Noche".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        vec![
            TicketType::new("General", Money::from_soles(30.0).unwrap(), 40).unwrap(),
            TicketType::new("VIP", Money::from_soles(50.0).unwrap(), 10).unwrap(),
        ],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .unwrap();
    repo.create_event(&event).unwrap();

    // Empty summary
    let empty_sum = repo.get_event_summary("evt-sum").expect("empty summary");
    assert_eq!(empty_sum.cobrado.to_soles(), 0.0);
    assert_eq!(empty_sum.pendiente.to_soles(), 0.0);
    assert_eq!(empty_sum.total_proyectado.to_soles(), 0.0);
    assert_eq!(empty_sum.gastos.to_soles(), 0.0);
    assert_eq!(empty_sum.ganancia_neta_soles, 0.0);
    assert_eq!(empty_sum.capacidad_total, 50);
    assert_eq!(empty_sum.entradas_emitidas, 0);
    assert_eq!(empty_sum.aforo_proyectado, 0);

    // Add expenses
    repo.create_expense(
        &Expense::new(
            "g1".to_string(),
            "evt-sum".to_string(),
            ExpenseCategory::LocalAlquiler,
            Money::from_soles(500.0).unwrap(),
            "Alquiler sala".to_string(),
            "2026-10-10".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
        )
        .unwrap(),
    )
    .unwrap();

    repo.create_expense(
        &Expense::new(
            "g2".to_string(),
            "evt-sum".to_string(),
            ExpenseCategory::SonidoLuces,
            Money::from_soles(300.0).unwrap(),
            "Luces".to_string(),
            "2026-10-11".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
        )
        .unwrap(),
    )
    .unwrap();

    // Insert entries into entradas and interesados directly via conn to simulate sales and leads
    let conn = pool.get().unwrap();
    // 2 tickets Yape Pagado @ 30.0 = 60.0
    conn.execute(
        "INSERT INTO entradas (id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion)
         VALUES ('t1', 'evt-sum', NULL, 'Comprador 1', '999', 'General', 3000, 'Yape', 'Pagado', 'qr1', 'st1', 'No ingresó', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO entradas (id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion)
         VALUES ('t2', 'evt-sum', NULL, 'Comprador 2', '999', 'General', 3000, 'Yape', 'Pagado', 'qr2', 'st2', 'No ingresó', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).unwrap();

    // 1 ticket Efectivo Pendiente @ 50.0 = 50.0
    conn.execute(
        "INSERT INTO entradas (id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion)
         VALUES ('t3', 'evt-sum', NULL, 'Comprador 3', '999', 'VIP', 5000, 'Efectivo', 'Pendiente', 'qr3', 'st3', 'No ingresó', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).unwrap();

    // 1 ticket Cortesía @ 0.0
    conn.execute(
        "INSERT INTO entradas (id, evento_id, interesado_id, nombre_comprador, contacto, tipo_entrada, precio_centimos, medio_pago, estado_pago, codigo_qr, scan_token, estado_checkin, hora_ingreso, fecha_creacion, fecha_actualizacion)
         VALUES ('t4', 'evt-sum', NULL, 'Invitado', '999', 'VIP', 0, 'Cortesía', 'Pagado', 'qr4', 'st4', 'No ingresó', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).unwrap();

    // 1 lead Confirmado with 3 persons
    conn.execute(
        "INSERT INTO interesados (id, evento_id, nombre, contacto, canal, promotora, personas, estado, convertido_at, fecha_creacion, fecha_actualizacion)
         VALUES ('i1', 'evt-sum', 'Lead Confirmado', '999', 'WhatsApp', 'P1', 3, 'Confirmado', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).unwrap();

    // 1 lead Pendiente with 2 persons (should not add to aforo proyectado)
    conn.execute(
        "INSERT INTO interesados (id, evento_id, nombre, contacto, canal, promotora, personas, estado, convertido_at, fecha_creacion, fecha_actualizacion)
         VALUES ('i2', 'evt-sum', 'Lead Pendiente', '999', 'WhatsApp', 'P1', 2, 'Pendiente', NULL, '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z');",
        [],
    ).unwrap();

    let sum = repo
        .get_event_summary("evt-sum")
        .expect("populated summary");
    assert_eq!(sum.cobrado.to_soles(), 60.0);
    assert_eq!(sum.pendiente.to_soles(), 50.0);
    assert_eq!(sum.total_proyectado.to_soles(), 110.0);
    assert_eq!(sum.gastos.to_soles(), 800.0);
    assert_eq!(sum.ganancia_neta_soles, -740.0); // 60.0 - 800.0
    assert_eq!(sum.capacidad_total, 50);
    assert_eq!(sum.entradas_emitidas, 4); // 2 paid + 1 pending + 1 cortesia
    assert_eq!(sum.aforo_proyectado, 7); // 4 entradas + 3 personas de lead confirmado

    // Breakdowns
    assert_eq!(sum.metodos_pago.yape_cobrado.to_soles(), 60.0);
    assert_eq!(sum.metodos_pago.yape_entradas, 2);
    assert_eq!(sum.metodos_pago.efectivo_pendiente.to_soles(), 50.0);
    assert_eq!(sum.metodos_pago.efectivo_entradas, 1);
    assert_eq!(sum.metodos_pago.cortesia_entradas, 1);

    assert_eq!(sum.categorias_gasto.local_alquiler.to_soles(), 500.0);
    assert_eq!(sum.categorias_gasto.sonido_luces.to_soles(), 300.0);
    assert_eq!(sum.categorias_gasto.transporte.to_soles(), 0.0);
}

#[test]
fn test_list_tickets_is_bounded_by_default_limit() {
    assert_eq!(DEFAULT_TICKETS_LIMIT, 500);

    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = SqliteTicketingRepository::new(pool);
    let list = repo
        .list_tickets("evt-non-existent", None, None, None)
        .unwrap();
    assert!(list.is_empty());
}

#[test]
fn test_update_event_preserves_sold_types_and_capacity() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = SqliteTicketingRepository::new(pool.clone());

    // 1. Create event with General (cap 50) and VIP (cap 20)
    let event = Event::new(
        "evt-c4".to_string(),
        "Concierto Invariante".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        vec![
            TicketType::new("General", Money::from_soles(30.0).unwrap(), 50).unwrap(),
            TicketType::new("VIP", Money::from_soles(60.0).unwrap(), 20).unwrap(),
        ],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .unwrap();
    repo.create_event(&event).unwrap();

    // 2. Issue 5 tickets of type "General"
    let creds: Vec<_> = (0..5)
        .map(|i| GeneratedCredentials {
            id: format!("tix-c4-{i}"),
            scan_token: ScanToken::new(format!("st_{:064x}", i + 1)).unwrap(),
            qr_secret: QrSecret::new(format!("{:032x}", i + 1)).unwrap(),
        })
        .collect();

    repo.issue_sale(
        &SellTicketsInput {
            evento_id: "evt-c4".to_string(),
            interesado_id: None,
            nombre_comprador: "Comprador C4".to_string(),
            contacto: "999888777".to_string(),
            tipo_entrada: "General".to_string(),
            cantidad: 5,
            medio_pago: PaymentMethod::Yape,
            estado_pago: PaymentStatus::Pagado,
        },
        creds,
    )
    .expect("issue sale");

    // 3. Attempt to update event removing "General" (only keeping VIP) -> MUST FAIL
    let bad_update_remove = Event::new(
        "evt-c4".to_string(),
        "Concierto Invariante".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        vec![TicketType::new("VIP", Money::from_soles(60.0).unwrap(), 20).unwrap()],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-02T00:00:00Z".to_string(),
    )
    .unwrap();
    let err_remove = repo.update_event(&bad_update_remove);
    assert!(err_remove.is_err());
    let err_val = err_remove.unwrap_err();
    assert_eq!(err_val.code(), "cannot_remove_sold_ticket_type");

    // 4. Attempt to update event reducing General capacity below emitted (cap 3 < 5) -> MUST FAIL
    let bad_update_cap = Event::new(
        "evt-c4".to_string(),
        "Concierto Invariante".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        vec![
            TicketType::new("General", Money::from_soles(30.0).unwrap(), 3).unwrap(),
            TicketType::new("VIP", Money::from_soles(60.0).unwrap(), 20).unwrap(),
        ],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-02T00:00:00Z".to_string(),
    )
    .unwrap();
    let err_cap = repo.update_event(&bad_update_cap);
    assert!(err_cap.is_err());
    let err_val2 = err_cap.unwrap_err();
    assert_eq!(err_val2.code(), "capacity_below_emitted");

    // 5. Valid update preserving General with capacity >= 5 -> MUST SUCCEED
    let good_update = Event::new(
        "evt-c4".to_string(),
        "Concierto Invariante Renombrado".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        vec![
            TicketType::new("General", Money::from_soles(35.0).unwrap(), 55).unwrap(),
            TicketType::new("VIP", Money::from_soles(70.0).unwrap(), 25).unwrap(),
        ],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-02T00:00:00Z".to_string(),
    )
    .unwrap();
    assert!(repo.update_event(&good_update).is_ok());
}
