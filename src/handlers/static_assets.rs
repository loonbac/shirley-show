use axum::body::Bytes;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use http_body_util::Full;

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

/// Public landing: what a visitor sees at the site root.
pub async fn serve_index(State(state): State<AppState>) -> Response {
    serve_html(&state.frontend_dir, "landing/index.html").await
}

/// The management panel, moved off the root so the landing can live there.
///
/// Old links to `/#/ticket/<token>` and friends are forwarded to `/panel` with
/// their hash preserved, so tickets already sold and shared keep working.
pub async fn serve_panel(State(state): State<AppState>) -> Response {
    serve_html(&state.frontend_dir, "index.html").await
}
