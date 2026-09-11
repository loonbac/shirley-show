use std::sync::Arc;
use tempfile::tempdir;

use shirley_show::domain::ticketing::Money;
use shirley_show::domain::ticketing::interest::{Interest, InterestStatus};
use shirley_show::domain::ticketing::{Event, TicketType};
use shirley_show::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::ticketing::EventRepository;
use shirley_show::use_cases::ticketing::interests::{
    CreateInterest, CreateInterestInput, DeleteInterest, GetInterest, ListInterests,
    UpdateInterest, UpdateInterestInput,
};
use shirley_show::use_cases::ticketing::summary::GetEventSummary;

fn setup_test_db() -> (tempfile::TempDir, Arc<SqliteTicketingRepository>) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();
    let repo = Arc::new(SqliteTicketingRepository::new(pool));
    (dir, repo)
}

fn create_sample_event(repo: &SqliteTicketingRepository) -> Event {
    let event = Event::new(
        "evt-test-1".to_string(),
        "Noche de Boleros".to_string(),
        "2026-11-20T20:00:00Z".to_string(),
        "Barranco Cultural".to_string(),
        vec![
            TicketType::new("General", Money::from_cents(3000), 100).unwrap(),
            TicketType::new("VIP", Money::from_cents(5000), 20).unwrap(),
        ],
        false,
        None,
        "2026-09-01T12:00:00Z".to_string(),
        "2026-09-01T12:00:00Z".to_string(),
    )
    .unwrap();
    repo.create_event(&event).unwrap();
    event
}

#[test]
fn test_interest_domain_validations() {
    // Valid creation
    let interest = Interest::new(
        "int-1".to_string(),
        "evt-test-1".to_string(),
        "  Ana Torres  ".to_string(),
        " +51987654321 ".to_string(),
        " WhatsApp ".to_string(),
        " Shirley ".to_string(),
        4,
        InterestStatus::Pendiente,
        None,
        "2026-09-01T12:00:00Z".to_string(),
        "2026-09-01T12:00:00Z".to_string(),
    )
    .unwrap();

    assert_eq!(interest.nombre, "Ana Torres");
    assert_eq!(interest.contacto, "+51987654321");
    assert_eq!(interest.canal, "WhatsApp");
    assert_eq!(interest.promotora, "Shirley");
    assert_eq!(interest.personas, 4);
    assert_eq!(interest.estado, InterestStatus::Pendiente);
    assert!(interest.convertido_at.is_none());

    // Invalid party size (personas < 1)
    let err = Interest::new(
        "int-2".to_string(),
        "evt-test-1".to_string(),
        "Carlos".to_string(),
        "999111222".to_string(),
        "Instagram".to_string(),
        "Shirley".to_string(),
        0,
        InterestStatus::Pendiente,
        None,
        "2026-09-01T12:00:00Z".to_string(),
        "2026-09-01T12:00:00Z".to_string(),
    );
    assert!(err.is_err());

    // Empty name
    let err = Interest::new(
        "int-3".to_string(),
        "evt-test-1".to_string(),
        "   ".to_string(),
        "999111222".to_string(),
        "Instagram".to_string(),
        "Shirley".to_string(),
        2,
        InterestStatus::Pendiente,
        None,
        "2026-09-01T12:00:00Z".to_string(),
        "2026-09-01T12:00:00Z".to_string(),
    );
    assert!(err.is_err());

    // Converted without Confirmado status must fail invariant
    let err = Interest::new(
        "int-4".to_string(),
        "evt-test-1".to_string(),
        "Lucia".to_string(),
        "999111222".to_string(),
        "Presencial".to_string(),
        "Shirley".to_string(),
        2,
        InterestStatus::Pendiente,
        Some("2026-09-02T12:00:00Z".to_string()),
        "2026-09-01T12:00:00Z".to_string(),
        "2026-09-01T12:00:00Z".to_string(),
    );
    assert!(err.is_err());
}

#[test]
fn test_interest_crud_and_projected_capacity() {
    let (_dir, repo) = setup_test_db();
    let event = create_sample_event(&repo);

    let create_uc = CreateInterest::new(repo.clone());
    let list_uc = ListInterests::new(repo.clone());
    let get_uc = GetInterest::new(repo.clone());
    let update_uc = UpdateInterest::new(repo.clone());
    let delete_uc = DeleteInterest::new(repo.clone());
    let summary_uc = GetEventSummary::new(repo.clone());

    // Initial summary: 0 tickets, 0 leads -> projected occupancy = 0
    let summary = summary_uc.execute(&event.id).unwrap();
    assert_eq!(summary.entradas_emitidas, 0);
    assert_eq!(summary.aforo_proyectado, 0);

    // 1. Create lead
    let lead1 = create_uc
        .execute(CreateInterestInput {
            evento_id: event.id.clone(),
            nombre: "Valeria Perez".to_string(),
            contacto: "+51912345678".to_string(),
            canal: "WhatsApp".to_string(),
            promotora: "Shirley".to_string(),
            personas: 5,
        })
        .unwrap();

    assert_eq!(lead1.estado, InterestStatus::Pendiente);
    assert_eq!(lead1.personas, 5);

    // Lead is Pendiente, so projected occupancy is still 0
    let summary = summary_uc.execute(&event.id).unwrap();
    assert_eq!(summary.aforo_proyectado, 0);

    // 2. List leads
    let leads = list_uc.execute(&event.id, None).unwrap();
    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].id, lead1.id);

    // 3. Get lead
    let fetched = get_uc.execute(&lead1.id).unwrap();
    assert_eq!(fetched.nombre, "Valeria Perez");

    // 4. Update lead to Confirmado -> projected occupancy increases to 5!
    let updated = update_uc
        .execute(UpdateInterestInput {
            id: lead1.id.clone(),
            nombre: "Valeria Perez Gomez".to_string(),
            contacto: "+51912345678".to_string(),
            canal: "WhatsApp".to_string(),
            promotora: "Shirley".to_string(),
            personas: 6,
            estado: InterestStatus::Confirmado,
        })
        .unwrap();

    assert_eq!(updated.estado, InterestStatus::Confirmado);
    assert_eq!(updated.personas, 6);

    let summary = summary_uc.execute(&event.id).unwrap();
    assert_eq!(summary.aforo_proyectado, 6);

    // 5. Add second lead as Confirmado
    let lead2 = create_uc
        .execute(CreateInterestInput {
            evento_id: event.id.clone(),
            nombre: "Jorge Ramos".to_string(),
            contacto: "+51988776655".to_string(),
            canal: "Instagram".to_string(),
            promotora: "Shirley".to_string(),
            personas: 3,
        })
        .unwrap();

    let _ = update_uc
        .execute(UpdateInterestInput {
            id: lead2.id.clone(),
            nombre: lead2.nombre.clone(),
            contacto: lead2.contacto.clone(),
            canal: lead2.canal.clone(),
            promotora: lead2.promotora.clone(),
            personas: lead2.personas,
            estado: InterestStatus::Confirmado,
        })
        .unwrap();

    let summary = summary_uc.execute(&event.id).unwrap();
    assert_eq!(summary.aforo_proyectado, 9); // 6 + 3

    // 6. Transition lead2 to Cancelado -> projected occupancy drops to 6
    let _ = update_uc
        .execute(UpdateInterestInput {
            id: lead2.id.clone(),
            nombre: lead2.nombre.clone(),
            contacto: lead2.contacto.clone(),
            canal: lead2.canal.clone(),
            promotora: lead2.promotora.clone(),
            personas: lead2.personas,
            estado: InterestStatus::Cancelado,
        })
        .unwrap();

    let summary = summary_uc.execute(&event.id).unwrap();
    assert_eq!(summary.aforo_proyectado, 6);

    // Filter by status
    let confirmed_leads = list_uc.execute(&event.id, Some("Confirmado")).unwrap();
    assert_eq!(confirmed_leads.len(), 1);
    assert_eq!(confirmed_leads[0].id, lead1.id);

    let cancelled_leads = list_uc.execute(&event.id, Some("Cancelado")).unwrap();
    assert_eq!(cancelled_leads.len(), 1);
    assert_eq!(cancelled_leads[0].id, lead2.id);

    // 7. Delete lead2
    let deleted = delete_uc.execute(&lead2.id).unwrap();
    assert!(deleted);
    let leads_after = list_uc.execute(&event.id, None).unwrap();
    assert_eq!(leads_after.len(), 1);
}

use shirley_show::domain::ticketing::ticket::{PaymentMethod, PaymentStatus};
use shirley_show::error::AppError;
use shirley_show::infra::random_credentials::OsRandomCredentials;
use shirley_show::ports::ticketing::TicketRepository;
use shirley_show::use_cases::ticketing::interests::{ConvertInterestInput, ConvertInterestToSale};

#[test]
fn test_convert_interest_to_sale_transactional_lifecycle_and_reconversion_rejection() {
    let (_dir, repo) = setup_test_db();
    let event = create_sample_event(&repo);

    let create_uc = CreateInterest::new(repo.clone());
    let get_uc = GetInterest::new(repo.clone());
    let creds_gen = Arc::new(OsRandomCredentials::new());
    let convert_uc = ConvertInterestToSale::new(repo.clone(), repo.clone(), creds_gen);

    // Create lead with 3 persons
    let lead = create_uc
        .execute(CreateInterestInput {
            evento_id: event.id.clone(),
            nombre: "Beatriz Mendoza".to_string(),
            contacto: "+51944556677".to_string(),
            canal: "WhatsApp".to_string(),
            promotora: "Shirley".to_string(),
            personas: 3,
        })
        .unwrap();

    assert_eq!(lead.estado, InterestStatus::Pendiente);
    assert!(lead.convertido_at.is_none());

    // 1. Attempt conversion with quantity exceeding capacity (VIP only has 20 seats)
    let err = convert_uc.execute(ConvertInterestInput {
        interesado_id: lead.id.clone(),
        tipo_entrada: "VIP".to_string(),
        cantidad: 25,
        medio_pago: PaymentMethod::Yape,
        estado_pago: PaymentStatus::Pagado,
    });
    assert!(err.is_err());
    let err_val = err.unwrap_err();
    assert!(matches!(
        err_val,
        AppError::Conflict {
            code: "capacity_exceeded",
            ..
        }
    ));

    // Verify lead was unchanged after failure
    let lead_after_fail = get_uc.execute(&lead.id).unwrap();
    assert_eq!(lead_after_fail.estado, InterestStatus::Pendiente);
    assert!(lead_after_fail.convertido_at.is_none());

    // Verify 0 tickets were emitted
    let tickets = repo.list_tickets(&event.id, None, None, None).unwrap();
    assert_eq!(tickets.len(), 0);

    // 2. Successful conversion to 3 General tickets
    let emitted_tickets = convert_uc
        .execute(ConvertInterestInput {
            interesado_id: lead.id.clone(),
            tipo_entrada: "General".to_string(),
            cantidad: 3,
            medio_pago: PaymentMethod::Yape,
            estado_pago: PaymentStatus::Pagado,
        })
        .unwrap();

    assert_eq!(emitted_tickets.len(), 3);
    for t in &emitted_tickets {
        assert_eq!(t.evento_id, event.id);
        assert_eq!(t.interesado_id, Some(lead.id.clone()));
        assert_eq!(t.nombre_comprador, "Beatriz Mendoza");
        assert_eq!(t.contacto, "+51944556677");
        assert_eq!(t.tipo_entrada, "General");
        assert_eq!(t.medio_pago, PaymentMethod::Yape);
        assert_eq!(t.estado_pago, PaymentStatus::Pagado);
        assert_eq!(t.precio, Money::from_cents(3000));
    }

    // Lead is now Confirmado and has convertido_at set
    let lead_converted = get_uc.execute(&lead.id).unwrap();
    assert_eq!(lead_converted.estado, InterestStatus::Confirmado);
    assert!(lead_converted.convertido_at.is_some());

    // 3. Re-conversion attempt MUST fail with 409 Conflict (interest_already_converted)
    let re_err = convert_uc.execute(ConvertInterestInput {
        interesado_id: lead.id.clone(),
        tipo_entrada: "General".to_string(),
        cantidad: 3,
        medio_pago: PaymentMethod::Yape,
        estado_pago: PaymentStatus::Pagado,
    });
    assert!(re_err.is_err());
    let re_err_val = re_err.unwrap_err();
    assert!(matches!(
        re_err_val,
        AppError::Conflict {
            code: "interest_already_converted",
            ..
        }
    ));

    // Confirm no additional tickets were emitted
    let total_tickets = repo.list_tickets(&event.id, None, None, None).unwrap();
    assert_eq!(total_tickets.len(), 3);
}
