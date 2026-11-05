//! Refuses the application surface on a host that may only serve the landing.
//!
//! `/api/agent/chat` has no authentication layer of its own, so the panel host
//! is the only thing protecting it. A border rule (Cloudflare Access) is one
//! configuration change away from being wrong, and anything that reaches the
//! container directly bypasses the border entirely. This guard is the second
//! line, in the application itself.

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
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

    let wants_application = path.starts_with("/api/") || path == "/panel" || path.starts_with("/panel/");

    // `Closed` matters as much as `Landing`: a request that arrives with no
    // Host header, an IP, or a foreign domain is not a visitor of the landing
    // and must not reach an API that has no auth layer.
    if wants_application && surface != PublicSurface::Application {
        return StatusCode::NOT_FOUND.into_response();
    }

    next.run(request).await
}
