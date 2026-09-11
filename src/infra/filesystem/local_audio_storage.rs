use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::domain::audio_format::AudioFormat;
use crate::error::AppError;
use crate::infra::filesystem::generated_audio_key::GeneratedAudioKey;
use crate::infra::filesystem::staged_promotion_guard::StagedPromotionGuard;

#[derive(Debug, Clone)]
pub struct LocalAudioStorage {
    files_dir: PathBuf,
    staging_dir: PathBuf,
    vocal_work_dir: PathBuf,
}

impl LocalAudioStorage {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base = base_dir.into();
        let files_dir = base.join("files");
        let staging_dir = base.join("staging");
        let vocal_work_dir = base.join("vocal-work");

        let _ = std::fs::create_dir_all(&files_dir);
        let _ = std::fs::create_dir_all(&staging_dir);
        let _ = std::fs::create_dir_all(&vocal_work_dir);

        Self {
            files_dir,
            staging_dir,
            vocal_work_dir,
        }
    }

    pub fn files_dir(&self) -> &Path {
        &self.files_dir
    }

    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }

    pub fn vocal_work_dir(&self) -> &Path {
        &self.vocal_work_dir
    }

    pub fn create_staging_file(&self) -> Result<(String, PathBuf), AppError> {
        let stage_id = Uuid::new_v4().to_string();
        let path = self.staging_dir.join(format!("{stage_id}.part"));
        let _ = std::fs::File::create(&path)
            .map_err(|e| AppError::Internal(format!("Failed to create staging file: {e}")))?;
        Ok((stage_id, path))
    }

    pub fn promote_staged_file_guarded(
        &self,
        stage_id: &str,
        format: AudioFormat,
    ) -> Result<(GeneratedAudioKey, StagedPromotionGuard), AppError> {
        let stage_path = self.staging_dir.join(format!("{stage_id}.part"));
        if !stage_path.exists() {
            return Err(AppError::NotFound {
                code: "staged_file_not_found",
                message: "Archivo temporal de audio no encontrado.".into(),
            });
        }

        let key = GeneratedAudioKey::generate(format);
        let dest_path = self.files_dir.join(key.as_str());

        std::fs::rename(&stage_path, &dest_path)
            .map_err(|e| AppError::Internal(format!("Failed to promote staged audio file: {e}")))?;

        let guard = StagedPromotionGuard::new(stage_path, dest_path);
        Ok((key, guard))
    }

    pub fn promote_staged_file(
        &self,
        stage_id: &str,
        format: AudioFormat,
    ) -> Result<GeneratedAudioKey, AppError> {
        let (key, mut guard) = self.promote_staged_file_guarded(stage_id, format)?;
        guard.defuse();
        Ok(key)
    }

    pub fn resolve_path(&self, key: &GeneratedAudioKey) -> PathBuf {
        self.files_dir.join(key.as_str())
    }

    pub fn resolve_path_str(&self, key_str: &str) -> Result<PathBuf, AppError> {
        let key = GeneratedAudioKey::parse(key_str)?;
        Ok(self.resolve_path(&key))
    }

    pub fn delete_file(&self, key_str: &str) -> Result<bool, AppError> {
        let path = self.resolve_path_str(key_str)?;
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| AppError::Internal(format!("Failed to delete audio file: {e}")))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn delete_staging(&self, stage_id: &str) -> Result<(), AppError> {
        let path = self.staging_dir.join(format!("{stage_id}.part"));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        Ok(())
    }
}
