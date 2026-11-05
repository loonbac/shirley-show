//! Guards the compatibility rules that keep a live event working.
//!
//! The system is in production with tickets already sold, printed and shared.
//! These tests exist because moving the panel to a subpath would silently break
//! every link already in someone's phone if the old URL stopped working.
//!
//! They read source and static files rather than issuing HTTP requests: what
//! must never regress is the shape of the contract, not one response.

use std::fs;

#[test]
fn test_public_landing_and_panel_are_separate_documents() {
    let landing = fs::read_to_string("frontend/landing/index.html")
        .expect("frontend/landing/index.html must exist");

    assert!(
        landing.contains("Shirley Show"),
        "the landing must present the artist"
    );

    // The landing must not load the panel bundle: it has to render on its own.
    assert!(
        !landing.contains("/assets/js/main.js"),
        "the landing must not depend on the panel bundle"
    );

    // The panel must remain reachable from the landing.
    assert!(
        landing.contains(r#"href="/panel""#),
        "the landing must link to the panel so the owner can still get in"
    );

    let router = fs::read_to_string("src/router.rs").expect("read router.rs");
    assert!(
        router.contains(r#".route("/panel""#),
        "the panel must be served at /panel"
    );
}

#[test]
fn test_old_ticket_links_are_forwarded_to_the_panel() {
    // Ticket links are hash routes (`/#/ticket/<token>`). A hash never reaches
    // the server, so the forward has to happen in the landing document itself.
    let landing = fs::read_to_string("frontend/landing/index.html")
        .expect("frontend/landing/index.html must exist");

    assert!(
        landing.contains("/panel"),
        "the landing must know the panel path"
    );
    assert!(
        landing.contains("location.hash"),
        "the landing must inspect the hash to forward legacy links"
    );
    assert!(
        landing.contains("location.replace"),
        "forwarding must replace history, not stack a back step on a ticket scan"
    );

    // Every hash route that already exists in the world must be forwarded.
    for view in [
        "ticket",
        "eventos",
        "presentations",
        "puerta",
    ] {
        assert!(
            landing.contains(view),
            "legacy hash route `{view}` must be forwarded, or existing links break"
        );
    }
}

#[test]
fn test_ticket_qr_payload_does_not_embed_a_domain() {
    // The QR code printed on a ticket holds the 32-hex secret and nothing else,
    // so moving the site to another domain cannot invalidate a printed ticket.
    let ticket = fs::read_to_string("src/domain/ticketing/ticket.rs")
        .expect("read ticket.rs");

    assert!(
        ticket.contains("cadena hexadecimal de 32 caracteres"),
        "the QR secret must stay a 32-character hex string"
    );

    // A URL in the QR payload would bind printed tickets to one domain.
    let qr_block_start = ticket
        .find("QrSecret")
        .expect("the ticket must have a QR secret type");
    let qr_block = &ticket[qr_block_start..ticket.len().min(qr_block_start + 4000)];
    assert!(
        !qr_block.contains("https://") && !qr_block.contains("http://"),
        "the QR payload must never embed a domain"
    );
}

#[test]
fn test_landing_assets_are_present() {
    for asset in [
        "frontend/landing/landing.css",
        "frontend/landing/img/hero.png",
        "frontend/landing/img/logo.png",
        "frontend/landing/img/sobre.png",
    ] {
        assert!(
            std::path::Path::new(asset).exists(),
            "landing asset `{asset}` is missing"
        );
    }
}
