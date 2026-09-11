use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use axum::extract::Multipart;

use crate::agent::turn_context::StagedAudioHandle;
use crate::domain::agent_message::HistoryMessage;
use crate::domain::show_item::MAX_AUDIO_BYTE_SIZE;
use crate::error::AppError;
use crate::handlers::agent::json_request::{MAX_MESSAGE_CHAR_LENGTH, MAX_PRESENTATION_ID_LENGTH};
use crate::infra::filesystem::detect_audio_format::detect_audio_format;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::infra::filesystem::staging_guard::StagingFileGuard;
use crate::use_cases::agent_chat::AgentChatInput;

pub async fn parse_and_validate_multipart_request(
    mut multipart: Multipart,
    storage: Arc<LocalAudioStorage>,
) -> Result<AgentChatInput, AppError> {
    let mut message: Option<String> = None;
    let mut presentation_id: Option<String> = None;
    let mut history: Vec<HistoryMessage> = Vec::new();
    let mut staging_guard: Option<StagingFileGuard> = None;
    let mut staged_format = None;
    let mut staged_filename: Option<String> = None;
    let mut total_bytes: i64 = 0;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request("invalid_request", format!("Error en multipart: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "message" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                message = Some(text);
            }
            "presentation_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                let trimmed = text.trim();
                if trimmed.chars().count() > MAX_PRESENTATION_ID_LENGTH {
                    return Err(AppError::bad_request(
                        "invalid_request",
                        "El identificador de presentación no puede exceder 128 caracteres.",
                    ));
                }
                if !trimmed.is_empty() {
                    presentation_id = Some(trimmed.to_string());
                }
            }
            "history" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    history = serde_json::from_str(trimmed).map_err(|e| {
                        AppError::bad_request(
                            "invalid_request",
                            format!("Historial de conversación JSON inválido: {e}"),
                        )
                    })?;
                }
            }
            "file" => {
                if staging_guard.is_some() {
                    return Err(AppError::bad_request(
                        "duplicate_audio_field",
                        "Solo se permite un archivo de audio por solicitud.",
                    ));
                }

                let filename = field.file_name().unwrap_or("audio.mp3").to_string();
                let ext = Path::new(&filename)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let (stage_id, stage_path) = storage.create_staging_file()?;
                let guard = StagingFileGuard::new(storage.clone(), stage_id, stage_path.clone());
                let mut file = std::fs::File::create(guard.path())
                    .map_err(|e| AppError::Internal(format!("Failed to open staging file: {e}")))?;

                let mut header_buf = Vec::new();

                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?
                {
                    total_bytes += chunk.len() as i64;
                    if total_bytes > MAX_AUDIO_BYTE_SIZE {
                        drop(file);
                        return Err(AppError::bad_request(
                            "file_too_large",
                            "El archivo excede el límite máximo de 100 MiB.",
                        ));
                    }

                    if header_buf.len() < 32 {
                        let needed = 32 - header_buf.len();
                        let take_len = chunk.len().min(needed);
                        header_buf.extend_from_slice(&chunk[..take_len]);
                    }

                    if let Err(e) = file.write_all(&chunk) {
                        drop(file);
                        return Err(AppError::Internal(format!(
                            "Failed to write to staging file: {e}"
                        )));
                    }
                }

                drop(file);

                let fmt = detect_audio_format(&header_buf, &ext).map_err(|_| {
                    AppError::bad_request(
                        "unsupported_format",
                        "Solo se admiten archivos de audio .mp3, .wav y .m4a.",
                    )
                })?;

                staged_format = Some(fmt);
                staging_guard = Some(guard);
                staged_filename = Some(filename);
            }
            _ => {}
        }
    }

    let final_message = match message {
        Some(m) if !m.trim().is_empty() => {
            let trimmed = m.trim();
            if trimmed.chars().count() > MAX_MESSAGE_CHAR_LENGTH {
                return Err(AppError::bad_request(
                    "invalid_request",
                    "El mensaje no puede exceder 32.000 caracteres.",
                ));
            }
            trimmed.to_string()
        }
        _ => {
            if staging_guard.is_some() {
                "Añadir esta pista de audio".to_string()
            } else {
                return Err(AppError::bad_request(
                    "invalid_request",
                    "El mensaje no puede estar vacío.",
                ));
            }
        }
    };

    let staged_audio = staging_guard.map(|guard| StagedAudioHandle::Available {
        guard,
        format: staged_format.expect("format must be detected"),
        byte_size: total_bytes,
    });

    Ok(AgentChatInput {
        message: final_message,
        history,
        presentation_id,
        staged_audio,
        staged_filename,
    })
}
