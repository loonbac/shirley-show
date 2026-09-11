use crate::error::AppError;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::ports::cleanup_queue::CleanupQueue;

#[derive(Debug, Clone, Default)]
pub struct ReconcileReport {
    pub processed_deletions: usize,
    pub failed_deletions: usize,
    pub purged_staging: usize,
    pub purged_vocal_work: usize,
}

pub fn reconcile_audio_files(
    storage: &LocalAudioStorage,
    cleanup_repo: &impl CleanupQueue,
) -> Result<ReconcileReport, AppError> {
    let mut report = ReconcileReport::default();

    // 1. Process pending deletions
    let pending_keys = cleanup_repo.get_pending()?;
    for key in pending_keys {
        match storage.delete_file(&key) {
            Ok(_) => {
                let _ = cleanup_repo.mark_completed(&key);
                report.processed_deletions += 1;
            }
            Err(e) => {
                let _ = cleanup_repo.record_failure(&key, &e.to_string());
                report.failed_deletions += 1;
            }
        }
    }

    // 2. Purge orphan staging files
    if let Ok(entries) = std::fs::read_dir(storage.staging_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && std::fs::remove_file(&path).is_ok() {
                report.purged_staging += 1;
            }
        }
    }

    // 3. Purge orphan vocal work directories and files
    if let Ok(entries) = std::fs::read_dir(storage.vocal_work_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if std::fs::remove_dir_all(&path).is_ok() {
                    report.purged_vocal_work += 1;
                }
            } else if path.is_file() && std::fs::remove_file(&path).is_ok() {
                report.purged_vocal_work += 1;
            }
        }
    }

    Ok(report)
}
