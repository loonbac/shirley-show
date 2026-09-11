use axum::Json;
use axum::extract::{FromRequest, Multipart, Request, State};
use axum::http::{StatusCode, header};

use crate::error::AppError;
use crate::handlers::agent::json_request::parse_and_validate_json_request;
use crate::handlers::agent::multipart_request::parse_and_validate_multipart_request;
use crate::state::AppState;
use crate::use_cases::agent_chat::AgentChatOutput;

pub async fn chat(
    State(state): State<AppState>,
    request: Request,
) -> Result<(StatusCode, Json<AgentChatOutput>), AppError> {
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    let input = if content_type.starts_with("application/json") {
        let bytes = axum::body::to_bytes(request.into_body(), 33 * 1024 * 1024)
            .await
            .map_err(|e| AppError::bad_request("invalid_request", format!("Error de lectura de body: {e}")))?;
        parse_and_validate_json_request(&bytes)?
    } else if content_type.starts_with("multipart/form-data") {
        let multipart = Multipart::from_request(request, &state)
            .await
            .map_err(|e| AppError::bad_request("invalid_request", format!("Error en multipart: {e}")))?;
        parse_and_validate_multipart_request(multipart, state.storage.clone()).await?
    } else {
        return Err(AppError::unsupported_media_type(
            "unsupported_media_type",
            "Content-Type no soportado. Se requiere application/json o multipart/form-data.",
        ));
    };

    let output = state.agent_chat.execute(input).await;
    Ok((StatusCode::OK, Json(output)))
}
