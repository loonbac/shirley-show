use axum::extract::{Path, State};
use axum::Json;

use crate::error::AppError;
use crate::handlers::dto::PresentationDetailResponse;
use crate::state::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PresentationDetailResponse>, AppError> {
    let presentation = state.get_presentation.execute(&id)?;
    Ok(Json(PresentationDetailResponse {
        presentation,
        items: Vec::new(),
    }))
}
