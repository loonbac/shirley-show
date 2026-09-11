use std::sync::Arc;

use crate::domain::show_item::{LyricsPayload, ShowItem, ShowItemPayload};
use crate::error::AppError;
use crate::ports::show_item_writer::ShowItemWriter;

pub struct AddLyrics<W: ShowItemWriter> {
    writer: Arc<W>,
}

impl<W: ShowItemWriter> AddLyrics<W> {
    pub fn new(writer: Arc<W>) -> Self {
        Self { writer }
    }

    pub fn execute(&self, presentation_id: &str, body: &str) -> Result<ShowItem, AppError> {
        let payload = LyricsPayload::new(body)?;
        self.writer
            .append(presentation_id, ShowItemPayload::Lyrics(payload))
    }
}
