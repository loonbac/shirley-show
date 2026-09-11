use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::Response;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::error::AppError;
use crate::handlers::audio::parse_range::{RangeSpec, parse_range};
use crate::state::AppState;

pub async fn stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let opened = state.open_track_audio.execute(&id)?;

    let range_header = headers.get(header::RANGE).and_then(|v| v.to_str().ok());

    let range_spec = parse_range(range_header, opened.byte_size);

    match range_spec {
        RangeSpec::Unsatisfiable => {
            let res = Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes */{}", opened.byte_size),
                )
                .body(Body::empty())
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(res)
        }
        RangeSpec::Full => {
            let builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_TYPE, opened.media_type)
                .header(header::CONTENT_LENGTH, opened.byte_size);

            if method == Method::HEAD {
                let res = builder
                    .body(Body::empty())
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(res)
            } else {
                let file = tokio::fs::File::open(&opened.path)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to open audio file: {e}")))?;
                let stream = ReaderStream::new(file);
                let body = Body::from_stream(stream);
                let res = builder
                    .body(body)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(res)
            }
        }
        RangeSpec::Satisfiable { start, end } => {
            let length = end - start + 1;
            let builder = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_TYPE, opened.media_type)
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{}", opened.byte_size),
                )
                .header(header::CONTENT_LENGTH, length);

            if method == Method::HEAD {
                let res = builder
                    .body(Body::empty())
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(res)
            } else {
                let mut file = tokio::fs::File::open(&opened.path)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to open audio file: {e}")))?;
                file.seek(SeekFrom::Start(start))
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to seek audio file: {e}")))?;
                let stream = ReaderStream::new(file.take(length));
                let body = Body::from_stream(stream);
                let res = builder
                    .body(body)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(res)
            }
        }
    }
}
