use axum::Json;
use axum::extract::State;

use crate::error::AppError;
use crate::handlers::dto::ListPresentationsResponse;
use crate::state::AppState;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<ListPresentationsResponse>, AppError> {
    let presentations = state.list_presentations.execute()?;
    Ok(Json(ListPresentationsResponse { presentations }))
}
