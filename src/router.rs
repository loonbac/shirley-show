use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use tower_http::services::ServeDir;

use crate::handlers::{agent, audio, items, presentations, static_assets};
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let frontend_dir = state.frontend_dir.clone();

    Router::new()
        .route("/", get(static_assets::serve_index))
        .nest_service("/assets", ServeDir::new(frontend_dir))
        .route("/api/presentations", get(presentations::list::list))
        .route("/api/presentations", post(presentations::create::create))
        .route("/api/presentations/{id}", get(presentations::get::get))
        .route(
            "/api/presentations/{id}",
            patch(presentations::rename::rename),
        )
        .route(
            "/api/presentations/{id}",
            axum::routing::delete(presentations::delete::delete),
        )
        .route(
            "/api/presentations/{presentation_id}/items",
            post(items::add_text::add_text),
        )
        .route(
            "/api/presentations/{presentation_id}/tracks",
            post(items::upload_track::upload_track).layer(DefaultBodyLimit::max(101 * 1024 * 1024)),
        )
        .route(
            "/api/presentations/{presentation_id}/items/{item_id}/move",
            post(items::reorder::reorder),
        )
        .route(
            "/api/presentations/{presentation_id}/items/{item_id}",
            patch(items::update::update).delete(items::delete::delete),
        )
        .route(
            "/api/tracks/{id}/audio",
            get(audio::stream::stream).head(audio::stream::stream),
        )
        .route(
            "/api/agent/chat",
            post(agent::chat).layer(DefaultBodyLimit::max(101 * 1024 * 1024)),
        )
        .route("/api/agent/gpu-status", get(agent::gpu_status))
        .with_state(state)
}
