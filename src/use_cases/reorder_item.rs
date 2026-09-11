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

    pub fn execute_target_position(
        &self,
        presentation_id: &str,
        item_id: &str,
        target_position: i64,
    ) -> Result<Vec<ShowItem>, AppError> {
        self.writer.move_to(presentation_id, item_id, target_position)
    }
}
