use axum::extract::{Path, State};
use axum::Json;

use crate::domain::move_direction::MoveDirection;
use crate::error::AppError;
use crate::handlers::dto::{MoveItemRequest, ReorderItemsResponse, ShowItemResponse};
use crate::state::AppState;

pub async fn reorder(
    State(state): State<AppState>,
    Path((presentation_id, item_id)): Path<(String, String)>,
    Json(payload): Json<MoveItemRequest>,
) -> Result<Json<ReorderItemsResponse>, AppError> {
    let direction: MoveDirection = payload.direction.parse()?;
    let items = state
        .reorder_item
        .execute(&presentation_id, &item_id, direction)?;

    let dtos = items.iter().map(ShowItemResponse::from).collect();
    Ok(Json(ReorderItemsResponse { items: dtos }))
}
