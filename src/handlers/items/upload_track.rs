use std::io::Write;
use std::path::Path;

use axum::extract::{Multipart, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::Json;

use crate::domain::show_item::MAX_AUDIO_BYTE_SIZE;
use crate::error::AppError;
use crate::handlers::dto::ShowItemResponse;
use crate::infra::filesystem::detect_audio_format::detect_audio_format;
use crate::infra::filesystem::StagingFileGuard;
use crate::state::AppState;

pub async fn upload_track(
    State(state): State<AppState>,
    AxumPath(presentation_id): AxumPath<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ShowItemResponse>), AppError> {
    let mut title: Option<String> = None;
    let mut staging_guard: Option<StagingFileGuard> = None;
    let mut detected_format = None;
    let mut total_bytes: i64 = 0;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request("invalid_multipart", e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "title" {
            let text = field
                .text()
                .await
                .map_err(|e| AppError::bad_request("invalid_title_field", e.to_string()))?;
            title = Some(text);
        } else if name == "audio" {
            if staging_guard.is_some() {
                return Err(AppError::bad_request(
                    "duplicate_audio_field",
                    "Solo se permite un archivo de audio por solicitud.",
                ));
            }

            let filename = field
                .file_name()
                .unwrap_or("audio.mp3")
                .to_string();

            let ext = Path::new(&filename)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let (stage_id, stage_path) = state.storage.create_staging_file()?;
            let guard = StagingFileGuard::new(state.storage.clone(), stage_id, stage_path.clone());
            let mut file = std::fs::File::create(guard.path()).map_err(|e| {
                AppError::Internal(format!("Failed to open staging file: {e}"))
            })?;

            let mut header_buf = Vec::new();

            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| AppError::bad_request("stream_error", e.to_string()))?
            {
                total_bytes += chunk.len() as i64;
                if total_bytes > MAX_AUDIO_BYTE_SIZE {
                    drop(file);
                    return Err(AppError::payload_too_large(
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

            let fmt = detect_audio_format(&header_buf, &ext)?;
            detected_format = Some(fmt);
            staging_guard = Some(guard);
        }
    }

    let title = match title {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            return Err(AppError::validation(
                "title_required",
                "El título de la pista no puede estar vacío.",
                "title",
            ));
        }
    };

    let guard = match staging_guard.take() {
        Some(g) => g,
        None => {
            return Err(AppError::bad_request(
                "missing_audio_field",
                "Se requiere el campo 'audio' con el archivo correspondiente.",
            ));
        }
    };

    let format = match detected_format {
        Some(f) => f,
        None => {
            return Err(AppError::unsupported_media_type(
                "unsupported_media_type",
                "Formato de audio no reconocido.",
            ));
        }
    };

    let stage_id = guard.stage_id().to_string();
    let item = state.upload_track.execute(
        &presentation_id,
        &title,
        &stage_id,
        format,
        total_bytes,
    )?;

    guard.defuse();

    Ok((StatusCode::CREATED, Json(ShowItemResponse::from(&item))))
}
