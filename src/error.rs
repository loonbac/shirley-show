use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation error: {message}")]
    Validation {
        code: &'static str,
        message: String,
        field: Option<String>,
    },

    #[error("Resource not found: {message}")]
    NotFound {
        code: &'static str,
        message: String,
    },

    #[error("Conflict: {message}")]
    Conflict {
        code: &'static str,
        message: String,
    },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn validation(code: &'static str, message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::Validation {
            code,
            message: message.into(),
            field: Some(field.into()),
        }
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::NotFound {
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::Validation { code, message, field } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                ErrorBody {
                    code: code.to_string(),
                    message,
                    field,
                },
            ),
            AppError::NotFound { code, message } => (
                StatusCode::NOT_FOUND,
                ErrorBody {
                    code: code.to_string(),
                    message,
                    field: None,
                },
            ),
            AppError::Conflict { code, message } => (
                StatusCode::CONFLICT,
                ErrorBody {
                    code: code.to_string(),
                    message,
                    field: None,
                },
            ),
            AppError::Database(_err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "database_error".to_string(),
                    message: "Database error occurred".to_string(),
                    field: None,
                },
            ),
            AppError::Internal(_err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "internal_error".to_string(),
                    message: "Internal server error occurred".to_string(),
                    field: None,
                },
            ),
        };

        (status, Json(ErrorResponse { error: body })).into_response()
    }
}
