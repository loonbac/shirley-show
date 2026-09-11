use crate::domain::move_direction::MoveDirection;
use crate::domain::show_item::{ShowItem, ShowItemPayload};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct DeleteItemResult {
    pub deleted_id: String,
    pub items: Vec<ShowItem>,
    pub audio_key: Option<String>,
}

pub trait ShowItemWriter: Send + Sync {
    fn append(&self, presentation_id: &str, payload: ShowItemPayload) -> Result<ShowItem, AppError>;
    fn swap(
        &self,
        presentation_id: &str,
        item_id: &str,
        direction: MoveDirection,
    ) -> Result<Vec<ShowItem>, AppError>;
    fn delete(&self, presentation_id: &str, item_id: &str) -> Result<DeleteItemResult, AppError>;
}
