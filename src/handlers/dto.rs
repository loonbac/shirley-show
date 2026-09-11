use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize)]
pub struct PresentationDetailResponse {
    pub presentation: Presentation,
    pub items: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DeletePresentationResponse {
    pub deleted_id: String,
    pub cleanup_pending: bool,
}
