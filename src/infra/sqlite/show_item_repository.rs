use chrono::Utc;
use rusqlite::params;

use crate::domain::move_direction::MoveDirection;
use crate::domain::show_item::{ShowItem, ShowItemPayload};
use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;
use crate::ports::show_item_reader::ShowItemReader;
use crate::ports::show_item_writer::{DeleteItemResult, ShowItemWriter};

#[derive(Clone)]
pub struct SqliteShowItemRepository {
    pool: DbPool,
}

impl SqliteShowItemRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_item(row: &rusqlite::Row<'_>) -> Result<ShowItem, AppError> {
        ShowItem::from_db(
            row.get(0).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(1).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(2).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(3).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(4).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(5).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(6).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(7).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(8).map_err(|e| AppError::Database(e.to_string()))?,
            row.get(9).map_err(|e| AppError::Database(e.to_string()))?,
        )
    }
}

impl ShowItemReader for SqliteShowItemRepository {
    fn list_by_presentation(&self, presentation_id: &str) -> Result<Vec<ShowItem>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, presentation_id, position, kind, title, body, audio_key, media_type, byte_size, created_at
                 FROM show_items
                 WHERE presentation_id = ?1
                 ORDER BY position ASC;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare list items query: {e}")))?;

        let mut rows = stmt
            .query([presentation_id])
            .map_err(|e| AppError::Database(format!("Failed to execute list items query: {e}")))?;

        let mut items = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| AppError::Database(format!("Failed to fetch row: {e}")))?
        {
            items.push(Self::row_to_item(row)?);
        }

        Ok(items)
    }

    fn get(&self, presentation_id: &str, item_id: &str) -> Result<Option<ShowItem>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, presentation_id, position, kind, title, body, audio_key, media_type, byte_size, created_at
                 FROM show_items
                 WHERE presentation_id = ?1 AND id = ?2;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare get item query: {e}")))?;

        let mut rows = stmt
            .query(params![presentation_id, item_id])
            .map_err(|e| AppError::Database(format!("Failed to execute get item query: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| AppError::Database(format!("Failed to fetch item row: {e}")))?
        {
            Ok(Some(Self::row_to_item(row)?))
        } else {
            Ok(None)
        }
    }

    fn get_by_id(&self, item_id: &str) -> Result<Option<ShowItem>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, presentation_id, position, kind, title, body, audio_key, media_type, byte_size, created_at
                 FROM show_items
                 WHERE id = ?1;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare get item by id query: {e}")))?;

        let mut rows = stmt
            .query([item_id])
            .map_err(|e| AppError::Database(format!("Failed to execute get item by id query: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| AppError::Database(format!("Failed to fetch item row: {e}")))?
        {
            Ok(Some(Self::row_to_item(row)?))
        } else {
            Ok(None)
        }
    }
}

impl ShowItemWriter for SqliteShowItemRepository {
    fn append(&self, presentation_id: &str, payload: ShowItemPayload) -> Result<ShowItem, AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(format!("Failed to begin immediate transaction: {e}")))?;

        let pres_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM presentations WHERE id = ?1;",
                [presentation_id],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to check presentation existence: {e}")))?;

        if pres_exists == 0 {
            return Err(AppError::not_found(
                "presentation_not_found",
                "Presentación no encontrada.",
            ));
        }

        let next_pos: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM show_items WHERE presentation_id = ?1;",
                [presentation_id],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to calculate next position: {e}")))?;

        let item = ShowItem::new(presentation_id, next_pos, payload);

        let kind = item.kind().as_str();
        let (title, body, audio_key, media_type, byte_size) = match &item.payload {
            ShowItemPayload::Track(t) => (
                Some(t.title.as_str()),
                None,
                Some(t.audio_key.as_str()),
                Some(t.media_type.as_str()),
                Some(t.byte_size),
            ),
            ShowItemPayload::Lyrics(l) => (None, Some(l.body.as_str()), None, None, None),
            ShowItemPayload::Note(n) => (None, Some(n.body.as_str()), None, None, None),
        };

        tx.execute(
            "INSERT INTO show_items (id, presentation_id, position, kind, title, body, audio_key, media_type, byte_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            params![
                &item.id,
                &item.presentation_id,
                item.position,
                kind,
                title,
                body,
                audio_key,
                media_type,
                byte_size,
                &item.created_at,
            ],
        )
        .map_err(|e| AppError::Database(format!("Failed to insert show item: {e}")))?;

        tx.commit()
            .map_err(|e| AppError::Database(format!("Failed to commit append transaction: {e}")))?;

        Ok(item)
    }

    fn swap(
        &self,
        presentation_id: &str,
        item_id: &str,
        direction: MoveDirection,
    ) -> Result<Vec<ShowItem>, AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(format!("Failed to begin immediate transaction: {e}")))?;

        let pres_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM presentations WHERE id = ?1;",
                [presentation_id],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to check presentation existence: {e}")))?;

        if pres_exists == 0 {
            return Err(AppError::not_found(
                "presentation_not_found",
                "Presentación no encontrada.",
            ));
        }

        let curr_pos: Option<i64> = tx
            .query_row(
                "SELECT position FROM show_items WHERE presentation_id = ?1 AND id = ?2;",
                params![presentation_id, item_id],
                |r| r.get(0),
            )
            .ok();

        let curr_pos = match curr_pos {
            Some(p) => p,
            None => {
                return Err(AppError::not_found(
                    "item_not_found",
                    "Elemento no encontrado en esta presentación.",
                ))
            }
        };

        let max_pos: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(position), 0) FROM show_items WHERE presentation_id = ?1;",
                [presentation_id],
                |r| r.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to query max position: {e}")))?;

        let target_pos = match direction {
            MoveDirection::Up => {
                if curr_pos == 0 {
                    return Err(AppError::conflict(
                        "reorder_boundary",
                        "El elemento ya está al inicio de la presentación.",
                    ));
                }
                curr_pos - 1
            }
            MoveDirection::Down => {
                if curr_pos >= max_pos {
                    return Err(AppError::conflict(
                        "reorder_boundary",
                        "El elemento ya está al final de la presentación.",
                    ));
                }
                curr_pos + 1
            }
        };

        let temp_pos = max_pos + 1000;

        // Step 1: move current item to temporary safe position
        tx.execute(
            "UPDATE show_items SET position = ?1 WHERE presentation_id = ?2 AND id = ?3;",
            params![temp_pos, presentation_id, item_id],
        )
        .map_err(|e| AppError::Database(format!("Failed to shift item to temp position: {e}")))?;

        // Step 2: move neighbor item to curr_pos
        tx.execute(
            "UPDATE show_items SET position = ?1 WHERE presentation_id = ?2 AND position = ?3;",
            params![curr_pos, presentation_id, target_pos],
        )
        .map_err(|e| AppError::Database(format!("Failed to move neighbor item: {e}")))?;

        // Step 3: move current item to target_pos
        tx.execute(
            "UPDATE show_items SET position = ?1 WHERE presentation_id = ?2 AND id = ?3;",
            params![target_pos, presentation_id, item_id],
        )
        .map_err(|e| AppError::Database(format!("Failed to move item to target position: {e}")))?;

        tx.commit()
            .map_err(|e| AppError::Database(format!("Failed to commit swap transaction: {e}")))?;

        self.list_by_presentation(presentation_id)
    }

    fn delete(&self, presentation_id: &str, item_id: &str) -> Result<DeleteItemResult, AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| AppError::Database(format!("Failed to begin immediate transaction: {e}")))?;

        let item_data: Option<(i64, Option<String>)> = tx
            .query_row(
                "SELECT position, audio_key FROM show_items WHERE presentation_id = ?1 AND id = ?2;",
                params![presentation_id, item_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();

        let (del_pos, audio_key) = match item_data {
            Some(d) => d,
            None => {
                return Err(AppError::not_found(
                    "item_not_found",
                    "Elemento no encontrado en esta presentación.",
                ))
            }
        };

        if let Some(ref key) = audio_key {
            let now = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT OR IGNORE INTO pending_audio_deletions (audio_key, created_at, attempts) VALUES (?1, ?2, 0);",
                params![key, now],
            )
            .map_err(|e| AppError::Database(format!("Failed to queue pending audio deletion: {e}")))?;
        }

        tx.execute(
            "DELETE FROM show_items WHERE presentation_id = ?1 AND id = ?2;",
            params![presentation_id, item_id],
        )
        .map_err(|e| AppError::Database(format!("Failed to delete show item: {e}")))?;

        // Recompact positions: shift by 1,000,000 first, then bring back to (pos - 1)
        tx.execute(
            "UPDATE show_items SET position = position + 1000000 WHERE presentation_id = ?1 AND position > ?2;",
            params![presentation_id, del_pos],
        )
        .map_err(|e| AppError::Database(format!("Failed to shift positions forward: {e}")))?;

        tx.execute(
            "UPDATE show_items SET position = position - 1000000 - 1 WHERE presentation_id = ?1 AND position >= 1000000;",
            params![presentation_id],
        )
        .map_err(|e| AppError::Database(format!("Failed to recompact positions: {e}")))?;

        tx.commit()
            .map_err(|e| AppError::Database(format!("Failed to commit delete transaction: {e}")))?;

        let items = self.list_by_presentation(presentation_id)?;

        Ok(DeleteItemResult {
            deleted_id: item_id.to_string(),
            items,
            audio_key,
        })
    }
}
