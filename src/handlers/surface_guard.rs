//! Refuses the application surface on a host that may only serve the landing.
//!
//! `/api/agent/chat` has no authentication layer of its own, so the panel host
//! is the only thing protecting it. A border rule (Cloudflare Access) is one
//! configuration change away from being wrong, and anything that reaches the
//! container directly bypasses the border entirely. This guard is the second
//! line, in the application itself.

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::config::PublicSurface;
use crate::state::AppState;

/// Rejects requests that ask a landing host for the application.
///
/// A 404 is deliberate: the path does not exist on that host, so an anonymous
/// visitor learns nothing about what runs behind the panel host.
pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());

    let surface = state.surface.classify(host);
    let path = request.uri().path();
    let method = request.method().as_str();

    let wants_application =
        path.starts_with("/api/") || path == "/panel" || path.starts_with("/panel/");

    // One route is public by design: the buyer's proof of purchase. The ticket
    // link shared over WhatsApp is a hash route, so the landing has to fetch the
    // ticket itself, and the buyer is anonymous by definition. It is pinned to
    // the exact prefix, with no wildcard behind it: `/api/public/ticket/{token}`
    // reads one ticket by an unguessable token and does nothing else.
    let is_public_ticket_read =
        method == "GET" && path.starts_with("/api/public/ticket/");

    match surface {
        PublicSurface::Application => {}
        // The door is the only surface besides the panel that can write, and it
        // runs with a long session so it cannot be trusted with the whole
        // application. It gets a positive allowlist instead of a bare refusal.
        PublicSurface::Door => {
            if wants_application && !state.surface.door_allows(method, path) {
                return refused();
            }
        }
        // `Closed` matters as much as `Landing`: a request that arrives with no
        // Host header, an IP, or a foreign domain is not a visitor of the
        // landing and must not reach an API that has no auth layer.
        // Only the landing shows a buyer their ticket. `Closed` keeps refusing
        // everything: an unknown host is not a deployment that opted into
        // serving anonymous buyers.
        PublicSurface::Landing => {
            if wants_application && !is_public_ticket_read {
                return refused();
            }
        }
        PublicSurface::Closed => {
            if wants_application {
                return refused();
            }
        }
    }

    next.run(request).await
}

/// A refusal that can be told apart from a handler's own 404.
///
/// Both are "not found", which is what an anonymous visitor should see, but a
/// test (or an operator reading logs) needs to know which layer refused.
fn refused() -> Response {
    let mut response = StatusCode::NOT_FOUND.into_response();
    response
        .headers_mut()
        .insert("x-surface-refused", HeaderValue::from_static("1"));
    response
}
