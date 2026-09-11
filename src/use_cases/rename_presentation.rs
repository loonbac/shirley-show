use chrono::Utc;
use std::sync::Arc;

use crate::domain::Presentation;
use crate::error::AppError;
use crate::ports::presentation_writer::PresentationWriter;

pub struct RenamePresentation<W: PresentationWriter> {
    writer: Arc<W>,
}

impl<W: PresentationWriter> RenamePresentation<W> {
    pub fn new(writer: Arc<W>) -> Self {
        Self { writer }
    }

    pub fn execute(&self, id: &str, new_title: &str) -> Result<Presentation, AppError> {
        let trimmed = new_title.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation(
                "title_required",
                "Escribe un nombre.",
                "title",
            ));
        }

        let updated_at = Utc::now().to_rfc3339();
        self.writer.rename(id, trimmed, &updated_at)
    }
}
