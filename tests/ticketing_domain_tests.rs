use shirley_show::domain::ticketing::{
    Event, EventSummary, Expense, ExpenseCategory, ExpenseCategoryBreakdown, Money,
    PaymentMethodBreakdown, TicketType,
};

#[test]
fn test_money_valid_creation_and_arithmetic() {
    let m1 = Money::from_soles(25.50).expect("valid soles");
    assert_eq!(m1.cents(), 2550);
    assert_eq!(m1.to_soles(), 25.50);

    let m2 = Money::from_cents(450);
    assert_eq!(m2.cents(), 450);
    assert_eq!(m2.to_soles(), 4.50);

    let sum = m1.add(&m2);
    assert_eq!(sum.cents(), 3000);
    assert_eq!(sum.to_soles(), 30.00);
}

#[test]
fn test_money_rejection_of_invalid_inputs() {
    assert!(Money::from_soles(-10.0).is_err(), "Negative soles rejected");
    assert!(Money::from_soles(f64::NAN).is_err(), "NaN rejected");
    assert!(
        Money::from_soles(f64::INFINITY).is_err(),
        "Infinity rejected"
    );
    assert!(
        Money::from_soles(10.555).is_err(),
        "More than 2 decimals rejected"
    );
}

#[test]
fn test_ticket_type_validation() {
    let valid = TicketType::new("General", Money::from_soles(30.0).unwrap(), 100);
    assert!(valid.is_ok());

    let empty_name = TicketType::new("   ", Money::from_soles(30.0).unwrap(), 100);
    assert!(empty_name.is_err(), "Empty trimmed name rejected");

    let zero_capacity = TicketType::new("General", Money::from_soles(30.0).unwrap(), 0);
    assert!(zero_capacity.is_err(), "Zero capacity rejected");
}

#[test]
fn test_event_validation_and_ticket_types() {
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
    );
    assert!(event.is_ok());
    let ev = event.unwrap();
    assert_eq!(ev.total_capacity(), 100);

    // Empty name rejected
    assert!(
        Event::new(
            "evt-2".to_string(),
            "   ".to_string(),
            "2026-10-15T21:00:00Z".to_string(),
            "Lugar".to_string(),
            vec![TicketType::new("General", Money::from_soles(10.0).unwrap(), 10).unwrap()],
            false,
            None,
            "2026-10-01T00:00:00Z".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
        )
        .is_err()
    );

    // Invalid RFC3339 date rejected
    assert!(
        Event::new(
            "evt-3".to_string(),
            "Nombre".to_string(),
            "invalid-date".to_string(),
            "Lugar".to_string(),
            vec![TicketType::new("General", Money::from_soles(10.0).unwrap(), 10).unwrap()],
            false,
            None,
            "2026-10-01T00:00:00Z".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
        )
        .is_err()
    );

    // Empty ticket types rejected
    assert!(
        Event::new(
            "evt-4".to_string(),
            "Nombre".to_string(),
            "2026-10-15T21:00:00Z".to_string(),
            "Lugar".to_string(),
            vec![],
            false,
            None,
            "2026-10-01T00:00:00Z".to_string(),
            "2026-10-01T00:00:00Z".to_string(),
        )
        .is_err()
    );
}

#[test]
fn test_expense_category_closed_set_and_expense_validation() {
    let cat_ok = ExpenseCategory::parse("Sonido/Luces");
    assert!(cat_ok.is_ok());
    assert_eq!(cat_ok.unwrap().as_str(), "Sonido/Luces");

    let cat_bad = ExpenseCategory::parse("Vestuario");
    assert!(cat_bad.is_err(), "Invalid category rejected");

    let exp = Expense::new(
        "exp-1".to_string(),
        "evt-1".to_string(),
        ExpenseCategory::parse("Transporte").unwrap(),
        Money::from_soles(150.0).unwrap(),
        "Taxi equipos".to_string(),
        "2026-10-14".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    );
    assert!(exp.is_ok());

    // Invalid date rejected
    let exp_bad_date = Expense::new(
        "exp-2".to_string(),
        "evt-1".to_string(),
        ExpenseCategory::parse("Transporte").unwrap(),
        Money::from_soles(150.0).unwrap(),
        "Taxi".to_string(),
        "not-a-date".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    );
    assert!(exp_bad_date.is_err());
}

#[test]
fn test_triangulate_edge_cases() {
    // 1. Zero prices allowed for tickets/expenses
    let zero_money = Money::from_soles(0.0).expect("zero soles");
    assert_eq!(zero_money.cents(), 0);
    assert_eq!(zero_money.to_soles(), 0.0);

    let cortesia_type = TicketType::new("Cortesía", zero_money, 20);
    assert!(cortesia_type.is_ok());

    let zero_expense = Expense::new(
        "exp-0".to_string(),
        "evt-1".to_string(),
        ExpenseCategory::BebidasCatering,
        zero_money,
        "Canje agua".to_string(),
        "2026-10-14".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    );
    assert!(zero_expense.is_ok());

    // 2. Duplicate ticket type names case-insensitive
    let dup_types = vec![
        TicketType::new("General", Money::from_soles(20.0).unwrap(), 50).unwrap(),
        TicketType::new("general", Money::from_soles(25.0).unwrap(), 20).unwrap(),
    ];
    let dup_event = Event::new(
        "evt-dup".to_string(),
        "Concierto".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "Teatro".to_string(),
        dup_types,
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    );
    assert!(
        dup_event.is_err(),
        "Case-insensitive duplicate ticket type name must be rejected"
    );

    // 3. Negative net profit (Gastos > Cobrado)
    let cobrado = Money::from_soles(200.0).unwrap();
    let pendiente = Money::from_soles(100.0).unwrap();
    let gastos = Money::from_soles(550.0).unwrap();

    let summary = EventSummary::new(
        cobrado,
        pendiente,
        gastos,
        100,
        10,
        15,
        4,
        PaymentMethodBreakdown::default(),
        ExpenseCategoryBreakdown::default(),
    );

    assert_eq!(summary.total_proyectado.to_soles(), 300.0);
    assert_eq!(summary.ganancia_neta, -35000); // in cents: 20000 - 55000 = -35000
    assert_eq!(summary.ganancia_neta_soles, -350.0);

    // 4. Whitespace trimming: strings with spaces are trimmed
    let space_event = Event::new(
        "evt-space".to_string(),
        "  Concierto Acústico  ".to_string(),
        "2026-10-15T21:00:00Z".to_string(),
        "  Barranco  ".to_string(),
        vec![TicketType::new("  VIP  ", Money::from_soles(50.0).unwrap(), 10).unwrap()],
        false,
        None,
        "2026-10-01T00:00:00Z".to_string(),
        "2026-10-01T00:00:00Z".to_string(),
    )
    .unwrap();

    assert_eq!(space_event.nombre, "Concierto Acústico");
    assert_eq!(space_event.lugar, "Barranco");
    assert_eq!(space_event.tipos_de_entrada[0].nombre, "VIP");
}
