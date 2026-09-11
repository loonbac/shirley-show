use std::path::{Path, PathBuf};

/// RAII guard that reverts a promoted audio file back to its staging path
/// if dropped before being explicitly defused.
pub struct StagedPromotionGuard {
    stage_path: PathBuf,
    promoted_path: PathBuf,
    active: bool,
}

impl StagedPromotionGuard {
    pub fn new(stage_path: PathBuf, promoted_path: PathBuf) -> Self {
        Self {
            stage_path,
            promoted_path,
            active: true,
        }
    }

    pub fn stage_path(&self) -> &Path {
        &self.stage_path
    }

    pub fn promoted_path(&self) -> &Path {
        &self.promoted_path
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for StagedPromotionGuard {
    fn drop(&mut self) {
        if self.active && self.promoted_path.exists() {
            // Attempt to revert the promoted file back to the staging location
            if std::fs::rename(&self.promoted_path, &self.stage_path).is_err() {
                // If rollback rename fails, remove promoted file to avoid orphaned managed files
                let _ = std::fs::remove_file(&self.promoted_path);
            }
        }
    }
}
