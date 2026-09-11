use std::sync::Arc;
use tempfile::NamedTempFile;

use shirley_show::domain::ticketing::{ExpenseCategory, Money, TicketType};
use shirley_show::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::use_cases::ticketing::events::{
    CreateEvent, CreateEventInput, DeleteEvent, FinalizeEvent, GetEvent, ListEvents, ReopenEvent,
    UpdateEvent, UpdateEventInput,
};
use shirley_show::use_cases::ticketing::expenses::{
    CreateExpense, CreateExpenseInput, DeleteExpense, ListExpenses, UpdateExpense,
    UpdateExpenseInput,
};
use shirley_show::use_cases::ticketing::summary::GetEventSummary;

#[test]
fn test_event_use_cases_full_lifecycle() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let create_uc = CreateEvent::new(repo.clone());
    let list_uc = ListEvents::new(repo.clone());
    let get_uc = GetEvent::new(repo.clone());
    let update_uc = UpdateEvent::new(repo.clone());
    let finalize_uc = FinalizeEvent::new(repo.clone());
    let reopen_uc = ReopenEvent::new(repo.clone());
    let delete_uc = DeleteEvent::new(repo.clone());

    // 1. Create
    let input = CreateEventInput {
        nombre: "Festival de Primavera".to_string(),
        fecha_hora: "2026-10-20T20:00:00Z".to_string(),
        lugar: "Auditorio Municipal".to_string(),
        tipos_de_entrada: vec![
            TicketType::new("General", Money::from_soles(25.0).unwrap(), 150).unwrap(),
            TicketType::new("VIP", Money::from_soles(60.0).unwrap(), 30).unwrap(),
        ],
    };
    let event = create_uc.execute(input).expect("create event");
    let event_id = event.id.clone();
    assert_eq!(event.nombre, "Festival de Primavera");
    assert!(!event.finalizado);

    // 2. Get
    let fetched = get_uc.execute(&event_id).expect("get event");
    assert_eq!(fetched.id, event_id);

    // 3. List
    let list = list_uc.execute().expect("list events");
    assert_eq!(list.len(), 1);

    // 4. Update
    let update_input = UpdateEventInput {
        id: event_id.clone(),
        nombre: "Festival de Primavera - Edición Especial".to_string(),
        fecha_hora: "2026-10-21T20:00:00Z".to_string(),
        lugar: "Auditorio Central".to_string(),
        tipos_de_entrada: vec![TicketType::new("General", Money::from_soles(30.0).unwrap(), 180).unwrap()],
    };
    let updated = update_uc.execute(update_input).expect("update event");
    assert_eq!(updated.nombre, "Festival de Primavera - Edición Especial");
    assert_eq!(updated.total_capacity(), 180);

    // 5. Finalize
    finalize_uc.execute(&event_id).expect("finalize");
    assert!(get_uc.execute(&event_id).unwrap().finalizado);

    // 6. Reopen
    reopen_uc.execute(&event_id).expect("reopen");
    assert!(!get_uc.execute(&event_id).unwrap().finalizado);

    // 7. Delete
    delete_uc.execute(&event_id).expect("delete");
    assert!(get_uc.execute(&event_id).is_err());
}

#[test]
fn test_expense_and_summary_use_cases() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let pool = init_pool(temp_file.path().to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let create_event = CreateEvent::new(repo.clone());
    let event = create_event
        .execute(CreateEventInput {
            nombre: "Show Acústico".to_string(),
            fecha_hora: "2026-11-01T21:00:00Z".to_string(),
            lugar: "La Noche".to_string(),
            tipos_de_entrada: vec![TicketType::new("General", Money::from_soles(20.0).unwrap(), 50).unwrap()],
        })
        .unwrap();

    let create_exp = CreateExpense::new(repo.clone());
    let list_exp = ListExpenses::new(repo.clone());
    let update_exp = UpdateExpense::new(repo.clone());
    let delete_exp = DeleteExpense::new(repo.clone());
    let summary_uc = GetEventSummary::new(repo.clone());

    // Create expense
    let exp = create_exp
        .execute(CreateExpenseInput {
            evento_id: event.id.clone(),
            categoria: ExpenseCategory::SonidoLuces,
            monto: Money::from_soles(200.0).unwrap(),
            descripcion: "Alquiler monitores".to_string(),
            fecha: "2026-11-01".to_string(),
        })
        .expect("create expense");
    let exp_id = exp.id.clone();

    // List expenses
    let expenses = list_exp.execute(&event.id).expect("list expenses");
    assert_eq!(expenses.len(), 1);

    // Summary reflects expense
    let sum = summary_uc.execute(&event.id).expect("summary");
    assert_eq!(sum.gastos.to_soles(), 200.0);
    assert_eq!(sum.ganancia_neta_soles, -200.0);

    // Update expense
    let upd_exp = update_exp
        .execute(UpdateExpenseInput {
            id: exp_id.clone(),
            categoria: ExpenseCategory::SonidoLuces,
            monto: Money::from_soles(250.0).unwrap(),
            descripcion: "Alquiler monitores y cableado".to_string(),
            fecha: "2026-11-01".to_string(),
        })
        .expect("update expense");
    assert_eq!(upd_exp.monto.to_soles(), 250.0);

    // Delete expense
    delete_exp.execute(&exp_id).expect("delete expense");
    assert_eq!(list_exp.execute(&event.id).unwrap().len(), 0);
}
