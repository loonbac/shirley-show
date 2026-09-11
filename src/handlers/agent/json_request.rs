use serde::Deserialize;

use crate::domain::agent_message::HistoryMessage;
use crate::error::AppError;
use crate::use_cases::agent_chat::AgentChatInput;

pub const MAX_MESSAGE_CHAR_LENGTH: usize = 32_000;
pub const MAX_PRESENTATION_ID_LENGTH: usize = 128;
pub const MAX_STAGED_AUDIO_ID_LENGTH: usize = 128;

#[derive(Debug, Deserialize)]
pub struct AgentChatJsonPayload {
    pub message: Option<String>,
    pub presentation_id: Option<String>,
    pub staged_audio_id: Option<String>,
    #[serde(default)]
    pub history: Vec<HistoryMessage>,
}

pub fn parse_and_validate_json_request(body_bytes: &[u8]) -> Result<AgentChatInput, AppError> {
    let payload: AgentChatJsonPayload = serde_json::from_slice(body_bytes)
        .map_err(|e| AppError::bad_request("invalid_request", format!("JSON inválido: {e}")))?;

    let raw_message = payload.message.unwrap_or_default();
    let trimmed = raw_message.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "invalid_request",
            "El mensaje no puede estar vacío.",
        ));
    }

    if trimmed.chars().count() > MAX_MESSAGE_CHAR_LENGTH {
        return Err(AppError::bad_request(
            "invalid_request",
            "El mensaje no puede exceder 32.000 caracteres.",
        ));
    }

    let presentation_id = if let Some(id) = payload.presentation_id {
        let trimmed_id = id.trim().to_string();
        if trimmed_id.chars().count() > MAX_PRESENTATION_ID_LENGTH {
            return Err(AppError::bad_request(
                "invalid_request",
                "El identificador de presentación no puede exceder 128 caracteres.",
            ));
        }
        if trimmed_id.is_empty() {
            None
        } else {
            Some(trimmed_id)
        }
    } else {
        None
    };

    let staged_audio_id = if let Some(id) = payload.staged_audio_id {
        let trimmed_id = id.trim().to_string();
        if trimmed_id.chars().count() > MAX_STAGED_AUDIO_ID_LENGTH {
            return Err(AppError::bad_request(
                "invalid_request",
                "El identificador de audio en preparación no puede exceder 128 caracteres.",
            ));
        }
        if trimmed_id.is_empty() {
            None
        } else {
            Some(trimmed_id)
        }
    } else {
        None
    };

    Ok(AgentChatInput {
        message: trimmed.to_string(),
        history: payload.history,
        presentation_id,
        staged_audio_id,
        staged_audio: None,
        staged_filename: None,
    })
}
