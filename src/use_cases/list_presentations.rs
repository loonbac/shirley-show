use std::sync::Arc;

use crate::domain::Presentation;
use crate::error::AppError;
use crate::ports::presentation_reader::PresentationReader;

pub struct ListPresentations<R: PresentationReader> {
    reader: Arc<R>,
}

impl<R: PresentationReader> ListPresentations<R> {
    pub fn new(reader: Arc<R>) -> Self {
        Self { reader }
    }

    pub fn execute(&self) -> Result<Vec<Presentation>, AppError> {
        self.reader.list()
    }
}
