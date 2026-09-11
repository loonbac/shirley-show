use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::show_item_kind::ShowItemKind;
use crate::error::AppError;

pub const MAX_AUDIO_BYTE_SIZE: i64 = 100 * 1024 * 1024; // 104,857,600 bytes (100 MiB)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackPayload {
    pub title: String,
    pub audio_key: String,
    pub media_type: String,
    pub byte_size: i64,
}

impl TrackPayload {
    pub fn new(
        title: impl Into<String>,
        audio_key: impl Into<String>,
        media_type: impl Into<String>,
        byte_size: i64,
    ) -> Result<Self, AppError> {
        let trimmed_title = title.into().trim().to_string();
        if trimmed_title.is_empty() {
            return Err(AppError::validation(
                "title_required",
                "El título de la pista no puede estar vacío.",
                "title",
            ));
        }

        let trimmed_key = audio_key.into().trim().to_string();
        if trimmed_key.is_empty() {
            return Err(AppError::validation(
                "audio_key_required",
                "Se requiere una clave de audio válida.",
                "audio_key",
            ));
        }

        let trimmed_media = media_type.into().trim().to_string();
        if trimmed_media.is_empty() {
            return Err(AppError::validation(
                "media_type_required",
                "Se requiere un tipo MIME de audio.",
                "media_type",
            ));
        }

        if byte_size < 0 {
            return Err(AppError::validation(
                "invalid_byte_size",
                "El tamaño del archivo no puede ser negativo.",
                "byte_size",
            ));
        }

        if byte_size > MAX_AUDIO_BYTE_SIZE {
            return Err(AppError::validation(
                "file_too_large",
                "El archivo excede el límite máximo de 100 MiB.",
                "byte_size",
            ));
        }

        Ok(Self {
            title: trimmed_title,
            audio_key: trimmed_key,
            media_type: trimmed_media,
            byte_size,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricsPayload {
    pub body: String,
}

impl LyricsPayload {
    pub fn new(body: impl Into<String>) -> Result<Self, AppError> {
        let trimmed_body = body.into().trim().to_string();
        if trimmed_body.is_empty() {
            return Err(AppError::validation(
                "body_required",
                "El contenido de la letra no puede estar vacío.",
                "body",
            ));
        }

        Ok(Self { body: trimmed_body })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotePayload {
    pub body: String,
}

impl NotePayload {
    pub fn new(body: impl Into<String>) -> Result<Self, AppError> {
        let trimmed_body = body.into().trim().to_string();
        if trimmed_body.is_empty() {
            return Err(AppError::validation(
                "body_required",
                "El contenido de la nota no puede estar vacío.",
                "body",
            ));
        }

        Ok(Self { body: trimmed_body })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ShowItemPayload {
    Track(TrackPayload),
    Lyrics(LyricsPayload),
    Note(NotePayload),
}

impl ShowItemPayload {
    pub fn kind(&self) -> ShowItemKind {
        match self {
            ShowItemPayload::Track(_) => ShowItemKind::Track,
            ShowItemPayload::Lyrics(_) => ShowItemKind::Lyrics,
            ShowItemPayload::Note(_) => ShowItemKind::Note,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowItem {
    pub id: String,
    pub presentation_id: String,
    pub position: i64,
    pub payload: ShowItemPayload,
    pub created_at: String,
}

impl ShowItem {
    pub fn new(
        presentation_id: impl Into<String>,
        position: i64,
        payload: ShowItemPayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            presentation_id: presentation_id.into(),
            position,
            payload,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn from_db(
        id: String,
        presentation_id: String,
        position: i64,
        kind: String,
        title: Option<String>,
        body: Option<String>,
        audio_key: Option<String>,
        media_type: Option<String>,
        byte_size: Option<i64>,
        created_at: String,
    ) -> Result<Self, AppError> {
        let parsed_kind = kind.parse::<ShowItemKind>()?;
        let payload = match parsed_kind {
            ShowItemKind::Track => {
                let t = title.ok_or_else(|| {
                    AppError::Database("Track item missing title in database".into())
                })?;
                let k = audio_key.ok_or_else(|| {
                    AppError::Database("Track item missing audio_key in database".into())
                })?;
                let m = media_type.ok_or_else(|| {
                    AppError::Database("Track item missing media_type in database".into())
                })?;
                let s = byte_size.ok_or_else(|| {
                    AppError::Database("Track item missing byte_size in database".into())
                })?;
                ShowItemPayload::Track(TrackPayload {
                    title: t,
                    audio_key: k,
                    media_type: m,
                    byte_size: s,
                })
            }
            ShowItemKind::Lyrics => {
                let b = body.ok_or_else(|| {
                    AppError::Database("Lyrics item missing body in database".into())
                })?;
                ShowItemPayload::Lyrics(LyricsPayload { body: b })
            }
            ShowItemKind::Note => {
                let b = body.ok_or_else(|| {
                    AppError::Database("Note item missing body in database".into())
                })?;
                ShowItemPayload::Note(NotePayload { body: b })
            }
        };

        Ok(Self {
            id,
            presentation_id,
            position,
            payload,
            created_at,
        })
    }

    pub fn kind(&self) -> ShowItemKind {
        self.payload.kind()
    }
}
