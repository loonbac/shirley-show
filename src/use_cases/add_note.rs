use std::sync::Arc;

use crate::domain::show_item::{NotePayload, ShowItem, ShowItemPayload};
use crate::error::AppError;
use crate::ports::show_item_writer::ShowItemWriter;

pub struct AddNote<W: ShowItemWriter> {
    writer: Arc<W>,
}

impl<W: ShowItemWriter> AddNote<W> {
    pub fn new(writer: Arc<W>) -> Self {
        Self { writer }
    }

    pub fn execute(&self, presentation_id: &str, body: &str) -> Result<ShowItem, AppError> {
        let payload = NotePayload::new(body)?;
        self.writer.append(presentation_id, ShowItemPayload::Note(payload))
    }
}
