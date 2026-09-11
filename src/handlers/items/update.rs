use axum::extract::{Path, State};
use axum::Json;

use crate::error::AppError;
use crate::handlers::dto::{ShowItemResponse, UpdateItemRequest};
use crate::state::AppState;

pub async fn update(
    State(state): State<AppState>,
    Path((presentation_id, item_id)): Path<(String, String)>,
    Json(payload): Json<UpdateItemRequest>,
) -> Result<Json<ShowItemResponse>, AppError> {
    let item = state
        .update_show_item
        .execute(&presentation_id, &item_id, payload.title, payload.body)?;

    Ok(Json(ShowItemResponse::from(&item)))
}
