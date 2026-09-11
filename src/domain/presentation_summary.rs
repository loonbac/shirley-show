use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShowItemPreview {
    Track {
        id: String,
        position: i64,
        title: String,
    },
    Lyrics {
        id: String,
        position: i64,
        snippet: String,
    },
    Note {
        id: String,
        position: i64,
        snippet: String,
    },
}

impl ShowItemPreview {
    pub fn normalize_snippet(body: &str, max_chars: usize) -> String {
        let words: Vec<&str> = body.split_whitespace().collect();
        let joined = words.join(" ");
        if joined.chars().count() > max_chars {
            let truncated: String = joined.chars().take(max_chars).collect();
            truncated
        } else {
            joined
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub tracks_count: i64,
    pub lyrics_count: i64,
    pub notes_count: i64,
    pub total_items: i64,
    pub preview_items: Vec<ShowItemPreview>,
}
