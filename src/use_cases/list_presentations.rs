use std::sync::Arc;

use crate::domain::presentation_summary::PresentationSummary;
use crate::error::AppError;
use crate::ports::presentation_summary_reader::PresentationSummaryReader;

pub struct ListPresentations<R: PresentationSummaryReader> {
    reader: Arc<R>,
}

impl<R: PresentationSummaryReader> ListPresentations<R> {
    pub fn new(reader: Arc<R>) -> Self {
        Self { reader }
    }

    pub fn execute(&self) -> Result<Vec<PresentationSummary>, AppError> {
        self.reader.list_summaries()
    }
}
