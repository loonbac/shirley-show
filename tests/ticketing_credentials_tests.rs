use std::collections::HashSet;

use shirley_show::domain::ticketing::Money;
use shirley_show::domain::ticketing::ticket::{
    CheckinStatus, PaymentMethod, PaymentStatus, Ticket,
};
use shirley_show::infra::random_credentials::OsRandomCredentials;
use shirley_show::ports::ticketing::CredentialGenerator;

#[test]
fn test_random_credentials_generation_format_and_uniqueness() {
    let generator = OsRandomCredentials::new();

    let mut scan_tokens = HashSet::new();
    let mut qr_secrets = HashSet::new();
    let mut ids = HashSet::new();

    for _ in 0..50 {
        let creds = generator.generate().expect("credential generation failed");

        // ID must be a valid 36-character UUID string
        assert_eq!(creds.id.len(), 36);
        assert!(ids.insert(creds.id));

        // scan_token must start with "st_" and have 64 hex chars (total 67)
        assert!(creds.scan_token.as_str().starts_with("st_"));
        assert_eq!(creds.scan_token.as_str().len(), 67);
        let scan_hex = &creds.scan_token.as_str()[3..];
        assert!(scan_hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(scan_tokens.insert(creds.scan_token.as_str().to_string()));

        // codigo_qr must be 32 hex chars, lowercase
        assert_eq!(creds.qr_secret.as_str().len(), 32);
        assert!(
            creds
                .qr_secret
                .as_str()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert!(qr_secrets.insert(creds.qr_secret.as_str().to_string()));

        // scan_token and qr_secret must be uncorrelated
        assert_ne!(creds.scan_token.as_str(), creds.qr_secret.as_str());
        assert!(!creds.scan_token.as_str().contains(creds.qr_secret.as_str()));
    }
}

#[test]
fn test_qr_svg_rendered_only_when_ticket_is_paid() {
    let generator = OsRandomCredentials::new();
    let creds = generator.generate().unwrap();

    let paid_ticket = Ticket::new(
        creds.id.clone(),
        "evt-123".to_string(),
        None,
        "Carlos Mendoza".to_string(),
        "987654321".to_string(),
        "General".to_string(),
        Money::from_soles(30.0).unwrap(),
        PaymentMethod::Yape,
        PaymentStatus::Pagado,
        creds.scan_token.clone(),
        creds.qr_secret.clone(),
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    )
    .expect("paid ticket creation failed");

    // Paid ticket must render a valid SVG string
    let svg = paid_ticket.render_qr_svg(None);
    assert!(svg.is_some(), "paid ticket must generate QR SVG");
    let svg_str = svg.unwrap();
    assert!(svg_str.contains("<svg"), "must be an SVG element");
    assert!(svg_str.contains("</svg>"), "must close the SVG element");
    // Plaintext QR secret string shouldn't be printed as raw text in the SVG
    // (though QR modules encode it, the string itself shouldn't be plain XML text)
    assert!(!svg_str.contains(&format!(">{}<", creds.qr_secret.as_str())));

    let creds2 = generator.generate().unwrap();
    let pending_ticket = Ticket::new(
        creds2.id,
        "evt-123".to_string(),
        None,
        "Ana Torres".to_string(),
        "912345678".to_string(),
        "VIP".to_string(),
        Money::from_soles(50.0).unwrap(),
        PaymentMethod::Efectivo,
        PaymentStatus::Pendiente,
        creds2.scan_token,
        creds2.qr_secret,
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    )
    .expect("pending ticket creation failed");

    // Pending ticket MUST NOT render QR SVG
    assert_eq!(
        pending_ticket.render_qr_svg(None),
        None,
        "pending ticket must never generate QR SVG"
    );
}

#[test]
fn test_ticket_domain_validation_and_courtesy_invariants() {
    let generator = OsRandomCredentials::new();
    let creds = generator.generate().unwrap();

    // Courtesy must be zero price and paid
    let cortesia = Ticket::new(
        creds.id.clone(),
        "evt-1".to_string(),
        None,
        "Invitado Especial".to_string(),
        "999888777".to_string(),
        "Cortesía".to_string(),
        Money::zero(),
        PaymentMethod::Cortesia,
        PaymentStatus::Pagado,
        creds.scan_token.clone(),
        creds.qr_secret.clone(),
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    )
    .expect("valid cortesia ticket");

    assert_eq!(cortesia.precio, Money::zero());
    assert_eq!(cortesia.estado_pago, PaymentStatus::Pagado);
    assert_eq!(cortesia.medio_pago, PaymentMethod::Cortesia);

    // Reject Courtesy with non-zero price
    let err = Ticket::new(
        creds.id.clone(),
        "evt-1".to_string(),
        None,
        "Invitado Especial".to_string(),
        "999888777".to_string(),
        "Cortesía".to_string(),
        Money::from_soles(20.0).unwrap(),
        PaymentMethod::Cortesia,
        PaymentStatus::Pagado,
        creds.scan_token.clone(),
        creds.qr_secret.clone(),
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    );
    assert!(
        err.is_err(),
        "courtesy with non-zero price must be rejected"
    );

    // Reject Courtesy with pending status
    let err2 = Ticket::new(
        creds.id.clone(),
        "evt-1".to_string(),
        None,
        "Invitado Especial".to_string(),
        "999888777".to_string(),
        "Cortesía".to_string(),
        Money::zero(),
        PaymentMethod::Cortesia,
        PaymentStatus::Pendiente,
        creds.scan_token.clone(),
        creds.qr_secret.clone(),
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    );
    assert!(
        err2.is_err(),
        "courtesy with pending status must be rejected"
    );

    // Reject check-in mismatch (Ingresó without hora_ingreso, or NoIngreso with hora_ingreso)
    let err3 = Ticket::new(
        creds.id.clone(),
        "evt-1".to_string(),
        None,
        "Comprador".to_string(),
        "999888777".to_string(),
        "General".to_string(),
        Money::from_soles(30.0).unwrap(),
        PaymentMethod::Yape,
        PaymentStatus::Pagado,
        creds.scan_token.clone(),
        creds.qr_secret.clone(),
        CheckinStatus::Ingreso,
        None, // missing hora_ingreso!
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    );
    assert!(err3.is_err(), "Ingresó without hora_ingreso must fail");

    // Check confirm_payment
    let mut ticket_to_pay = Ticket::new(
        creds.id,
        "evt-1".to_string(),
        None,
        "Comprador".to_string(),
        "999888777".to_string(),
        "General".to_string(),
        Money::from_soles(30.0).unwrap(),
        PaymentMethod::Efectivo,
        PaymentStatus::Pendiente,
        creds.scan_token,
        creds.qr_secret,
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    )
    .unwrap();

    assert_eq!(ticket_to_pay.render_qr_svg(None), None);
    ticket_to_pay.confirm_payment("2026-10-15T21:00:00Z".to_string());
    assert_eq!(ticket_to_pay.estado_pago, PaymentStatus::Pagado);
    assert_eq!(ticket_to_pay.estado_checkin, CheckinStatus::NoIngreso);
    assert!(ticket_to_pay.render_qr_svg(None).is_some());
}

#[test]
fn the_printed_qr_wraps_the_door_secret_and_never_the_buyer_link() {
    // Two forms describe the same credential and the door accepts both:
    // the bare 32-hex secret already printed, and that same secret inside a URL.
    //
    // New tickets get the URL form. It is the *door* secret on purpose:
    // `check_in` compares `codigo_qr`, so wrapping the buyer's `scan_token`
    // instead would print a QR no doorman could cash in -- the door refuses it
    // by design.
    let creds = OsRandomCredentials::new().generate().unwrap();
    let ticket = Ticket::new(
        creds.id,
        "evt-qr".to_string(),
        None,
        "Rosa Mendez".to_string(),
        "987654321".to_string(),
        "General".to_string(),
        Money::from_soles(20.0).unwrap(),
        PaymentMethod::Yape,
        PaymentStatus::Pagado,
        creds.scan_token.clone(),
        creds.qr_secret.clone(),
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    )
    .expect("paid ticket creation failed");

    // Form 1: no base URL configured -> the bare secret, exactly as before.
    assert_eq!(
        ticket.qr_payload(None),
        creds.qr_secret.as_str().to_string(),
        "without a base URL the payload must stay the bare 32-hex secret"
    );

    // A base URL that is not a URL cannot wrap anything: the bare form survives.
    assert_eq!(
        ticket.qr_payload(Some("shirleyshow.site")),
        creds.qr_secret.as_str().to_string(),
        "a base URL without a scheme must not be pasted onto the payload"
    );

    // Form 2: the new format.
    let wrapped = ticket.qr_payload(Some("https://shirleyshow.site/"));
    assert_eq!(
        wrapped,
        format!(
            "https://shirleyshow.site/#/ticket/{}",
            creds.qr_secret.as_str()
        ),
        "new tickets carry the door secret inside the public ticket URL"
    );

    // The printed QR must never carry the buyer's link token: the door refuses
    // it, so the ticket would be unusable at the venue.
    assert!(
        !wrapped.contains(creds.scan_token.as_str()),
        "the printed QR must not embed the scan_token: the door does not accept it"
    );

    // Both forms must remain scannable: the pattern the door uses has to extract
    // the secret out of either one.
    for payload in [creds.qr_secret.as_str().to_string(), wrapped] {
        let extracted = extract_door_secret(&payload);
        assert_eq!(
            extracted.as_deref(),
            Some(creds.qr_secret.as_str()),
            "the door must extract the same secret from `{payload}`"
        );
    }
}

/// Mirror of `DOOR_SECRET_PATTERN` in `door-controller.js`.
///
/// Duplicated in a test on purpose: if the two drift, the printed ticket stops
/// being scannable and nothing else in the suite would notice.
fn extract_door_secret(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    for start in 0..bytes.len().saturating_sub(31) {
        let boundary_before = start == 0 || !bytes[start - 1].is_ascii_hexdigit();
        if !boundary_before {
            continue;
        }
        let window = &bytes[start..start + 32];
        let all_hex = window.iter().all(|b| b.is_ascii_hexdigit());
        if !all_hex {
            continue;
        }
        let boundary_after =
            start + 32 == bytes.len() || !bytes[start + 32].is_ascii_hexdigit();
        if boundary_after {
            return Some(String::from_utf8_lossy(window).to_ascii_lowercase());
        }
    }
    None
}
