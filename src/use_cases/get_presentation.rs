use std::sync::Arc;

use crate::domain::Presentation;
use crate::error::AppError;
use crate::ports::presentation_reader::PresentationReader;

pub struct GetPresentation<R: PresentationReader> {
    reader: Arc<R>,
}

impl<R: PresentationReader> GetPresentation<R> {
    pub fn new(reader: Arc<R>) -> Self {
        Self { reader }
    }

    pub fn execute(&self, id: &str) -> Result<Presentation, AppError> {
        self.reader
            .get(id)?
            .ok_or_else(|| AppError::not_found("presentation_not_found", "Presentación no encontrada."))
    }
}
