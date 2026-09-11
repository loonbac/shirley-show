use axum::Json;
use axum::extract::{Path, State};

use crate::domain::move_direction::MoveDirection;
use crate::error::AppError;
use crate::handlers::dto::{MoveItemRequest, ReorderItemsResponse, ShowItemResponse};
use crate::state::AppState;

pub async fn reorder(
    State(state): State<AppState>,
    Path((presentation_id, item_id)): Path<(String, String)>,
    Json(payload): Json<MoveItemRequest>,
) -> Result<Json<ReorderItemsResponse>, AppError> {
    let items = match (payload.direction, payload.target_position) {
        (Some(dir_str), None) => {
            let direction: MoveDirection = dir_str.parse()?;
            state
                .reorder_item
                .execute(&presentation_id, &item_id, direction)?
        }
        (None, Some(target_pos)) => {
            state
                .reorder_item
                .execute_target_position(&presentation_id, &item_id, target_pos)?
        }
        _ => {
            return Err(AppError::bad_request(
                "ambiguous_reorder_selector",
                "Debe proporcionar exactamente un selector: 'direction' o 'target_position'.",
            ));
        }
    };

    let dtos = items.iter().map(ShowItemResponse::from).collect();
    Ok(Json(ReorderItemsResponse { items: dtos }))
}
