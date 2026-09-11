use std::sync::Arc;

use crate::domain::move_direction::MoveDirection;
use crate::domain::show_item::ShowItem;
use crate::error::AppError;
use crate::ports::show_item_writer::ShowItemWriter;

pub struct ReorderItem<W: ShowItemWriter> {
    writer: Arc<W>,
}

impl<W: ShowItemWriter> ReorderItem<W> {
    pub fn new(writer: Arc<W>) -> Self {
        Self { writer }
    }

    pub fn execute(
        &self,
        presentation_id: &str,
        item_id: &str,
        direction: MoveDirection,
    ) -> Result<Vec<ShowItem>, AppError> {
        self.writer.swap(presentation_id, item_id, direction)
    }
}
