use chrono::Utc;
use rusqlite::params;

use crate::domain::Presentation;
use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;
use crate::ports::presentation_reader::PresentationReader;
use crate::ports::presentation_writer::{DeleteResult, PresentationWriter};

#[derive(Clone)]
pub struct SqlitePresentationRepository {
    pool: DbPool,
}

impl SqlitePresentationRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl PresentationReader for SqlitePresentationRepository {
    fn list(&self) -> Result<Vec<Presentation>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, title, created_at, updated_at FROM presentations ORDER BY created_at ASC;")
            .map_err(|e| AppError::Database(format!("Failed to prepare list query: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Presentation::from_db(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            })
            .map_err(|e| AppError::Database(format!("Failed to query presentations: {e}")))?;

        let mut presentations = Vec::new();
        for r in rows {
            presentations.push(r.map_err(|e| AppError::Database(format!("Failed to read row: {e}")))?);
        }

        Ok(presentations)
    }

    fn get(&self, id: &str) -> Result<Option<Presentation>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, title, created_at, updated_at FROM presentations WHERE id = ?1;")
            .map_err(|e| AppError::Database(format!("Failed to prepare get query: {e}")))?;

        let mut rows = stmt
            .query([id])
            .map_err(|e| AppError::Database(format!("Failed to execute get query: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| AppError::Database(format!("Failed to fetch row: {e}")))?
        {
            Ok(Some(Presentation::from_db(
                row.get(0).map_err(|e| AppError::Database(e.to_string()))?,
                row.get(1).map_err(|e| AppError::Database(e.to_string()))?,
                row.get(2).map_err(|e| AppError::Database(e.to_string()))?,
                row.get(3).map_err(|e| AppError::Database(e.to_string()))?,
            )))
        } else {
            Ok(None)
        }
    }
}

impl PresentationWriter for SqlitePresentationRepository {
    fn create(&self, presentation: &Presentation) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        conn.execute(
            "INSERT INTO presentations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4);",
            params![
                &presentation.id,
                &presentation.title,
                &presentation.created_at,
                &presentation.updated_at
            ],
        )
        .map_err(|e| AppError::Database(format!("Failed to insert presentation: {e}")))?;

        Ok(())
    }

    fn rename(&self, id: &str, title: &str, updated_at: &str) -> Result<Presentation, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let rows_affected = conn
            .execute(
                "UPDATE presentations SET title = ?1, updated_at = ?2 WHERE id = ?3;",
                params![title, updated_at, id],
            )
            .map_err(|e| AppError::Database(format!("Failed to update presentation: {e}")))?;

        if rows_affected == 0 {
            return Err(AppError::not_found(
                "presentation_not_found",
                "Presentación no encontrada.",
            ));
        }

        self.get(id)?
            .ok_or_else(|| AppError::not_found("presentation_not_found", "Presentación no encontrada."))
    }

    fn delete(&self, id: &str) -> Result<DeleteResult, AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(format!("Failed to begin immediate transaction: {e}")))?;

        let count: i32 = tx
            .query_row(
                "SELECT COUNT(*) FROM presentations WHERE id = ?1;",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to check presentation existence: {e}")))?;

        if count == 0 {
            return Err(AppError::not_found(
                "presentation_not_found",
                "Presentación no encontrada.",
            ));
        }

        let now = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT OR IGNORE INTO pending_audio_deletions (audio_key, created_at, attempts)
             SELECT audio_key, ?1, 0 FROM show_items WHERE presentation_id = ?2 AND audio_key IS NOT NULL;",
            params![now, id],
        )
        .map_err(|e| AppError::Database(format!("Failed to queue pending audio deletions: {e}")))?;

        tx.execute("DELETE FROM presentations WHERE id = ?1;", [id])
            .map_err(|e| AppError::Database(format!("Failed to delete presentation: {e}")))?;

        tx.commit()
            .map_err(|e| AppError::Database(format!("Failed to commit delete transaction: {e}")))?;

        Ok(DeleteResult {
            deleted_id: id.to_string(),
            cleanup_pending: false,
        })
    }
}
