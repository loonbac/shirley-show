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

/// The landing and the printed photo both paint the brand logo over the middle
/// of the QR, which is only safe because the code carries 30% error recovery.
/// At the crate default (level M, 15%) the covered modules eat the budget and a
/// phone at the door fails to read the credential.
///
/// Asserted by measuring the module count: the same payload needs a larger
/// matrix at level H than at level M, so a silent revert to `QrCode::new` shows
/// up here instead of at the door.
#[test]
fn test_printed_qr_carries_enough_error_recovery_for_the_logo() {
    let generator = OsRandomCredentials::new();
    let creds = generator.generate().unwrap();

    let ticket = Ticket::new(
        creds.id,
        "evt-logo".to_string(),
        None,
        "Lucia Prado".to_string(),
        "987654321".to_string(),
        "General".to_string(),
        Money::from_soles(20.0).unwrap(),
        PaymentMethod::Efectivo,
        PaymentStatus::Pagado,
        creds.scan_token,
        creds.qr_secret,
        CheckinStatus::NoIngreso,
        None,
        "2026-10-15T20:00:00Z".to_string(),
        "2026-10-15T20:00:00Z".to_string(),
    )
    .expect("paid ticket creation failed");

    let svg = ticket
        .render_qr_svg(Some("https://shirleyshow.site"))
        .expect("paid ticket must render a QR");

    // The `qrcode` crate emits one `<path>` of `M<x> <y>h<w>v<h>` runs, where
    // the height of a run is exactly one module. Parse each run by its markers
    // instead of splitting on spaces: the runs are concatenated with no
    // separator, so `h5v5H20V20` is one token, not four.
    let mut min_step = usize::MAX;
    let mut min_x = usize::MAX;
    let mut max_x = 0usize;
    for chunk in svg.split('M').skip(1) {
        let (Some(hpos), Some(_)) = (chunk.find('h'), chunk.find('v')) else {
            continue;
        };
        let Some(vpos) = chunk.find('v') else { continue };
        if vpos <= hpos {
            continue;
        }
        let head = &chunk[..hpos];
        let w = &chunk[hpos + 1..vpos];
        let rest = &chunk[vpos + 1..];
        let hend = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let h = &rest[..hend];

        let mut head_parts = head.split_whitespace();
        let (Some(x), Some(_y)) = (head_parts.next(), head_parts.next()) else {
            continue;
        };
        let (Ok(x), Ok(w), Ok(h)) = (x.parse::<usize>(), w.parse::<usize>(), h.parse::<usize>())
        else {
            continue;
        };
        if h == 0 {
            continue;
        }
        min_step = min_step.min(h);
        min_x = min_x.min(x);
        max_x = max_x.max(x + w);
    }

    assert!(
        min_step != usize::MAX && max_x > min_x,
        "the QR SVG must contain module runs, not an empty path"
    );
    let modules = (max_x - min_x) / min_step;

    // A 32-hex or `https://shirleyshow.site/#/ticket/<hex>` payload at level H
    // is version 8 or larger (45x45). Level M would fit in 37x37.
    assert!(
        modules >= 45,
        "the printed QR must be encoded at level H so the centred logo stays \
         readable: a {modules}x{modules} matrix means level M came back"
    );
}
