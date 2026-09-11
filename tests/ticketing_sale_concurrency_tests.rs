use std::sync::Arc;
use std::thread;

use shirley_show::domain::ticketing::ticket::{CheckinStatus, PaymentMethod, PaymentStatus};
use shirley_show::domain::ticketing::{Event, Money, TicketType};
use shirley_show::infra::random_credentials::OsRandomCredentials;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use shirley_show::ports::ticketing::{EventRepository, SellTicketsInput, TicketRepository};
use shirley_show::use_cases::ticketing::tickets::{
    ConfirmTicketPayment, GetPublicTicket, SellTickets,
};
use tempfile::NamedTempFile;

fn setup_test_db() -> (Arc<SqliteTicketingRepository>, String, NamedTempFile) {
    let temp_file = NamedTempFile::new().unwrap();
    let pool = init_pool(temp_file.path().to_str().unwrap()).expect("pool creation failed");
    migrate_pool(&pool).expect("migrations failed");
    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let event_id = "evt-sale-test".to_string();
    let event = Event::new(
        event_id.clone(),
        "Concierto Acústico".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Barranco".to_string(),
        vec![
            TicketType::new("General".to_string(), Money::from_soles(30.0).unwrap(), 10).unwrap(),
            TicketType::new("VIP".to_string(), Money::from_soles(60.0).unwrap(), 5).unwrap(),
        ],
        false,
        None,
        "2026-10-01T10:00:00Z".to_string(),
        "2026-10-01T10:00:00Z".to_string(),
    )
    .unwrap();

    repo.create_event(&event).expect("create_event failed");
    (repo, event_id, temp_file)
}

#[test]
fn test_multi_row_transactional_ticket_issuance_and_rollback() {
    let (repo, event_id, _temp) = setup_test_db();
    let credentials_gen = Arc::new(OsRandomCredentials::new());

    let sell_tickets = SellTickets::new(
        repo.clone(),
        repo.clone(),
        credentials_gen.clone(),
    );

    // 1. Successful sale of 3 General tickets
    let sale_input = SellTicketsInput {
        evento_id: event_id.clone(),
        interesado_id: None,
        nombre_comprador: "Carlos Mendoza".to_string(),
        contacto: "987654321".to_string(),
        tipo_entrada: "General".to_string(),
        cantidad: 3,
        medio_pago: PaymentMethod::Yape,
        estado_pago: PaymentStatus::Pagado,
    };

    let issued = sell_tickets.execute(sale_input).expect("sale of 3 tickets should succeed");
    assert_eq!(issued.len(), 3);

    for ticket in &issued {
        assert_eq!(ticket.evento_id, event_id);
        assert_eq!(ticket.nombre_comprador, "Carlos Mendoza");
        assert_eq!(ticket.contacto, "987654321");
        assert_eq!(ticket.tipo_entrada, "General");
        assert_eq!(ticket.precio, Money::from_soles(30.0).unwrap());
        assert_eq!(ticket.medio_pago, PaymentMethod::Yape);
        assert_eq!(ticket.estado_pago, PaymentStatus::Pagado);
        assert_eq!(ticket.estado_checkin, CheckinStatus::NoIngreso);
        assert!(ticket.hora_ingreso.is_none());
        assert!(ticket.scan_token.as_str().starts_with("st_"));
        assert_eq!(ticket.codigo_qr.as_str().len(), 32);
    }

    // Verify all 3 IDs, scan_tokens, and qr_secrets are distinct
    let ids: std::collections::HashSet<_> = issued.iter().map(|t| &t.id).collect();
    assert_eq!(ids.len(), 3);
    let tokens: std::collections::HashSet<_> = issued.iter().map(|t| t.scan_token.as_str()).collect();
    assert_eq!(tokens.len(), 3);
    let qrs: std::collections::HashSet<_> = issued.iter().map(|t| t.codigo_qr.as_str()).collect();
    assert_eq!(qrs.len(), 3);

    // Check in database: exactly 3 rows in entradas
    let all_tickets = repo.list_tickets(&event_id, None, None, None).unwrap();
    assert_eq!(all_tickets.len(), 3);

    // 2. Capacity is 10. 3 emitted. Available = 7.
    // Attempting to sell 8 tickets MUST fail with capacity error and rollback completely.
    let oversized_sale = SellTicketsInput {
        evento_id: event_id.clone(),
        interesado_id: None,
        nombre_comprador: "Grupo Grande".to_string(),
        contacto: "911222333".to_string(),
        tipo_entrada: "General".to_string(),
        cantidad: 8,
        medio_pago: PaymentMethod::Efectivo,
        estado_pago: PaymentStatus::Pendiente,
    };

    let err = sell_tickets.execute(oversized_sale);
    assert!(err.is_err(), "oversized sale must be rejected");

    // CRITICAL: verify transactional rollback - exactly 3 rows remain, ZERO new rows inserted
    let tickets_after_fail = repo.list_tickets(&event_id, None, None, None).unwrap();
    assert_eq!(tickets_after_fail.len(), 3, "rollback must leave database completely unaffected");

    // 3. Reject non-positive quantity
    let zero_sale = SellTicketsInput {
        evento_id: event_id.clone(),
        interesado_id: None,
        nombre_comprador: "Test".to_string(),
        contacto: "123".to_string(),
        tipo_entrada: "General".to_string(),
        cantidad: 0,
        medio_pago: PaymentMethod::Yape,
        estado_pago: PaymentStatus::Pagado,
    };
    assert!(sell_tickets.execute(zero_sale).is_err());
}

#[test]
fn test_concurrent_ticket_sales_respect_capacity_ceiling() {
    let (repo, event_id, _temp) = setup_test_db();
    let credentials_gen = Arc::new(OsRandomCredentials::new());

    // VIP capacity is 5.
    // Spawn 5 concurrent threads, each trying to buy 2 VIP tickets (total requested = 10).
    // Exactly 2 threads must succeed (2 * 2 = 4 tickets) or thread combinations that don't exceed 5.
    let mut handles = Vec::new();

    for i in 0..5 {
        let repo_clone = repo.clone();
        let creds_clone = credentials_gen.clone();
        let evt = event_id.clone();

        handles.push(thread::spawn(move || {
            let sell_use_case = SellTickets::new(repo_clone.clone(), repo_clone, creds_clone);
            sell_use_case.execute(SellTicketsInput {
                evento_id: evt,
                interesado_id: None,
                nombre_comprador: format!("Comprador Concurrent {}", i),
                contacto: format!("99900000{}", i),
                tipo_entrada: "VIP".to_string(),
                cantidad: 2,
                medio_pago: PaymentMethod::Yape,
                estado_pago: PaymentStatus::Pagado,
            })
        }));
    }

    let mut success_count = 0;
    let mut fail_count = 0;
    for handle in handles {
        match handle.join().unwrap() {
            Ok(tickets) => {
                assert_eq!(tickets.len(), 2);
                success_count += 1;
            }
            Err(_) => {
                fail_count += 1;
            }
        }
    }

    // Exactly 2 transactions can succeed (2 * 2 = 4 <= 5). The 3rd would be 6 > 5 so must fail.
    assert_eq!(success_count, 2, "exactly two sales of 2 tickets can fit into capacity 5");
    assert_eq!(fail_count, 3, "remaining 3 threads must be rejected");

    let vip_tickets = repo.list_tickets(&event_id, None, None, Some("VIP")).unwrap();
    assert_eq!(vip_tickets.len(), 4, "database must have exactly 4 tickets persisted");
}

#[test]
fn test_triangulate_courtesy_invariants_and_public_ticket_concealment() {
    let (repo, event_id, _temp) = setup_test_db();
    let credentials_gen = Arc::new(OsRandomCredentials::new());

    let sell_tickets = SellTickets::new(
        repo.clone(),
        repo.clone(),
        credentials_gen,
    );

    // 1. Courtesy forces price 0.0 and Pagado
    let cortesia_sale = SellTicketsInput {
        evento_id: event_id.clone(),
        interesado_id: None,
        nombre_comprador: "Artista Invitado".to_string(),
        contacto: "998877665".to_string(),
        tipo_entrada: "General".to_string(),
        cantidad: 2,
        medio_pago: PaymentMethod::Cortesia,
        estado_pago: PaymentStatus::Pendiente, // Client passed Pendiente, must be forced to Pagado
    };

    let courtesy_tickets = sell_tickets.execute(cortesia_sale).expect("courtesy sale should succeed");
    assert_eq!(courtesy_tickets.len(), 2);
    for t in &courtesy_tickets {
        assert_eq!(t.precio, Money::zero(), "courtesy must force price 0.0");
        assert_eq!(t.estado_pago, PaymentStatus::Pagado, "courtesy must force Pagado");
        assert_eq!(t.medio_pago, PaymentMethod::Cortesia);
    }

    // 2. Issue a Pending ticket and test ConfirmTicketPayment & GetPublicTicket
    let pending_sale = SellTicketsInput {
        evento_id: event_id.clone(),
        interesado_id: None,
        nombre_comprador: "Marcos Diaz".to_string(),
        contacto: "987111222".to_string(),
        tipo_entrada: "General".to_string(),
        cantidad: 1,
        medio_pago: PaymentMethod::Efectivo,
        estado_pago: PaymentStatus::Pendiente,
    };
    let pending_tickets = sell_tickets.execute(pending_sale).unwrap();
    let pending_ticket = &pending_tickets[0];

    let get_public = GetPublicTicket::new(repo.clone(), repo.clone());

    // Public view for PENDING ticket
    let public_pending = get_public
        .execute(pending_ticket.scan_token.as_str())
        .expect("get public ticket pending");

    assert_eq!(public_pending.nombre_evento, "Concierto Acústico");
    assert_eq!(public_pending.nombre_comprador, "Marcos Diaz");
    assert_eq!(public_pending.tipo_entrada, "General");
    assert_eq!(public_pending.precio, 30.0);
    assert_eq!(public_pending.estado_pago, "Pendiente");
    assert_eq!(public_pending.estado_checkin, "No ingresó");
    assert!(public_pending.qr_svg.is_none(), "pending public ticket must NEVER expose QR SVG");

    // 3. Confirm Payment
    let confirm_payment = ConfirmTicketPayment::new(repo.clone());
    let confirmed = confirm_payment.execute(&pending_ticket.id).expect("confirm payment");
    assert_eq!(confirmed.estado_pago, PaymentStatus::Pagado);
    assert_eq!(confirmed.estado_checkin, CheckinStatus::NoIngreso);

    // Public view for PAID ticket
    let public_paid = get_public
        .execute(pending_ticket.scan_token.as_str())
        .expect("get public ticket paid");

    assert_eq!(public_paid.estado_pago, "Pagado");
    assert!(public_paid.qr_svg.is_some(), "paid public ticket MUST expose QR SVG");
    let svg = public_paid.qr_svg.as_ref().unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.ends_with("</svg>"));

    // Crucial security invariant: codigo_qr secret is NOT exposed in the JSON representation
    let json_val = serde_json::to_value(&public_paid).unwrap();
    assert!(json_val.get("codigo_qr").is_none(), "codigo_qr secret must never be exposed publicly");
    assert!(json_val.get("id").is_none(), "internal id must not be exposed");
    assert!(json_val.get("evento_id").is_none(), "evento_id must not be exposed");
    assert!(json_val.get("scan_token").is_none(), "scan_token must not be exposed in body");
}
