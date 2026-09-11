use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowItemUpdate {
    TrackTitle(String),
    TextBody(String),
}

impl ShowItemUpdate {
    pub fn new_track_title(title: impl Into<String>) -> Result<Self, AppError> {
        let trimmed = title.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::bad_request(
                "title_required",
                "El título no puede estar vacío.",
            ));
        }
        Ok(Self::TrackTitle(trimmed))
    }

    pub fn new_text_body(body: impl Into<String>) -> Result<Self, AppError> {
        let trimmed = body.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::bad_request(
                "body_required",
                "El contenido no puede estar vacío.",
            ));
        }
        Ok(Self::TextBody(trimmed))
    }
}
