use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuStatusOutput {
    pub mode: String,
    pub pc_online: bool,
}

pub async fn gpu_status(State(state): State<AppState>) -> (StatusCode, Json<GpuStatusOutput>) {
    let is_remote = state.vocal_separator.is_remote();
    let mode = if is_remote { "remote" } else { "local" };
    let pc_online = if is_remote {
        state.vocal_separator.probe(Duration::from_secs(3)).await
    } else {
        true
    };

    (
        StatusCode::OK,
        Json(GpuStatusOutput {
            mode: mode.to_string(),
            pc_online,
        }),
    )
}
