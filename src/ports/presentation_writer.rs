use serde::Serialize;

use crate::domain::Presentation;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct DeleteResult {
    pub deleted_id: String,
    pub cleanup_pending: bool,
}

pub trait PresentationWriter: Send + Sync {
    fn create(&self, presentation: &Presentation) -> Result<(), AppError>;
    fn rename(&self, id: &str, title: &str, updated_at: &str) -> Result<Presentation, AppError>;
    fn delete(&self, id: &str) -> Result<DeleteResult, AppError>;
}
