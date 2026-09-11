use crate::domain::presentation_summary::PresentationSummary;
use crate::error::AppError;

pub trait PresentationSummaryReader: Send + Sync {
    fn list_summaries(&self) -> Result<Vec<PresentationSummary>, AppError>;
}
