use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use axum::extract::Multipart;

use crate::agent::turn_context::{MAX_POSTER_SIZE_BYTES, StagedAudioHandle, StagedImageHandle};
use crate::domain::agent_message::HistoryMessage;
use crate::domain::show_item::MAX_AUDIO_BYTE_SIZE;
use crate::error::AppError;
use crate::handlers::agent::json_request::{
    MAX_CONVERSATION_ID_LENGTH, MAX_EVENT_ID_LENGTH, MAX_MESSAGE_CHAR_LENGTH,
    MAX_PRESENTATION_ID_LENGTH, MAX_SESSION_ID_LENGTH, MAX_STAGED_AUDIO_ID_LENGTH,
    ParsedAgentChatRequest,
};
use crate::infra::filesystem::detect_audio_format::{
    detect_audio_format, detect_audio_format_from_bytes,
};
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::infra::filesystem::staging_guard::StagingFileGuard;
use crate::ports::ticketing::ImageFormat;
use crate::use_cases::agent_chat::AgentChatInput;

pub async fn parse_and_validate_multipart_request(
    mut multipart: Multipart,
    storage: Arc<LocalAudioStorage>,
) -> Result<ParsedAgentChatRequest, AppError> {
    let mut message: Option<String> = None;
    let mut presentation_id: Option<String> = None;
    let mut event_id: Option<String> = None;
    let mut staged_audio_id: Option<String> = None;
    let mut conversation_id: Option<String> = None;
    let mut session_id: Option<String> = None;
    let mut history: Vec<HistoryMessage> = Vec::new();
    let mut staging_guard: Option<StagingFileGuard> = None;
    let mut staged_format = None;
    let mut staged_image: Option<StagedImageHandle> = None;
    let mut staged_filename: Option<String> = None;
    let mut total_bytes: i64 = 0;
    let mut has_file_field = false;

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
            "event_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                let trimmed = text.trim();
                if trimmed.chars().count() > MAX_EVENT_ID_LENGTH {
                    return Err(AppError::bad_request(
                        "invalid_request",
                        "El identificador de evento no puede exceder 128 caracteres.",
                    ));
                }
                if !trimmed.is_empty() {
                    event_id = Some(trimmed.to_string());
                }
            }
            "staged_audio_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                let trimmed = text.trim();
                if trimmed.chars().count() > MAX_STAGED_AUDIO_ID_LENGTH {
                    return Err(AppError::bad_request(
                        "invalid_request",
                        "El identificador de audio en preparación no puede exceder 128 caracteres.",
                    ));
                }
                if !trimmed.is_empty() {
                    staged_audio_id = Some(trimmed.to_string());
                }
            }
            "conversation_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                let trimmed = text.trim();
                if trimmed.chars().count() > MAX_CONVERSATION_ID_LENGTH {
                    return Err(AppError::bad_request(
                        "invalid_request",
                        "El identificador de conversación no puede exceder 128 caracteres.",
                    ));
                }
                if !trimmed.is_empty() {
                    conversation_id = Some(trimmed.to_string());
                }
            }
            "session_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?;
                let trimmed = text.trim();
                if trimmed.chars().count() > MAX_SESSION_ID_LENGTH {
                    return Err(AppError::bad_request(
                        "invalid_request",
                        "El identificador de sesión no puede exceder 128 caracteres.",
                    ));
                }
                if !trimmed.is_empty() {
                    session_id = Some(trimmed.to_string());
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
                if has_file_field {
                    return Err(AppError::bad_request(
                        "duplicate_audio_field",
                        "Solo se permite un archivo por solicitud.",
                    ));
                }
                has_file_field = true;

                let filename = field.file_name().unwrap_or("file").to_string();
                let ext = Path::new(&filename)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let mime = field.content_type().unwrap_or("").to_lowercase();

                let is_declared_image = matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp")
                    || mime.starts_with("image/");
                let is_declared_audio =
                    matches!(ext.as_str(), "mp3" | "wav" | "m4a") || mime.starts_with("audio/");

                // 1. Read initial chunks to collect up to 32 header bytes
                let mut initial_chunks: Vec<axum::body::Bytes> = Vec::new();
                let mut header_buf: Vec<u8> = Vec::new();

                while header_buf.len() < 32 {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            let needed = 32 - header_buf.len();
                            let take_len = chunk.len().min(needed);
                            header_buf.extend_from_slice(&chunk[..take_len]);
                            initial_chunks.push(chunk);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            return Err(AppError::bad_request("invalid_request", e.to_string()));
                        }
                    }
                }

                if header_buf.is_empty() {
                    return Err(AppError::bad_request(
                        "empty_file",
                        "El archivo adjunto está vacío.",
                    ));
                }

                // 2. Classify stream by magic bytes and declared intent
                enum TargetKind {
                    Image(ImageFormat),
                    Audio(crate::domain::audio_format::AudioFormat),
                }

                let target_kind = if is_declared_image {
                    match ImageFormat::detect(&header_buf) {
                        Ok(fmt) => TargetKind::Image(fmt),
                        Err(_) => {
                            return Err(AppError::bad_request(
                                "unsupported_format",
                                "Solo se admiten imágenes .jpg, .png y .webp con formato válido.",
                            ));
                        }
                    }
                } else if is_declared_audio {
                    match detect_audio_format(&header_buf, &ext) {
                        Ok(fmt) => TargetKind::Audio(fmt),
                        Err(_) => {
                            return Err(AppError::bad_request(
                                "unsupported_format",
                                "Solo se admiten archivos de audio .mp3, .wav y .m4a.",
                            ));
                        }
                    }
                } else {
                    // Generic metadata: try image first, then audio
                    if let Ok(fmt) = ImageFormat::detect(&header_buf) {
                        TargetKind::Image(fmt)
                    } else if let Some(fmt) = detect_audio_format_from_bytes(&header_buf) {
                        TargetKind::Audio(fmt)
                    } else {
                        return Err(AppError::bad_request(
                            "unsupported_format",
                            "Formato de archivo no soportado. Se admiten archivos de audio (.mp3, .wav, .m4a) o afiches de imagen (.jpg, .png, .webp).",
                        ));
                    }
                };

                match target_kind {
                    TargetKind::Image(image_fmt) => {
                        let mut image_data = Vec::new();
                        let mut total_image_bytes = 0usize;

                        for chunk in initial_chunks {
                            total_image_bytes += chunk.len();
                            if total_image_bytes > MAX_POSTER_SIZE_BYTES {
                                return Err(AppError::bad_request(
                                    "file_too_large",
                                    "El archivo excede el límite máximo de 10 MiB.",
                                ));
                            }
                            image_data.extend_from_slice(&chunk);
                        }

                        while let Some(chunk) = field
                            .chunk()
                            .await
                            .map_err(|e| AppError::bad_request("invalid_request", e.to_string()))?
                        {
                            total_image_bytes += chunk.len();
                            if total_image_bytes > MAX_POSTER_SIZE_BYTES {
                                return Err(AppError::bad_request(
                                    "file_too_large",
                                    "El archivo excede el límite máximo de 10 MiB.",
                                ));
                            }
                            image_data.extend_from_slice(&chunk);
                        }

                        staged_image = Some(StagedImageHandle::Available {
                            data: image_data,
                            format: image_fmt,
                            byte_size: total_image_bytes,
                        });
                        staged_filename = Some(filename);
                    }

                    TargetKind::Audio(audio_fmt) => {
                        let (stage_id, stage_path) = storage.create_staging_file()?;
                        let guard =
                            StagingFileGuard::new(storage.clone(), stage_id, stage_path.clone());
                        let mut file = std::fs::File::create(guard.path()).map_err(|e| {
                            AppError::Internal(format!("Failed to open staging file: {e}"))
                        })?;

                        for chunk in initial_chunks {
                            total_bytes += chunk.len() as i64;
                            if total_bytes > MAX_AUDIO_BYTE_SIZE {
                                drop(file);
                                return Err(AppError::bad_request(
                                    "file_too_large",
                                    "El archivo excede el límite máximo de 100 MiB.",
                                ));
                            }

                            if let Err(e) = file.write_all(&chunk) {
                                drop(file);
                                return Err(AppError::Internal(format!(
                                    "Failed to write to staging file: {e}"
                                )));
                            }
                        }

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

                            if let Err(e) = file.write_all(&chunk) {
                                drop(file);
                                return Err(AppError::Internal(format!(
                                    "Failed to write to staging file: {e}"
                                )));
                            }
                        }

                        drop(file);

                        staged_format = Some(audio_fmt);
                        staging_guard = Some(guard);
                        staged_filename = Some(filename);
                    }
                }
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
            if staged_image.is_some() {
                "Usar esta imagen como afiche del evento activo".to_string()
            } else if staging_guard.is_some() {
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

    Ok(ParsedAgentChatRequest {
        input: AgentChatInput {
            message: final_message,
            history,
            presentation_id,
            event_id,
            staged_audio_id,
            staged_audio,
            staged_image,
            staged_filename,
            conversation_id,
            pending_destructive_action: None,
        },
        session_id,
    })
}
