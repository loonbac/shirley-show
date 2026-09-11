use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::handlers::ticketing::dto::{EventSummaryDto, EventSummaryResponse};
use crate::state::AppState;

pub async fn get_summary(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.ticketing.get_summary.execute(&id) {
        Ok(summary) => (
            StatusCode::OK,
            Json(EventSummaryResponse {
                resumen: EventSummaryDto::from_domain(&summary),
            }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}
