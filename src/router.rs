use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use tower_http::services::ServeDir;

use crate::handlers::{agent, audio, items, presentations, static_assets, ticketing};
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
        .route(
            "/api/eventos",
            get(ticketing::events::list).post(ticketing::events::create),
        )
        .route(
            "/api/eventos/{id}",
            get(ticketing::events::get)
                .patch(ticketing::events::update)
                .delete(ticketing::events::delete),
        )
        .route(
            "/api/eventos/{id}/finalizar",
            post(ticketing::events::finalize),
        )
        .route("/api/eventos/{id}/reabrir", post(ticketing::events::reopen))
        .route(
            "/api/eventos/{id}/imagen",
            get(ticketing::events::get_poster)
                .post(ticketing::events::upload_poster)
                .layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
        .route(
            "/api/eventos/{id}/gastos",
            get(ticketing::expenses::list).post(ticketing::expenses::create),
        )
        .route(
            "/api/gastos/{id}",
            patch(ticketing::expenses::update).delete(ticketing::expenses::delete),
        )
        .route(
            "/api/eventos/{id}/resumen",
            get(ticketing::summary::get_summary),
        )
        .route(
            "/api/eventos/{id}/entradas",
            get(ticketing::tickets::list).post(ticketing::tickets::sell),
        )
        .route(
            "/api/entradas/{id}",
            get(ticketing::tickets::get)
                .patch(ticketing::tickets::update_contact)
                .delete(ticketing::tickets::delete),
        )
        .route(
            "/api/entradas/{id}/confirmar-pago",
            post(ticketing::tickets::confirm_payment),
        )
        .route("/api/entradas/checkin", post(ticketing::tickets::check_in))
        .route(
            "/api/public/ticket/{scan_token}",
            get(ticketing::tickets::get_public_ticket),
        )
        .route(
            "/api/eventos/{id}/interesados",
            get(ticketing::interests::list).post(ticketing::interests::create),
        )
        .route(
            "/api/interesados/{id}",
            get(ticketing::interests::get)
                .patch(ticketing::interests::update)
                .delete(ticketing::interests::delete),
        )
        .route(
            "/api/interesados/{id}/vender",
            post(ticketing::interests::convert),
        )
        .with_state(state)
}
