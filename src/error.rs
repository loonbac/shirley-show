use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {message}")]
    BadRequest { code: &'static str, message: String },

    #[error("Validation error: {message}")]
    Validation {
        code: &'static str,
        message: String,
        field: Option<String>,
    },

    #[error("Resource not found: {message}")]
    NotFound { code: &'static str, message: String },

    #[error("Conflict: {message}")]
    Conflict { code: &'static str, message: String },

    #[error("Payload too large: {message}")]
    PayloadTooLarge { code: &'static str, message: String },

    #[error("Unsupported media type: {message}")]
    UnsupportedMediaType { code: &'static str, message: String },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            message: message.into(),
        }
    }

    pub fn validation(
        code: &'static str,
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
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

    pub fn payload_too_large(code: &'static str, message: impl Into<String>) -> Self {
        Self::PayloadTooLarge {
            code,
            message: message.into(),
        }
    }

    pub fn unsupported_media_type(code: &'static str, message: impl Into<String>) -> Self {
        Self::UnsupportedMediaType {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::BadRequest { code, .. } => code,
            Self::Validation { code, .. } => code,
            Self::NotFound { code, .. } => code,
            Self::Conflict { code, .. } => code,
            Self::PayloadTooLarge { code, .. } => code,
            Self::UnsupportedMediaType { code, .. } => code,
            Self::Database(_) => "database_error",
            Self::Internal(_) => "internal_error",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::BadRequest { message, .. } => message,
            Self::Validation { message, .. } => message,
            Self::NotFound { message, .. } => message,
            Self::Conflict { message, .. } => message,
            Self::PayloadTooLarge { message, .. } => message,
            Self::UnsupportedMediaType { message, .. } => message,
            Self::Database(msg) => msg,
            Self::Internal(msg) => msg,
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
            AppError::BadRequest { code, message } => (
                StatusCode::BAD_REQUEST,
                ErrorBody {
                    code: code.to_string(),
                    message,
                    field: None,
                },
            ),
            AppError::Validation {
                code,
                message,
                field,
            } => (
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
            AppError::PayloadTooLarge { code, message } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                ErrorBody {
                    code: code.to_string(),
                    message,
                    field: None,
                },
            ),
            AppError::UnsupportedMediaType { code, message } => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
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
