use std::sync::Arc;
use std::thread;

use shirley_show::domain::ticketing::ticket::{CheckinStatus, PaymentMethod, PaymentStatus};
use shirley_show::domain::ticketing::{Event, Money, TicketType};
use shirley_show::infra::random_credentials::OsRandomCredentials;
use shirley_show::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::ticketing::{
    CheckInOutcome, EventRepository, SellTicketsInput, TicketRepository,
};
use shirley_show::use_cases::ticketing::checkin::CheckInTicket;
use shirley_show::use_cases::ticketing::tickets::SellTickets;
use tempfile::NamedTempFile;

fn setup_door_test_db() -> (
    Arc<SqliteTicketingRepository>,
    String,
    String,
    NamedTempFile,
) {
    let temp_file = NamedTempFile::new().unwrap();
    let pool = init_pool(temp_file.path().to_str().unwrap()).expect("pool creation failed");
    migrate_pool(&pool).expect("migrations failed");
    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let event1_id = "evt-door-miraflores".to_string();
    let event1 = Event::new(
        event1_id.clone(),
        "Show Miraflores".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Miraflores".to_string(),
        vec![TicketType::new("General".to_string(), Money::from_soles(40.0).unwrap(), 50).unwrap()],
        false,
        None,
        "2026-10-01T10:00:00Z".to_string(),
        "2026-10-01T10:00:00Z".to_string(),
    )
    .unwrap();
    repo.create_event(&event1).unwrap();

    let event2_id = "evt-door-barranco".to_string();
    let event2 = Event::new(
        event2_id.clone(),
        "Show Barranco".to_string(),
        "2026-10-16T21:00:00Z".to_string(),
        "Barranco".to_string(),
        vec![TicketType::new("General".to_string(), Money::from_soles(40.0).unwrap(), 50).unwrap()],
        false,
        None,
        "2026-10-01T10:00:00Z".to_string(),
        "2026-10-01T10:00:00Z".to_string(),
    )
    .unwrap();
    repo.create_event(&event2).unwrap();

    (repo, event1_id, event2_id, temp_file)
}

#[test]
fn test_checkin_validation_outcomes_and_security_rules() {
    let (repo, event1_id, event2_id, _temp) = setup_door_test_db();
    let creds_gen = Arc::new(OsRandomCredentials::new());
    let sell_tickets = SellTickets::new(repo.clone(), repo.clone(), creds_gen);
    let checkin = CheckInTicket::new(repo.clone());

    // Issue 1 paid ticket for event 1
    let paid_tickets = sell_tickets
        .execute(SellTicketsInput {
            evento_id: event1_id.clone(),
            interesado_id: None,
            nombre_comprador: "Lucia Alva".to_string(),
            contacto: "987654321".to_string(),
            tipo_entrada: "General".to_string(),
            cantidad: 1,
            medio_pago: PaymentMethod::Yape,
            estado_pago: PaymentStatus::Pagado,
        })
        .unwrap();
    let paid_ticket = &paid_tickets[0];

    // Issue 1 pending ticket for event 1
    let pending_tickets = sell_tickets
        .execute(SellTicketsInput {
            evento_id: event1_id.clone(),
            interesado_id: None,
            nombre_comprador: "Pedro Gomez".to_string(),
            contacto: "911222333".to_string(),
            tipo_entrada: "General".to_string(),
            cantidad: 1,
            medio_pago: PaymentMethod::Efectivo,
            estado_pago: PaymentStatus::Pendiente,
        })
        .unwrap();
    let pending_ticket = &pending_tickets[0];

    // Issue 1 paid ticket for event 2
    let event2_tickets = sell_tickets
        .execute(SellTicketsInput {
            evento_id: event2_id.clone(),
            interesado_id: None,
            nombre_comprador: "Rosa Flores".to_string(),
            contacto: "955444333".to_string(),
            tipo_entrada: "General".to_string(),
            cantidad: 1,
            medio_pago: PaymentMethod::Yape,
            estado_pago: PaymentStatus::Pagado,
        })
        .unwrap();
    let event2_ticket = &event2_tickets[0];

    // Case 1: Reject scan_token submitted as check-in credential
    let res_scan_token = checkin
        .execute(&event1_id, paid_ticket.scan_token.as_str())
        .unwrap();
    match res_scan_token {
        CheckInOutcome::Invalida { .. } => {}
        other => panic!(
            "scan_token must be rejected with Invalida, got: {:?}",
            other
        ),
    }

    // Case 2: Reject unknown or malformed code
    let res_unknown = checkin
        .execute(&event1_id, "00001111222233334444555566667777")
        .unwrap();
    match res_unknown {
        CheckInOutcome::Invalida { motivo } => {
            assert!(motivo.to_lowercase().contains("no encontrado"));
        }
        other => panic!("unknown code must be Invalida, got: {:?}", other),
    }

    // Case 3: Reject ticket from another event (cross-event)
    let res_cross = checkin
        .execute(&event1_id, event2_ticket.codigo_qr.as_str())
        .unwrap();
    match res_cross {
        CheckInOutcome::Invalida { motivo } => {
            assert!(motivo.to_lowercase().contains("otro evento"));
        }
        other => panic!("cross-event code must be Invalida, got: {:?}", other),
    }

    // Case 4: Reject pending payment ticket
    let res_pending = checkin
        .execute(&event1_id, pending_ticket.codigo_qr.as_str())
        .unwrap();
    match res_pending {
        CheckInOutcome::Invalida { motivo } => {
            assert!(motivo.to_lowercase().contains("pendiente"));
        }
        other => panic!("pending payment must be Invalida, got: {:?}", other),
    }

    // Case 5: Valid check-in for paid ticket
    let res_valid = checkin
        .execute(&event1_id, paid_ticket.codigo_qr.as_str())
        .unwrap();
    let first_entry_time = match res_valid {
        CheckInOutcome::Valida {
            nombre_comprador,
            tipo_entrada,
            hora_ingreso,
        } => {
            assert_eq!(nombre_comprador, "Lucia Alva");
            assert_eq!(tipo_entrada, "General");
            assert!(!hora_ingreso.is_empty());
            hora_ingreso
        }
        other => panic!("first checkin must be Valida, got: {:?}", other),
    };

    // Verify DB state
    let updated_ticket = repo.get_ticket(&paid_ticket.id).unwrap().unwrap();
    assert_eq!(updated_ticket.estado_checkin, CheckinStatus::Ingreso);
    assert_eq!(
        updated_ticket.hora_ingreso.as_deref(),
        Some(first_entry_time.as_str())
    );

    // Case 6: Re-scanning already used ticket returns Ya Usada
    let res_already_used = checkin
        .execute(&event1_id, paid_ticket.codigo_qr.as_str())
        .unwrap();
    match res_already_used {
        CheckInOutcome::YaUsada { hora_ingreso } => {
            assert_eq!(hora_ingreso, first_entry_time);
        }
        other => panic!("second checkin must be Ya Usada, got: {:?}", other),
    }
}

#[test]
fn test_concurrent_door_checkin_produces_exactly_one_valid_and_one_used() {
    let (repo, event_id, _, _temp) = setup_door_test_db();
    let creds_gen = Arc::new(OsRandomCredentials::new());
    let sell_tickets = SellTickets::new(repo.clone(), repo.clone(), creds_gen);

    let tickets = sell_tickets
        .execute(SellTicketsInput {
            evento_id: event_id.clone(),
            interesado_id: None,
            nombre_comprador: "Competitor".to_string(),
            contacto: "999888777".to_string(),
            tipo_entrada: "General".to_string(),
            cantidad: 1,
            medio_pago: PaymentMethod::Yape,
            estado_pago: PaymentStatus::Pagado,
        })
        .unwrap();
    let ticket = Arc::new(tickets[0].clone());

    // Spawn 2 threads competing to check in the exact same ticket simultaneously
    let t1_repo = repo.clone();
    let t1_evt = event_id.clone();
    let t1_ticket = ticket.clone();
    let h1 = thread::spawn(move || {
        let checkin = CheckInTicket::new(t1_repo);
        checkin.execute(&t1_evt, t1_ticket.codigo_qr.as_str())
    });

    let t2_repo = repo.clone();
    let t2_evt = event_id.clone();
    let t2_ticket = ticket.clone();
    let h2 = thread::spawn(move || {
        let checkin = CheckInTicket::new(t2_repo);
        checkin.execute(&t2_evt, t2_ticket.codigo_qr.as_str())
    });

    let r1 = h1.join().unwrap().unwrap();
    let r2 = h2.join().unwrap().unwrap();

    let mut valida_count = 0;
    let mut ya_usada_count = 0;

    for res in [r1, r2] {
        match res {
            CheckInOutcome::Valida { .. } => valida_count += 1,
            CheckInOutcome::YaUsada { .. } => ya_usada_count += 1,
            CheckInOutcome::Invalida { motivo } => {
                panic!("Neither concurrent thread should get Invalida: {}", motivo)
            }
        }
    }

    assert_eq!(
        valida_count, 1,
        "exactly ONE concurrent scan must be Valida"
    );
    assert_eq!(
        ya_usada_count, 1,
        "the other concurrent scan must be Ya Usada"
    );
}
