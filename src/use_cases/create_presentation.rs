use std::sync::Arc;

use crate::domain::Presentation;
use crate::error::AppError;
use crate::ports::presentation_writer::PresentationWriter;

pub struct CreatePresentation<W: PresentationWriter> {
    writer: Arc<W>,
}

impl<W: PresentationWriter> CreatePresentation<W> {
    pub fn new(writer: Arc<W>) -> Self {
        Self { writer }
    }

    pub fn execute(&self, title: &str) -> Result<Presentation, AppError> {
        let presentation = Presentation::new(title)?;
        self.writer.create(&presentation)?;
        Ok(presentation)
    }
}
