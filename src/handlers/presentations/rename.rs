use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::{ErrorBody, ErrorResponse};
use crate::handlers::dto::{PresentationResponse, RenamePresentationRequest};
use crate::state::AppState;

pub async fn rename(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<RenamePresentationRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match payload {
        Ok(body) => body,
        Err(rejection) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: "invalid_request".to_string(),
                        message: rejection.body_text(),
                        field: None,
                    },
                }),
            )
                .into_response();
        }
    };

    match state.rename_presentation.execute(&id, &request.title) {
        Ok(presentation) => (
            StatusCode::OK,
            Json(PresentationResponse { presentation }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}
