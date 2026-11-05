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
fn test_the_door_accepts_both_ticket_payload_forms() {
    // Two forms describe the same credential and the door has to open for both:
    //
    //   1. the bare 32-hex secret, which is what every ticket printed so far
    //      carries. It must keep working: a printed ticket cannot be reissued.
    //   2. the same secret wrapped in a URL, for tickets printed from now on.
    //
    // What must NOT happen is mining a 32-hex prefix out of the buyer's link
    // (`st_` + 64 hex). That string contains 32 hex in a row, so a naive
    // `match(/[0-9a-fA-F]{32}/)` extracts a plausible secret that belongs to
    // nobody and refuses a ticket that is perfectly valid.
    let controller = fs::read_to_string("frontend/js/modules/events/door-controller.js")
        .expect("read door-controller.js");

    assert!(
        controller.contains("DOOR_SECRET_PATTERN"),
        "the door must parse the credential with an explicit pattern"
    );

    // The boundaries are the whole fix: without the lookahead a 40-hex string is
    // sliced into a valid-looking secret, and without the lookbehind a longer
    // run is matched from an arbitrary offset.
    assert!(
        controller.contains("(?!["),
        "the pattern must not slice a 32-hex prefix out of a longer hex run"
    );
    assert!(
        controller.contains("(?:"),
        "the pattern must anchor to a boundary before the secret"
    );
    assert!(
        controller.contains("st_[0-9a-fA-F]{64}"),
        "the door must recognise the buyer's link and name it, instead of \
         reporting a printing defect that does not exist"
    );

    // The printed payload stays the 32-hex secret, so moving the site to another
    // domain cannot invalidate a ticket already in someone's hands.
    let ticket = fs::read_to_string("src/domain/ticketing/ticket.rs").expect("read ticket.rs");
    assert!(
        ticket.contains("cadena hexadecimal de 32 caracteres"),
        "the printed secret stays 32 hex: retrocompatibility is not optional"
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

#[test]
fn the_public_ticket_never_carries_the_raw_scan_secret() {
    // Measured in production against the real event: the JSON the public route
    // returned included `codigo_qr` in plain hex.
    //
    // The SVG has to stay: without it the buyer cannot enter the venue, and the
    // WhatsApp link is how the ticket reaches them. The `scan_token` in that link
    // is the bearer credential (64 hex, unguessable), so the page can show the
    // code.
    //
    // What must not appear is the raw hex string. It adds no capability the SVG
    // does not already carry, but it turns the secret into something anyone can
    // copy, paste and forward without decoding an image.
    let dto = fs::read_to_string("src/use_cases/ticketing/tickets.rs")
        .expect("read tickets.rs");

    let start = dto
        .find("pub struct PublicTicketDto")
        .expect("PublicTicketDto must exist");
    let end = start + dto[start..].find('}').expect("struct must close");
    let block = &dto[start..end];

    assert!(
        !block.contains("pub codigo_qr:"),
        "the public ticket must not carry the raw scan secret in its JSON: the          SVG is the entry credential, the hex string is only a copyable leak"
    );
    assert!(
        block.contains("pub qr_svg:"),
        "the buyer still needs the QR image: it is their entry credential"
    );
}

#[test]
fn the_printed_qr_payload_is_decided_by_the_server_not_the_client() {
    // The ticket photo is drawn in the browser, so the QR payload could easily be
    // derived from `window.location` there. That would print the *panel* domain,
    // which sits behind Cloudflare Access: a buyer scanning their own ticket would
    // land on a login screen instead of their ticket.
    //
    // The server owns the payload and the client only draws it.
    let photo = fs::read_to_string("frontend/js/modules/events/ticket-photo.js")
        .expect("read ticket-photo.js");

    assert!(
        photo.contains("entrada.qr_payload"),
        "the printed QR payload must come from the server"
    );

    let qr_value_start = photo
        .find("let valorQR")
        .expect("the photo builder must compute the QR value");
    let qr_value_block = &photo[qr_value_start..photo.len().min(qr_value_start + 600)];

    assert!(
        !qr_value_block.contains("window.location") && !qr_value_block.contains("location.origin"),
        "the QR payload must never be derived from the browser's own location"
    );

    // And the server must actually send it.
    let dto = fs::read_to_string("src/handlers/ticketing/dto.rs").expect("read dto.rs");
    assert!(
        dto.contains("pub qr_payload: Option<String>"),
        "the authenticated DTO must carry the server-decided payload"
    );
}
