use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::error::AppError;
use crate::handlers::dto::{CreateItemRequest, ShowItemResponse};
use crate::state::AppState;

pub async fn add_text(
    State(state): State<AppState>,
    Path(presentation_id): Path<String>,
    Json(payload): Json<CreateItemRequest>,
) -> Result<(StatusCode, Json<ShowItemResponse>), AppError> {
    let item = match payload.item_type.trim().to_lowercase().as_str() {
        "lyrics" => state.add_lyrics.execute(&presentation_id, &payload.body)?,
        "note" => state.add_note.execute(&presentation_id, &payload.body)?,
        _ => {
            return Err(AppError::validation(
                "invalid_item_type",
                "El tipo de elemento debe ser 'lyrics' o 'note'.",
                "type",
            ));
        }
    };

    Ok((StatusCode::CREATED, Json(ShowItemResponse::from(&item))))
}
