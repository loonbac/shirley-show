use crate::error::AppError;

pub trait CleanupQueue: Send + Sync {
    fn get_pending(&self) -> Result<Vec<String>, AppError>;
    fn mark_completed(&self, audio_key: &str) -> Result<(), AppError>;
    fn record_failure(&self, audio_key: &str, error: &str) -> Result<(), AppError>;
}
