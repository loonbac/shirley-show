use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;
use crate::ports::cleanup_queue::CleanupQueue;

#[derive(Clone)]
pub struct SqliteCleanupRepository {
    pool: DbPool,
}

impl SqliteCleanupRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl CleanupQueue for SqliteCleanupRepository {
    fn get_pending(&self) -> Result<Vec<String>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT audio_key FROM pending_audio_deletions ORDER BY created_at ASC;")
            .map_err(|e| AppError::Database(format!("Failed to prepare get_pending query: {e}")))?;

        let rows = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("Failed to query pending deletions: {e}")))?;

        let mut keys = Vec::new();
        for r in rows {
            keys.push(r.map_err(|e| AppError::Database(format!("Failed to read audio_key: {e}")))?);
        }

        Ok(keys)
    }

    fn mark_completed(&self, audio_key: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        conn.execute(
            "DELETE FROM pending_audio_deletions WHERE audio_key = ?1;",
            [audio_key],
        )
        .map_err(|e| AppError::Database(format!("Failed to delete pending deletion: {e}")))?;

        Ok(())
    }

    fn record_failure(&self, audio_key: &str, error: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        conn.execute(
            "UPDATE pending_audio_deletions SET attempts = attempts + 1, last_error = ?1 WHERE audio_key = ?2;",
            [error, audio_key],
        )
        .map_err(|e| AppError::Database(format!("Failed to record pending deletion failure: {e}")))?;

        Ok(())
    }
}
