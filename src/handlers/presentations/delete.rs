use axum::Json;
use axum::extract::{Path, State};

use crate::error::AppError;
use crate::handlers::dto::DeletePresentationResponse;
use crate::state::AppState;

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DeletePresentationResponse>, AppError> {
    let result = state.delete_presentation.execute(&id)?;
    Ok(Json(DeletePresentationResponse {
        deleted_id: result.deleted_id,
        cleanup_pending: result.cleanup_pending,
    }))
}
