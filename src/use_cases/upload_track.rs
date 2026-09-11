use std::sync::Arc;

use crate::domain::audio_format::AudioFormat;
use crate::domain::show_item::{ShowItem, ShowItemPayload, TrackPayload};
use crate::error::AppError;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::ports::show_item_writer::ShowItemWriter;

pub struct UploadTrack<W: ShowItemWriter> {
    writer: Arc<W>,
    storage: Arc<LocalAudioStorage>,
}

impl<W: ShowItemWriter> UploadTrack<W> {
    pub fn new(writer: Arc<W>, storage: Arc<LocalAudioStorage>) -> Self {
        Self { writer, storage }
    }

    pub fn execute(
        &self,
        presentation_id: &str,
        title: &str,
        stage_id: &str,
        format: AudioFormat,
        byte_size: i64,
    ) -> Result<ShowItem, AppError> {
        // 1. Promote staged file to managed files directory
        let key = self.storage.promote_staged_file(stage_id, format)?;

        // 2. Build track payload
        let payload_result = TrackPayload::new(title, key.as_str(), format.media_type(), byte_size);
        let payload = match payload_result {
            Ok(p) => p,
            Err(e) => {
                // Compensation: remove promoted file
                let _ = self.storage.delete_file(key.as_str());
                return Err(e);
            }
        };

        // 3. Persist item to database
        match self.writer.append(presentation_id, ShowItemPayload::Track(payload)) {
            Ok(item) => Ok(item),
            Err(e) => {
                // Compensation: remove promoted file from disk
                let _ = self.storage.delete_file(key.as_str());
                Err(e)
            }
        }
    }
}
