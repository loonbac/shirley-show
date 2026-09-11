use std::sync::Arc;

use chrono::Utc;

use crate::domain::show_item::ShowItem;
use crate::domain::show_item_kind::ShowItemKind;
use crate::domain::show_item_update::ShowItemUpdate;
use crate::error::AppError;
use crate::ports::show_item_reader::ShowItemReader;
use crate::ports::show_item_writer::ShowItemWriter;

pub struct UpdateShowItem<R: ShowItemReader, W: ShowItemWriter> {
    reader: Arc<R>,
    writer: Arc<W>,
}

impl<R: ShowItemReader, W: ShowItemWriter> UpdateShowItem<R, W> {
    pub fn new(reader: Arc<R>, writer: Arc<W>) -> Self {
        Self { reader, writer }
    }

    pub fn execute(
        &self,
        presentation_id: &str,
        item_id: &str,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<ShowItem, AppError> {
        match (&title, &body) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(AppError::bad_request(
                    "invalid_update_fields",
                    "Debe proporcionar exactamente uno de 'title' o 'body'.",
                ));
            }
            _ => {}
        }

        let item = self
            .reader
            .get(presentation_id, item_id)?
            .ok_or_else(|| AppError::not_found("item_not_found", "Elemento no encontrado."))?;

        let update = match (item.kind(), title, body) {
            (ShowItemKind::Track, Some(t), None) => {
                let trimmed = t.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::bad_request(
                        "empty_update_value",
                        "El título no puede estar vacío.",
                    ));
                }
                ShowItemUpdate::TrackTitle(trimmed)
            }
            (ShowItemKind::Track, None, Some(_)) => {
                return Err(AppError::bad_request(
                    "incompatible_field",
                    "No se puede actualizar 'body' en una pista de audio.",
                ));
            }
            (ShowItemKind::Lyrics | ShowItemKind::Note, None, Some(b)) => {
                let trimmed = b.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::bad_request(
                        "empty_update_value",
                        "El contenido no puede estar vacío.",
                    ));
                }
                ShowItemUpdate::TextBody(trimmed)
            }
            (ShowItemKind::Lyrics | ShowItemKind::Note, Some(_), None) => {
                return Err(AppError::bad_request(
                    "incompatible_field",
                    "No se puede actualizar 'title' en una letra o nota.",
                ));
            }
            _ => unreachable!(),
        };

        let now = Utc::now().to_rfc3339();
        self.writer.update(presentation_id, item_id, update, &now)
    }
}
