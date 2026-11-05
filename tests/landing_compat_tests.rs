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

    // The panel is not reachable from the public page at all. Its only way in is
    // its own host, behind Cloudflare Access. An anonymous visitor who reaches
    // `/api/agent/chat` spends real provider tokens, so a convenience link on the
    // landing is not convenience: it is the shortest path to a paid endpoint.
    assert!(
        !landing.contains("/panel"),
        "the public landing must contain no reference to the panel: it is served \
         behind Access on its own host, and a link here exposes it to anyone"
    );

    let router = fs::read_to_string("src/router.rs").expect("read router.rs");
    assert!(
        router.contains(r#".route("/panel""#),
        "the panel must be served at /panel"
    );
}

#[test]
fn test_the_landing_never_forwards_a_visitor_into_the_panel() {
    // This document used to rewrite any hash route (`#/eventos`, `#/panel`, ...)
    // into `/panel#/...`. That handed the panel to anyone who typed a fragment,
    // without ever knowing the path: the shortest way around Access.
    //
    // The landing now answers on its own. Ticket links are shown by the landing
    // itself, so nothing needs forwarding and no client-side redirect exists to
    // be abused.
    let landing = fs::read_to_string("frontend/landing/index.html")
        .expect("frontend/landing/index.html must exist");

    assert!(
        !landing.contains("location.replace"),
        "the landing must not rewrite its own location: the only redirect it had \
         pointed at the panel, and a client-side redirect to a protected host is a \
         bypass with extra steps"
    );
    assert!(
        !landing.contains("panelViews"),
        "the hash-route forwarder must be gone, not merely disabled"
    );
}

#[test]
fn the_landing_does_not_load_the_panel_bundle() {
    // A ticket page on the public host must stay a leaf: if it pulled the panel
    // bundle it would drag the whole application, its API client and its chat
    // surface onto the anonymous host.
    let landing = fs::read_to_string("frontend/landing/index.html")
        .expect("frontend/landing/index.html must exist");

    assert!(
        !landing.contains("/assets/"),
        "the landing must ship no asset from the panel bundle"
    );
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

/// `:has()` needs Chrome 105+ / Safari 15.4+ / Firefox 121+ (all 2023). The
/// video tiles already carry `data-video="1"`, and an attribute selector works
/// everywhere, so the landing must not depend on `:has()` for layout.
#[test]
fn test_landing_does_not_depend_on_has_selector() {
    let css = fs::read_to_string("frontend/landing/landing.css")
        .expect("frontend/landing/landing.css must exist");
    // Prose is not code: a comment may name the selector it warns about, so
    // strip comments before looking for real rules.
    let mut css_rules = String::with_capacity(css.len());
    let chars: Vec<char> = css.chars().collect();
    let mut i = 0;
    let mut in_comment = false;
    while i < chars.len() {
        let next = chars.get(i + 1).copied();
        if !in_comment && chars[i] == '/' && next == Some('*') {
            in_comment = true;
            i += 2;
            continue;
        }
        if in_comment && chars[i] == '*' && next == Some('/') {
            in_comment = false;
            i += 2;
            continue;
        }
        if !in_comment {
            css_rules.push(chars[i]);
        }
        i += 1;
    }
    assert!(
        !css_rules.contains(":has("),
        "landing.css must not use :has(): older mobile browsers drop the rule \
         and the video label collides with the play disc again"
    );

    let html = fs::read_to_string("frontend/landing/index.html")
        .expect("frontend/landing/index.html must exist");
    assert!(
        html.contains("data-video=\"1\""),
        "video tiles must expose data-video=\"1\" so CSS can target them \
         without :has()"
    );
}
