use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presentation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Presentation {
    pub fn new(raw_title: &str) -> Result<Self, AppError> {
        let trimmed = raw_title.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation(
                "title_required",
                "Escribe un nombre.",
                "title",
            ));
        }

        let now = Utc::now().to_rfc3339();
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            title: trimmed.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn rename(&mut self, raw_title: &str) -> Result<(), AppError> {
        let trimmed = raw_title.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation(
                "title_required",
                "Escribe un nombre.",
                "title",
            ));
        }

        self.title = trimmed.to_string();
        self.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn from_db(id: String, title: String, created_at: String, updated_at: String) -> Self {
        Self {
            id,
            title,
            created_at,
            updated_at,
        }
    }
}
