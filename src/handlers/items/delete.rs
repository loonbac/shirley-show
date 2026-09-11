use axum::extract::{Path, State};
use axum::Json;

use crate::error::AppError;
use crate::handlers::dto::{DeleteItemResponse, ShowItemResponse};
use crate::state::AppState;

pub async fn delete(
    State(state): State<AppState>,
    Path((presentation_id, item_id)): Path<(String, String)>,
) -> Result<Json<DeleteItemResponse>, AppError> {
    let result = state.delete_item.execute(&presentation_id, &item_id)?;

    let dtos = result.items.iter().map(ShowItemResponse::from).collect();
    Ok(Json(DeleteItemResponse {
        deleted_id: result.deleted_id,
        items: dtos,
        cleanup_pending: result.cleanup_pending,
    }))
}
