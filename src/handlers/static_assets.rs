use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use http_body_util::Full;

use crate::config::PublicSurface;
use crate::state::AppState;

/// Serves a file from the frontend directory as HTML.
///
/// The public landing and the panel are separate documents on purpose: the
/// landing must render without the panel bundle, and the panel keeps working
/// at its own path exactly as before.
async fn serve_html(frontend_dir: &std::path::Path, relative: &str) -> Response {
    let index_path = frontend_dir.join(relative);

    match tokio::fs::read(&index_path).await {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(axum::body::Body::new(Full::new(Bytes::from(content))))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// The document served at `/`, decided by the host that asked.
///
/// The door runs on its own host, but it is a **mode of the panel application**,
/// not a page of the landing: the doorman needs `main.js` to enter door mode, and
/// the landing never loads it. Serving the landing there answered 200 and made
/// the door unreachable — a 200 that hides a failure is worse than a 404.
///
/// The decision is made per request from the same `Host` the surface guard uses,
/// so the two layers cannot disagree about which site this is.
pub async fn serve_index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    let host = headers.get(header::HOST).and_then(|value| value.to_str().ok());

    // The panel host serves the panel at its root, the same way the door serves
    // the door: `panel.shirleyshow.site` is the owner's entry point, and showing
    // them the public landing there would make the one host they were told to
    // use look like the site they already had.
    //
    // A single-host deployment keeps the landing at `/`: there `panel_host` is
    // empty, the comparison is false, and nothing changes for the existing setup
    // or its tests.
    let serves_panel = match state.surface.classify(host) {
        PublicSurface::Door => true,
        PublicSurface::Application => state.surface.is_panel_host(host),
        _ => false,
    };

    if serves_panel {
        serve_html(&state.frontend_dir, "index.html").await
    } else {
        serve_html(&state.frontend_dir, "landing/index.html").await
    }
}

/// The management panel, moved off the root so the landing can live there.
///
/// Reached at `/panel` on the panel host. It is **not** linked from the landing
/// and the landing does not forward to it: the panel lives behind Access on its
/// own host, and `/api/agent/chat` spends paid tokens on every anonymous call.
/// Old `/#/ticket/<token>` links are no longer forwarded; printed QR codes never
/// depended on that forwarding because they carry no domain and no route.
pub async fn serve_panel(State(state): State<AppState>) -> Response {
    serve_html(&state.frontend_dir, "index.html").await
}
