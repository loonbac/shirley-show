use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::Response;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::error::AppError;
use crate::handlers::audio::parse_range::{RangeSpec, parse_range};
use crate::state::AppState;

#[derive(Debug, Default, Deserialize)]
pub struct AudioStreamQuery {
    pub download: Option<String>,
}

fn extension_from_media_type(media_type: &str) -> &'static str {
    match media_type {
        "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
        "audio/mp4" | "audio/x-m4a" | "audio/m4a" => "m4a",
        _ => "mp3",
    }
}

fn sanitize_title(title: &str) -> String {
    let filtered: String = title
        .chars()
        .filter(|c| !c.is_control() && *c != '"' && *c != '\\' && *c != '/')
        .collect();

    let collapsed = filtered.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = collapsed.chars().take(100).collect();
    let trimmed = truncated.trim();

    if trimmed.is_empty() {
        "pista".to_string()
    } else {
        trimmed.to_string()
    }
}

fn to_ascii_filename(title: &str) -> String {
    let mut ascii = String::with_capacity(title.len());
    for c in title.chars() {
        match c {
            'á' | 'à' | 'ä' | 'â' => ascii.push('a'),
            'é' | 'è' | 'ë' | 'ê' => ascii.push('e'),
            'í' | 'ì' | 'ï' | 'î' => ascii.push('i'),
            'ó' | 'ò' | 'ö' | 'ô' => ascii.push('o'),
            'ú' | 'ù' | 'ü' | 'û' => ascii.push('u'),
            'ñ' => ascii.push('n'),
            'Á' | 'À' | 'Ä' | 'Â' => ascii.push('A'),
            'É' | 'È' | 'Ë' | 'Ê' => ascii.push('E'),
            'Í' | 'Ì' | 'Ï' | 'Î' => ascii.push('I'),
            'Ó' | 'Ò' | 'Ö' | 'Ô' => ascii.push('O'),
            'Ú' | 'Ù' | 'Ü' | 'Û' => ascii.push('U'),
            'Ñ' => ascii.push('N'),
            c if c.is_ascii() && !c.is_control() && c != '"' && c != '\\' => ascii.push(c),
            _ => ascii.push('_'),
        }
    }
    let collapsed = ascii.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        "pista".to_string()
    } else {
        trimmed.to_string()
    }
}

fn rfc5987_percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 2);
    for b in input.as_bytes() {
        match b {
            b'0'..=b'9'
            | b'a'..=b'z'
            | b'A'..=b'Z'
            | b'!'
            | b'#'
            | b'$'
            | b'&'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~' => {
                encoded.push(*b as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(encoded, "%{:02X}", b);
            }
        }
    }
    encoded
}

fn build_content_disposition(title: &str, media_type: &str) -> String {
    let ext = extension_from_media_type(media_type);
    let clean_title = sanitize_title(title);
    let ascii_title = to_ascii_filename(&clean_title);
    let ascii_filename = format!("{ascii_title}.{ext}");

    if clean_title.is_ascii() {
        format!("attachment; filename=\"{ascii_filename}\"")
    } else {
        let utf8_filename = format!("{clean_title}.{ext}");
        let encoded = rfc5987_percent_encode(&utf8_filename);
        format!("attachment; filename=\"{ascii_filename}\"; filename*=UTF-8''{encoded}")
    }
}

pub async fn stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<AudioStreamQuery>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let opened = state.open_track_audio.execute(&id)?;

    let is_download = matches!(query.download.as_deref(), Some("1") | Some("true"));
    let disposition_header = if is_download {
        Some(build_content_disposition(&opened.title, &opened.media_type))
    } else {
        None
    };

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
            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_TYPE, opened.media_type)
                .header(header::CONTENT_LENGTH, opened.byte_size);

            if let Some(ref disp) = disposition_header {
                builder = builder.header(header::CONTENT_DISPOSITION, disp);
            }

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
            let mut builder = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_TYPE, opened.media_type)
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{}", opened.byte_size),
                )
                .header(header::CONTENT_LENGTH, length);

            if let Some(ref disp) = disposition_header {
                builder = builder.header(header::CONTENT_DISPOSITION, disp);
            }

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
