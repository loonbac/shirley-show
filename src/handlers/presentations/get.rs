use axum::Json;
use axum::extract::{Path, State};

use crate::error::AppError;
use crate::handlers::dto::{PresentationDetailResponse, ShowItemResponse};
use crate::ports::show_item_reader::ShowItemReader;
use crate::state::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PresentationDetailResponse>, AppError> {
    let presentation = state.get_presentation.execute(&id)?;
    let items = state
        .item_repo
        .list_by_presentation(&id)?
        .iter()
        .map(ShowItemResponse::from)
        .collect();

    Ok(Json(PresentationDetailResponse {
        presentation,
        items,
    }))
}
