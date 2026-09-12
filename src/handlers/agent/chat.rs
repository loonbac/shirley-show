use axum::Json;
use axum::extract::{FromRequest, Multipart, Request, State};
use axum::http::{StatusCode, header};

use crate::domain::agent_tool::AgentNotice;
use crate::error::AppError;
use crate::handlers::agent::json_request::parse_and_validate_json_request;
use crate::handlers::agent::multipart_request::parse_and_validate_multipart_request;
use crate::state::AppState;
use crate::use_cases::agent_chat::AgentChatOutput;

pub async fn chat(
    State(state): State<AppState>,
    request: Request,
) -> Result<(StatusCode, Json<AgentChatOutput>), AppError> {
    let start_time = tokio::time::Instant::now();
    let parse_deadline = start_time + std::time::Duration::from_secs(30);

    let parse_result = tokio::time::timeout_at(parse_deadline, async {
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.starts_with("application/json") {
            let bytes = axum::body::to_bytes(request.into_body(), 33 * 1024 * 1024)
                .await
                .map_err(|e| {
                    AppError::bad_request(
                        "invalid_request",
                        format!("Error de lectura de body: {e}"),
                    )
                })?;
            parse_and_validate_json_request(&bytes)
        } else if content_type.starts_with("multipart/form-data") {
            let multipart = Multipart::from_request(request, &state)
                .await
                .map_err(|e| {
                    AppError::bad_request("invalid_request", format!("Error en multipart: {e}"))
                })?;
            parse_and_validate_multipart_request(multipart, state.storage.clone()).await
        } else {
            Err(AppError::unsupported_media_type(
                "unsupported_media_type",
                "Content-Type no soportado. Se requiere application/json o multipart/form-data.",
            ))
        }
    })
    .await;

    let parsed_request = match parse_result {
        Ok(Ok(p)) => p,
        Ok(Err(err)) => return Err(err),
        Err(_) => {
            return Ok((
                StatusCode::OK,
                Json(AgentChatOutput {
                    message: "La solicitud tardó más del límite de tiempo (30 s) y fue cancelada."
                        .to_string(),
                    actions_taken: vec![],
                    active_presentation_id: None,
                    active_event_id: None,
                    staged_audio_id: None,
                    notice: Some(AgentNotice {
                        code: "agent_timeout".to_string(),
                        retryable: true,
                    }),
                    conversation_id: uuid::Uuid::new_v4().to_string(),
                }),
            ));
        }
    };

    let mut input = parsed_request.input;
    // Si falta session_id en la solicitud, se genera un UUID efímero por request
    // (documentado como menos óptimo para la reutilización de caché en OpenCode Go / vLLM prefix cache).
    let session_id = parsed_request
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let conversation_id = input
        .conversation_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let pending_action = state.destructive_registry.take(&conversation_id);
    input.conversation_id = Some(conversation_id.clone());
    input.pending_destructive_action = pending_action;

    let mut output = crate::infra::groq::client::CURRENT_SESSION_ID
        .scope(
            session_id,
            state.agent_chat.execute_dynamic(input, start_time),
        )
        .await;
    output.conversation_id = conversation_id;
    Ok((StatusCode::OK, Json(output)))
}
