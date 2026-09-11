use serde::{Deserialize, Serialize};

use crate::domain::show_item::{ShowItem, ShowItemPayload};
use crate::domain::Presentation;

#[derive(Debug, Deserialize)]
pub struct CreatePresentationRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct RenamePresentationRequest {
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct ListPresentationsResponse {
    pub presentations: Vec<Presentation>,
}

#[derive(Debug, Serialize)]
pub struct PresentationResponse {
    pub presentation: Presentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShowItemResponse {
    Track {
        id: String,
        position: i64,
        title: String,
        audio_url: String,
        media_type: String,
        byte_size: i64,
    },
    Lyrics {
        id: String,
        position: i64,
        body: String,
    },
    Note {
        id: String,
        position: i64,
        body: String,
    },
}

impl From<&ShowItem> for ShowItemResponse {
    fn from(item: &ShowItem) -> Self {
        match &item.payload {
            ShowItemPayload::Track(t) => ShowItemResponse::Track {
                id: item.id.clone(),
                position: item.position,
                title: t.title.clone(),
                audio_url: format!("/api/tracks/{}/audio", item.id),
                media_type: t.media_type.clone(),
                byte_size: t.byte_size,
            },
            ShowItemPayload::Lyrics(l) => ShowItemResponse::Lyrics {
                id: item.id.clone(),
                position: item.position,
                body: l.body.clone(),
            },
            ShowItemPayload::Note(n) => ShowItemResponse::Note {
                id: item.id.clone(),
                position: item.position,
                body: n.body.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PresentationDetailResponse {
    pub presentation: Presentation,
    pub items: Vec<ShowItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct DeletePresentationResponse {
    pub deleted_id: String,
    pub cleanup_pending: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    #[serde(rename = "type")]
    pub item_type: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveItemRequest {
    pub direction: String,
}

#[derive(Debug, Serialize)]
pub struct ReorderItemsResponse {
    pub items: Vec<ShowItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct DeleteItemResponse {
    pub deleted_id: String,
    pub items: Vec<ShowItemResponse>,
    pub cleanup_pending: bool,
}
