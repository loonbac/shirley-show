use crate::domain::presentation_summary::{PresentationSummary, ShowItemPreview};
use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;
use crate::ports::presentation_summary_reader::PresentationSummaryReader;

#[derive(Clone)]
pub struct PresentationSummaryRepository {
    pool: DbPool,
}

impl PresentationSummaryRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl PresentationSummaryReader for PresentationSummaryRepository {
    fn list_summaries(&self) -> Result<Vec<PresentationSummary>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Database(format!("Failed to acquire connection: {e}")))?;

        let mut stmt = conn
            .prepare(
                "WITH counts AS (
                     SELECT
                         presentation_id,
                         SUM(CASE WHEN kind = 'track' THEN 1 ELSE 0 END) AS tracks_count,
                         SUM(CASE WHEN kind = 'lyrics' THEN 1 ELSE 0 END) AS lyrics_count,
                         SUM(CASE WHEN kind = 'note' THEN 1 ELSE 0 END) AS notes_count,
                         COUNT(*) AS total_items
                     FROM show_items
                     GROUP BY presentation_id
                 ),
                 ranked_preview AS (
                     SELECT
                         id AS item_id,
                         presentation_id,
                         position,
                         kind,
                         title,
                         body,
                         ROW_NUMBER() OVER (PARTITION BY presentation_id ORDER BY position ASC) AS rnk
                     FROM show_items
                 )
                 SELECT
                     p.id,
                     p.title,
                     p.created_at,
                     p.updated_at,
                     COALESCE(c.tracks_count, 0) AS tracks_count,
                     COALESCE(c.lyrics_count, 0) AS lyrics_count,
                     COALESCE(c.notes_count, 0) AS notes_count,
                     COALESCE(c.total_items, 0) AS total_items,
                     rp.item_id,
                     rp.position,
                     rp.kind,
                     rp.title,
                     rp.body
                 FROM presentations p
                 LEFT JOIN counts c ON c.presentation_id = p.id
                 LEFT JOIN ranked_preview rp ON rp.presentation_id = p.id AND rp.rnk <= 3
                 ORDER BY p.created_at ASC, rp.position ASC;",
            )
            .map_err(|e| AppError::Database(format!("Failed to prepare list_summaries query: {e}")))?;

        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("Failed to execute list_summaries query: {e}")))?;

        let mut summaries: Vec<PresentationSummary> = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| AppError::Database(format!("Failed to fetch row: {e}")))?
        {
            let id: String = row.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            let title: String = row.get(1).map_err(|e| AppError::Database(e.to_string()))?;
            let created_at: String = row.get(2).map_err(|e| AppError::Database(e.to_string()))?;
            let updated_at: String = row.get(3).map_err(|e| AppError::Database(e.to_string()))?;
            let tracks_count: i64 = row.get(4).map_err(|e| AppError::Database(e.to_string()))?;
            let lyrics_count: i64 = row.get(5).map_err(|e| AppError::Database(e.to_string()))?;
            let notes_count: i64 = row.get(6).map_err(|e| AppError::Database(e.to_string()))?;
            let total_items: i64 = row.get(7).map_err(|e| AppError::Database(e.to_string()))?;

            let item_id: Option<String> = row.get(8).map_err(|e| AppError::Database(e.to_string()))?;
            let position: Option<i64> = row.get(9).map_err(|e| AppError::Database(e.to_string()))?;
            let kind: Option<String> = row.get(10).map_err(|e| AppError::Database(e.to_string()))?;
            let item_title: Option<String> = row.get(11).map_err(|e| AppError::Database(e.to_string()))?;
            let body: Option<String> = row.get(12).map_err(|e| AppError::Database(e.to_string()))?;

            let last_matches = summaries.last().map(|s| s.id == id).unwrap_or(false);

            if !last_matches {
                summaries.push(PresentationSummary {
                    id: id.clone(),
                    title,
                    created_at,
                    updated_at,
                    tracks_count,
                    lyrics_count,
                    notes_count,
                    total_items,
                    preview_items: Vec::new(),
                });
            }

            if let (Some(iid), Some(pos), Some(k)) = (item_id, position, kind) {
                let current = summaries.last_mut().expect("summary exists");
                let preview = match k.as_str() {
                    "track" => ShowItemPreview::Track {
                        id: iid,
                        position: pos,
                        title: item_title.unwrap_or_default(),
                    },
                    "lyrics" => ShowItemPreview::Lyrics {
                        id: iid,
                        position: pos,
                        snippet: ShowItemPreview::normalize_snippet(&body.unwrap_or_default(), 80),
                    },
                    "note" => ShowItemPreview::Note {
                        id: iid,
                        position: pos,
                        snippet: ShowItemPreview::normalize_snippet(&body.unwrap_or_default(), 80),
                    },
                    _ => continue,
                };
                current.preview_items.push(preview);
            }
        }

        Ok(summaries)
    }
}
