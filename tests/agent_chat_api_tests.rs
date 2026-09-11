use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

use shirley_show::domain::agent_message::CompletionResponse;
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::infra::filesystem::LocalAudioStorage;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};
use shirley_show::router::create_router;
use shirley_show::state::AppState;
use shirley_show::use_cases::agent_chat::AgentLimits;

#[derive(Clone)]
struct MockProvider {
    response: CompletionResponse,
}

impl ChatCompletionProvider for MockProvider {
    fn complete(
        &self,
        _request: CompletionRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        CompletionResponse,
                        shirley_show::infra::groq::error::ProviderError,
                    >,
                > + Send,
        >,
    > {
        let resp = self.response.clone();
        Box::pin(async move { Ok(resp) })
    }
}

fn setup_test_app() -> (axum::Router, AppState, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let audio_dir = temp_dir.path().join("audio");
    std::fs::create_dir_all(audio_dir.join("files")).unwrap();
    std::fs::create_dir_all(audio_dir.join("staging")).unwrap();

    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(audio_dir));
    let mut state = AppState::new_with_items(pool, storage);
    state.frontend_dir = temp_dir.path().join("frontend");
    std::fs::create_dir_all(&state.frontend_dir).unwrap();

    let app = create_router(state.clone());
    (app, state, temp_dir)
}

fn setup_app_with_provider(response: CompletionResponse) -> (axum::Router, AppState, TempDir) {
    let (_app, state, tmp) = setup_test_app();
    let provider = Arc::new(MockProvider { response });
    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "llama-3.3-70b-versatile".to_string(),
    );
    let app = create_router(state.clone());
    (app, state, tmp)
}

fn build_multipart(
    boundary: &str,
    fields: &[(&str, &str)],
    files: &[(&str, &str, &str, &[u8])], // (field_name, filename, mime, bytes)
) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, val) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name).as_bytes(),
        );
        body.extend_from_slice(val.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    for (field_name, filename, mime, data) in files {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                field_name, filename, mime
            )
            .as_bytes(),
        );
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    body
}

#[tokio::test]
async fn test_chat_api_unconfigured_key_returns_200_with_notice() {
    let (app, _, _tmp) = setup_test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "message": "Hola" }).to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["actions_taken"], json!([]));
    assert_eq!(json["notice"]["code"], "agent_unconfigured");
    assert!(json["message"].as_str().unwrap().contains("GROQ_API_KEY"));
}

#[tokio::test]
async fn test_chat_api_valid_text_json_request() {
    let (app, _, _tmp) = setup_app_with_provider(CompletionResponse {
        content: Some("¡Hola! Soy Shirley.".to_string()),
        tool_calls: vec![],
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "message": "Hola Shirley" }).to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "¡Hola! Soy Shirley.");
    assert_eq!(json["actions_taken"], json!([]));
}

#[tokio::test]
async fn test_chat_api_empty_or_whitespace_message_rejected() {
    let (app, _, _tmp) = setup_test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "message": "   " }).to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn test_chat_api_oversized_message_rejected() {
    let (app, _, _tmp) = setup_test_app();

    let long_msg = "a".repeat(32_001);
    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "message": long_msg }).to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn test_chat_api_unsupported_content_type_rejected_415() {
    let (app, _, _tmp) = setup_test_app();

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("Hola"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "unsupported_media_type");
}

#[tokio::test]
async fn test_chat_api_multipart_audio_upload_and_staging_lifecycle() {
    let pres_name = "Show Multipart";
    let (_app, state, _tmp) = setup_test_app();
    let pres = state.create_presentation.execute(pres_name).unwrap();

    // Provider that calls add_track_from_staged
    let provider = Arc::new(MockProvider {
        response: CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_track_1".to_string(),
                name: "add_track_from_staged".to_string(),
                arguments: json!({
                    "presentation_id": pres.id,
                    "title": "Pista Batería"
                })
                .to_string(),
            }],
        },
    });

    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "llama-3.3-70b-versatile".to_string(),
    );
    let app = create_router(state.clone());

    let boundary = "----WebKitFormBoundaryXyZ123";
    let audio_bytes = [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00];
    let body_bytes = build_multipart(
        boundary,
        &[
            ("presentation_id", &pres.id),
            ("message", "Añade esta pista"),
        ],
        &[("file", "bateria.mp3", "audio/mpeg", &audio_bytes)],
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["actions_taken"].as_array().unwrap().len(), 1);
    assert_eq!(json["actions_taken"][0]["tool"], "add_track_from_staged");

    // Staging directory must be empty after promotion
    let staging_dir = state.storage.staging_dir();
    let count = std::fs::read_dir(staging_dir).unwrap().count();
    assert_eq!(
        count, 0,
        "Staging directory must be empty after consumption"
    );
}

#[tokio::test]
async fn test_chat_api_multipart_empty_text_defaults_with_audio() {
    let (_app, state, _tmp) = setup_test_app();
    let pres = state.create_presentation.execute("Show Auto").unwrap();

    let provider = Arc::new(MockProvider {
        response: CompletionResponse {
            content: Some("Pista añadida correctamente.".to_string()),
            tool_calls: vec![],
        },
    });

    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "llama-3.3-70b-versatile".to_string(),
    );
    let app = create_router(state.clone());

    let boundary = "----WebKitFormBoundaryEmptyMsg";
    let audio_bytes = [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00];
    let body_bytes = build_multipart(
        boundary,
        &[("presentation_id", &pres.id), ("message", "")],
        &[("file", "cancion.mp3", "audio/mpeg", &audio_bytes)],
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chat_api_multipart_unsupported_audio_format_rejected_400() {
    let (app, _, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundaryBadFmt";
    let body_bytes = build_multipart(
        boundary,
        &[("message", "pista")],
        &[("file", "audio.flac", "audio/flac", b"fLaC12345678")],
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "unsupported_format");
}

#[tokio::test]
async fn test_chat_api_multipart_duplicate_file_field_rejected_400() {
    let (app, _, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundaryDupFile";
    let audio_bytes = [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00];
    let body_bytes = build_multipart(
        boundary,
        &[("message", "pista")],
        &[
            ("file", "track1.mp3", "audio/mpeg", &audio_bytes),
            ("file", "track2.mp3", "audio/mpeg", &audio_bytes),
        ],
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "duplicate_audio_field");
}

#[tokio::test]
async fn test_chat_api_multipart_client_abort_purges_staging() {
    let (app, state, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundaryAbort";
    // Send truncated body without closing boundary and with interrupted file stream
    let mut truncated_body = Vec::new();
    truncated_body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    truncated_body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"cancion.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n",
    );
    // Write partial bytes
    truncated_body.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00]);
    // Abort stream abruptly without closing boundary

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(truncated_body))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    // Axum multipart parser returns 400 Bad Request when stream terminates unexpectedly
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Assert staging directory has 0 lingering .part files
    let staging_dir = state.storage.staging_dir();
    let remaining_staging = std::fs::read_dir(staging_dir).unwrap().count();
    assert_eq!(
        remaining_staging, 0,
        "Staging guard must have purged partial staging file upon stream error"
    );
}

#[tokio::test]
async fn test_chat_api_json_history_truncation_and_32k_character_cap() {
    let (app, _state, _tmp) = setup_app_with_provider(CompletionResponse {
        content: Some("Respuesta ok".to_string()),
        tool_calls: vec![],
    });

    let mut history = Vec::new();
    for i in 0..25 {
        history.push(json!({
            "role": if i % 2 == 0 { "user" } else { "assistant" },
            "content": format!("Mensaje número {} con texto de relleno {}", i, "x".repeat(1500)),
        }));
    }

    let payload = json!({
        "message": "¿Qué shows tengo?",
        "history": history,
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Respuesta ok");
}

#[tokio::test]
async fn test_chat_api_json_history_single_message_exceeding_32k_chars() {
    let (app, _state, _tmp) = setup_app_with_provider(CompletionResponse {
        content: Some("Respuesta ok".to_string()),
        tool_calls: vec![],
    });

    let huge_content = "x".repeat(35_000);
    let history = vec![json!({
        "role": "user",
        "content": huge_content,
    })];

    let payload = json!({
        "message": "Hola Shirley",
        "history": history,
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "Respuesta ok");
}

#[tokio::test]
async fn test_chat_api_multipart_attachment_exceeding_100_mib_rejected_400() {
    use tokio::io::AsyncReadExt;
    use tokio_util::io::ReaderStream;

    let (app, state, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundaryOversizedAudio";
    let header_part = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"message\"\r\n\r\nAgrega pista\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"huge.mp3\"\r\n\
         Content-Type: audio/mpeg\r\n\r\n"
    );

    let mut first_chunk = header_part.into_bytes();
    first_chunk.extend_from_slice(&[0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let prefix = std::io::Cursor::new(first_chunk);
    let zeroes = tokio::io::repeat(0).take(105 * 1024 * 1024);
    let chained = prefix.chain(zeroes);
    let stream = ReaderStream::new(chained);
    let body = Body::from_stream(stream);

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "file_too_large");

    let staging_dir = state.storage.staging_dir();
    let remaining_staging = std::fs::read_dir(staging_dir).unwrap().count();
    assert_eq!(
        remaining_staging, 0,
        "Staging files must be purged after 100 MiB rejection"
    );
}
