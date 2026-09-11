use std::sync::Arc;

use crate::domain::show_item::ShowItem;
use crate::error::AppError;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::ports::cleanup_queue::CleanupQueue;
use crate::ports::show_item_writer::ShowItemWriter;

pub struct DeleteItemOutput {
    pub deleted_id: String,
    pub items: Vec<ShowItem>,
    pub cleanup_pending: bool,
}

pub struct DeleteItem<W: ShowItemWriter, C: CleanupQueue> {
    writer: Arc<W>,
    storage: Arc<LocalAudioStorage>,
    cleanup: Arc<C>,
}

impl<W: ShowItemWriter, C: CleanupQueue> DeleteItem<W, C> {
    pub fn new(writer: Arc<W>, storage: Arc<LocalAudioStorage>, cleanup: Arc<C>) -> Self {
        Self {
            writer,
            storage,
            cleanup,
        }
    }

    pub fn execute(
        &self,
        presentation_id: &str,
        item_id: &str,
    ) -> Result<DeleteItemOutput, AppError> {
        let res = self.writer.delete(presentation_id, item_id)?;

        let mut cleanup_pending = false;
        if let Some(ref key) = res.audio_key {
            match self.storage.delete_file(key) {
                Ok(_) => {
                    let _ = self.cleanup.mark_completed(key);
                }
                Err(e) => {
                    let _ = self.cleanup.record_failure(key, &e.to_string());
                    cleanup_pending = true;
                }
            }
        }

        Ok(DeleteItemOutput {
            deleted_id: res.deleted_id,
            items: res.items,
            cleanup_pending,
        })
    }
}
