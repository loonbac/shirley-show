use axum::routing::{get, patch, post};
use axum::Router;

use crate::handlers::presentations;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/presentations", get(presentations::list::list))
        .route("/api/presentations", post(presentations::create::create))
        .route("/api/presentations/{id}", get(presentations::get::get))
        .route("/api/presentations/{id}", patch(presentations::rename::rename))
        .route(
            "/api/presentations/{id}",
            axum::routing::delete(presentations::delete::delete),
        )
        .with_state(state)
}
