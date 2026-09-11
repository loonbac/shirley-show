use std::sync::Arc;

use crate::error::AppError;
use crate::ports::presentation_writer::{DeleteResult, PresentationWriter};

pub struct DeletePresentation<W: PresentationWriter> {
    writer: Arc<W>,
}

impl<W: PresentationWriter> DeletePresentation<W> {
    pub fn new(writer: Arc<W>) -> Self {
        Self { writer }
    }

    pub fn execute(&self, id: &str) -> Result<DeleteResult, AppError> {
        self.writer.delete(id)
    }
}
