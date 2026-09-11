use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;

/// RAII guard that automatically removes an unpromoted staging file on drop unless defused.
pub struct StagingFileGuard {
    storage: Arc<LocalAudioStorage>,
    stage_id: Option<String>,
    path: PathBuf,
}

impl StagingFileGuard {
    pub fn new(storage: Arc<LocalAudioStorage>, stage_id: String, path: PathBuf) -> Self {
        Self { storage, stage_id: Some(stage_id), path }
    }

    pub fn stage_id(&self) -> &str {
        self.stage_id.as_deref().unwrap_or("")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn defuse(mut self) {
        self.stage_id = None;
    }
}

impl Drop for StagingFileGuard {
    fn drop(&mut self) {
        if let Some(ref stage_id) = self.stage_id {
            let _ = self.storage.delete_staging(stage_id);
        }
    }
}
