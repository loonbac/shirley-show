use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::{ErrorBody, ErrorResponse};
use crate::handlers::dto::{CreatePresentationRequest, PresentationResponse};
use crate::state::AppState;

pub async fn create(
    State(state): State<AppState>,
    payload: Result<Json<CreatePresentationRequest>, JsonRejection>,
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

    match state.create_presentation.execute(&request.title) {
        Ok(presentation) => (
            StatusCode::CREATED,
            Json(PresentationResponse { presentation }),
        )
            .into_response(),
        Err(err) => err.into_response(),
    }
}
