use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShowItemKind {
    Track,
    Lyrics,
    Note,
}

impl ShowItemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShowItemKind::Track => "track",
            ShowItemKind::Lyrics => "lyrics",
            ShowItemKind::Note => "note",
        }
    }
}

impl FromStr for ShowItemKind {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "track" => Ok(ShowItemKind::Track),
            "lyrics" => Ok(ShowItemKind::Lyrics),
            "note" => Ok(ShowItemKind::Note),
            _ => Err(AppError::validation(
                "invalid_item_kind",
                "Item kind must be 'track', 'lyrics', or 'note'",
                "kind",
            )),
        }
    }
}
