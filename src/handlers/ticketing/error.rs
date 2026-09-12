use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, ErrorBody, ErrorResponse};

#[derive(Debug)]
pub struct TicketingError(pub AppError);

impl IntoResponse for TicketingError {
    fn into_response(self) -> Response {
        match self.0 {
            AppError::Validation {
                code,
                message,
                field,
            } => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: code.to_string(),
                        message,
                        field,
                    },
                }),
            )
                .into_response(),
            AppError::Conflict { code, message }
                if code == "event_finalized" || code == "capacity_exceeded" =>
            {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: ErrorBody {
                            code: code.to_string(),
                            message,
                            field: None,
                        },
                    }),
                )
                    .into_response()
            }
            other => other.into_response(),
        }
    }
}

impl From<AppError> for TicketingError {
    fn from(err: AppError) -> Self {
        TicketingError(err)
    }
}

pub fn ticketing_error_response(err: AppError) -> Response {
    TicketingError(err).into_response()
}
