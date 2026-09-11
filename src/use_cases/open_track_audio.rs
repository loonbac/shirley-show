use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::show_item::{ShowItem, ShowItemPayload};
use crate::error::AppError;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::ports::show_item_reader::ShowItemReader;

#[derive(Debug, Clone)]
pub struct OpenedAudioTrack {
    pub item: ShowItem,
    pub path: PathBuf,
    pub media_type: String,
    pub byte_size: u64,
}

pub struct OpenTrackAudio<R: ShowItemReader> {
    reader: Arc<R>,
    storage: Arc<LocalAudioStorage>,
}

impl<R: ShowItemReader> OpenTrackAudio<R> {
    pub fn new(reader: Arc<R>, storage: Arc<LocalAudioStorage>) -> Self {
        Self { reader, storage }
    }

    pub fn execute(&self, track_id: &str) -> Result<OpenedAudioTrack, AppError> {
        let item = self
            .reader
            .get_by_id(track_id)?
            .ok_or_else(|| AppError::not_found("track_not_found", "Pista no encontrada."))?;

        let (audio_key, media_type, expected_size) = match &item.payload {
            ShowItemPayload::Track(t) => (t.audio_key.clone(), t.media_type.clone(), t.byte_size),
            _ => {
                return Err(AppError::bad_request(
                    "not_a_track",
                    "El elemento solicitado no es una pista de audio.",
                ));
            }
        };

        let path = self.storage.resolve_path_str(&audio_key)?;
        if !path.exists() {
            return Err(AppError::not_found(
                "audio_unavailable",
                "El archivo de audio no se encuentra disponible en disco.",
            ));
        }

        let metadata = std::fs::metadata(&path)
            .map_err(|e| AppError::Internal(format!("Failed to read audio file metadata: {e}")))?;

        let byte_size = metadata.len();
        if byte_size == 0 && expected_size > 0 {
            return Err(AppError::not_found(
                "audio_unavailable",
                "El archivo de audio está vacío o corrupto.",
            ));
        }

        Ok(OpenedAudioTrack {
            item,
            path,
            media_type,
            byte_size,
        })
    }
}
