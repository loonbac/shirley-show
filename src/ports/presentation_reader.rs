use crate::domain::Presentation;
use crate::error::AppError;

pub trait PresentationReader: Send + Sync {
    fn list(&self) -> Result<Vec<Presentation>, AppError>;
    fn get(&self, id: &str) -> Result<Option<Presentation>, AppError>;
}
