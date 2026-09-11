use crate::domain::ShowItem;
use crate::error::AppError;

pub trait ShowItemReader: Send + Sync {
    fn list_by_presentation(&self, presentation_id: &str) -> Result<Vec<ShowItem>, AppError>;
    fn get(&self, presentation_id: &str, item_id: &str) -> Result<Option<ShowItem>, AppError>;
    fn get_by_id(&self, item_id: &str) -> Result<Option<ShowItem>, AppError>;
}
