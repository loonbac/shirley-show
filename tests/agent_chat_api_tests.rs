use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

use shirley_show::config::Config;

use shirley_show::domain::agent_message::CompletionResponse;
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::infra::filesystem::LocalAudioStorage;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};
use shirley_show::ports::show_item_reader::ShowItemReader;
use shirley_show::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};
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

    let config = Config {
        bind_addr: "127.0.0.1:3000".parse().unwrap(),
        data_dir: temp_dir.path().join("data"),
        frontend_dir: temp_dir.path().join("frontend"),
        groq: Default::default(),
        yt_dlp_bin: None,
        agent_timeout_secs: 120,
        separation_timeout_secs: 600,
        remote_separation_timeout_secs: 1500,
        joshua_pc: None,
    };
    std::fs::create_dir_all(&config.data_dir).unwrap();
    std::fs::create_dir_all(&config.frontend_dir).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(audio_dir));
    let staged_audio_registry =
        Arc::new(shirley_show::state::staged_audio_registry::StagedAudioRegistry::new());
    let youtube_downloader = Arc::new(shirley_show::infra::youtube::ProcessYoutubeDownloader::new(
        shirley_show::infra::youtube::ProcessYoutubeDownloaderConfig::default(),
    ));

    let state = AppState::new_with_all_components_and_config(
        pool,
        storage,
        staged_audio_registry,
        youtube_downloader,
        &config,
    );

    let app = create_router(state.clone());
    (app, state, temp_dir)
}

fn setup_app_with_provider(response: CompletionResponse) -> (axum::Router, AppState, TempDir) {
    let (_app, state, tmp) = setup_test_app();
    let provider = Arc::new(MockProvider { response });
    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
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
        content: Some("¡Hola! Soy Ressy.".to_string()),
        tool_calls: vec![],
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "message": "Hola Ressy" }).to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["message"], "¡Hola! Soy Ressy.");
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
        "openai/gpt-oss-120b".to_string(),
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
        "openai/gpt-oss-120b".to_string(),
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
        "message": "Hola Ressy",
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

static CONFIG_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_config_reads_yt_dlp_bin_and_agent_timeout_secs() {
    let _guard = CONFIG_ENV_MUTEX.lock().unwrap();

    unsafe {
        // 1. Defaults when unset
        std::env::remove_var("YT_DLP_BIN");
        std::env::remove_var("SHIRLEY_AGENT_TIMEOUT_SECS");
        let cfg = Config::from_env().expect("config should load with defaults");
        assert_eq!(cfg.yt_dlp_bin, None);
        assert_eq!(cfg.agent_timeout_secs, 120);

        // 2. Custom values with trimming
        std::env::set_var("YT_DLP_BIN", "  /custom/bin/yt-dlp  ");
        std::env::set_var("SHIRLEY_AGENT_TIMEOUT_SECS", " 90 ");
        let cfg2 = Config::from_env().expect("config should load with custom env");
        assert_eq!(cfg2.yt_dlp_bin, Some(PathBuf::from("/custom/bin/yt-dlp")));
        assert_eq!(cfg2.agent_timeout_secs, 90);

        // 3. Empty string for YT_DLP_BIN falls back to None (PATH search)
        std::env::set_var("YT_DLP_BIN", "   ");
        let cfg3 = Config::from_env().expect("empty yt_dlp_bin should parse as None");
        assert_eq!(cfg3.yt_dlp_bin, None);

        // 4. Invalid SHIRLEY_AGENT_TIMEOUT_SECS values rejected
        std::env::set_var("SHIRLEY_AGENT_TIMEOUT_SECS", "0");
        assert!(
            Config::from_env().is_err(),
            "0 seconds timeout must be rejected"
        );

        std::env::set_var("SHIRLEY_AGENT_TIMEOUT_SECS", "-5");
        assert!(
            Config::from_env().is_err(),
            "negative timeout must be rejected"
        );

        std::env::set_var("SHIRLEY_AGENT_TIMEOUT_SECS", "invalid_number");
        assert!(
            Config::from_env().is_err(),
            "non-numeric timeout must be rejected"
        );

        std::env::remove_var("SHIRLEY_AGENT_TIMEOUT_SECS");

        // 5. SHIRLEY_SEPARATION_TIMEOUT_SECS boundary tests (clamp at 600 max)
        std::env::set_var("SHIRLEY_SEPARATION_TIMEOUT_SECS", " 450 ");
        let cfg_sep = Config::from_env().expect("separation timeout 450 valid");
        assert_eq!(cfg_sep.separation_timeout_secs, 450);

        std::env::set_var("SHIRLEY_SEPARATION_TIMEOUT_SECS", "600");
        let cfg_sep600 = Config::from_env().expect("separation timeout 600 valid");
        assert_eq!(cfg_sep600.separation_timeout_secs, 600);

        std::env::set_var("SHIRLEY_SEPARATION_TIMEOUT_SECS", "601");
        let cfg_sep601 = Config::from_env().expect("separation timeout > 600 valid");
        assert_eq!(cfg_sep601.separation_timeout_secs, 601);

        std::env::set_var("SHIRLEY_SEPARATION_TIMEOUT_SECS", "0");
        assert!(
            Config::from_env().is_err(),
            "separation timeout 0 must be rejected"
        );

        // Clean up
        std::env::remove_var("YT_DLP_BIN");
        std::env::remove_var("SHIRLEY_AGENT_TIMEOUT_SECS");
        std::env::remove_var("SHIRLEY_SEPARATION_TIMEOUT_SECS");
    }
}

#[tokio::test]
async fn test_chat_api_json_accepts_and_roundtrips_staged_audio_id() {
    let (app, _state, _tmp) = setup_app_with_provider(CompletionResponse {
        content: Some("¿A qué presentación deseas agregarlo?".to_string()),
        tool_calls: vec![],
    });

    // Valid staged_audio_id forwarded and preserved in output
    let payload = json!({
        "message": "Audio listo sin destino",
        "staged_audio_id": "staged-yt-12345",
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["staged_audio_id"], "staged-yt-12345");

    // staged_audio_id exceeding 128 characters rejected 400
    let oversized_staged_id = "s".repeat(129);
    let invalid_payload = json!({
        "message": "Hola",
        "staged_audio_id": oversized_staged_id,
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&invalid_payload).unwrap()))
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);
    let body2 = res2.into_body().collect().await.unwrap().to_bytes();
    let json2: Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(json2["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn test_chat_api_multipart_accepts_and_roundtrips_staged_audio_id() {
    let (app, _state, _tmp) = setup_app_with_provider(CompletionResponse {
        content: Some("Audio staged recibido en multipart".to_string()),
        tool_calls: vec![],
    });

    let boundary = "----WebKitFormBoundaryStagedAudio";
    let body_bytes = build_multipart(
        boundary,
        &[
            ("message", "¿En qué show lo pongo?"),
            ("staged_audio_id", "staged-multipart-uuid-777"),
        ],
        &[],
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["staged_audio_id"], "staged-multipart-uuid-777");

    // Oversized staged_audio_id in multipart rejected 400
    let oversized_staged_id = "m".repeat(129);
    let invalid_bytes = build_multipart(
        boundary,
        &[
            ("message", "Hola"),
            ("staged_audio_id", &oversized_staged_id),
        ],
        &[],
    );

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(invalid_bytes))
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);
}

struct DelayedChainedProvider {
    call_count: std::sync::atomic::AtomicUsize,
}

impl ChatCompletionProvider for DelayedChainedProvider {
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
        let count = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            if count == 0 {
                // Turn 1: create presentation
                Ok(CompletionResponse {
                    content: None,
                    tool_calls: vec![ToolCall {
                        id: "call_create_1".to_string(),
                        name: "create_presentation".to_string(),
                        arguments: json!({ "name": "Show Deadline Test" }).to_string(),
                    }],
                })
            } else {
                // Turn 2: sleep longer than deadline
                tokio::time::sleep(Duration::from_millis(600)).await;
                Ok(CompletionResponse {
                    content: Some("Respuesta tardía que no debe llegar".to_string()),
                    tool_calls: vec![],
                })
            }
        })
    }
}

#[tokio::test]
async fn test_chat_api_unified_deadline_preserves_actions_and_returns_200_on_timeout() {
    let (_app, state, _tmp) = setup_test_app();
    let provider = Arc::new(DelayedChainedProvider {
        call_count: std::sync::atomic::AtomicUsize::new(0),
    });

    // Configure a 150ms timeout: enough time for iteration 0 tool execution,
    // but timing out during iteration 1 (600ms sleep)
    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_millis(150),
        separation_timeout: Duration::from_millis(150),
    };
    let state =
        state.with_agent_provider(Some(provider), limits, "openai/gpt-oss-120b".to_string());
    let app = create_router(state);

    let payload = json!({
        "message": "Crea el show Show Deadline Test y haz algo lento",
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "Timeout must return HTTP 200");

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // 1. Notice with code agent_timeout and retryable
    assert_eq!(json["notice"]["code"], "agent_timeout");
    assert_eq!(json["notice"]["retryable"], true);

    // 2. Actions committed before timeout MUST be preserved!
    let actions = json["actions_taken"]
        .as_array()
        .expect("actions_taken array");
    assert_eq!(
        actions.len(),
        1,
        "create_presentation must be preserved in actions_taken"
    );
    assert_eq!(actions[0]["tool"], "create_presentation");

    // 3. Message reports committed actions and timeout notice without hardcoded "(30 s)"
    let message = json["message"].as_str().expect("message string");
    assert!(
        message.contains("Show Deadline Test")
            || message.contains("creada")
            || message.contains("creé"),
        "Message must reflect committed action"
    );
    assert!(
        !message.contains("(30 s)"),
        "Message must not hardcode (30 s)"
    );
}

#[derive(Clone)]
struct FakeVocalSeparator {
    fail_with_error: Arc<std::sync::Mutex<Option<VocalSeparationError>>>,
    execution_count: Arc<std::sync::atomic::AtomicUsize>,
    delay: Option<Duration>,
}

impl FakeVocalSeparator {
    fn new() -> Self {
        Self {
            fail_with_error: Arc::new(std::sync::Mutex::new(None)),
            execution_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            delay: None,
        }
    }

    fn failing(err: VocalSeparationError) -> Self {
        Self {
            fail_with_error: Arc::new(std::sync::Mutex::new(Some(err))),
            execution_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            delay: None,
        }
    }

    fn with_delay(delay: Duration) -> Self {
        Self {
            fail_with_error: Arc::new(std::sync::Mutex::new(None)),
            execution_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            delay: Some(delay),
        }
    }
}

impl VocalSeparator for FakeVocalSeparator {
    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>,
    > {
        self.execution_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let err_opt = self.fail_with_error.lock().unwrap().clone();
        let delay = self.delay;
        let out_path = request.output_path;
        Box::pin(async move {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            if let Some(err) = err_opt {
                return Err(err);
            }
            std::fs::write(&out_path, &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00])
                .expect("write fake mp3 part");
            Ok(SeparatedAudio {
                duration_secs: 180,
                size_bytes: 7200000,
            })
        })
    }
}

#[derive(Clone)]
struct MultiStepMockProvider {
    responses: Arc<std::sync::Mutex<std::collections::VecDeque<CompletionResponse>>>,
}

impl MultiStepMockProvider {
    fn new(responses: Vec<CompletionResponse>) -> Self {
        Self {
            responses: Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from(
                responses,
            ))),
        }
    }
}

impl ChatCompletionProvider for MultiStepMockProvider {
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
        let next = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| CompletionResponse {
                content: Some("Final default".to_string()),
                tool_calls: vec![],
            });
        Box::pin(async move { Ok(next) })
    }
}

#[tokio::test]
async fn test_chat_api_separation_single_step_persists_to_active_presentation() {
    let (_app, state, _tmp) = setup_test_app();
    let fake_sep = Arc::new(FakeVocalSeparator::new());
    let state = state.with_vocal_separator(fake_sep.clone());
    let pres = state
        .create_presentation
        .execute("Show Activo API")
        .unwrap();

    let staged_src = state.storage.staging_dir().join("api_src.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-api-src".to_string(),
        staged_src,
        "Tema API".to_string(),
        180,
        5000000,
    );

    let provider = Arc::new(MultiStepMockProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_api".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_add_api".to_string(),
                name: "add_track_from_staged".to_string(),
                arguments: json!({
                    "presentation_id": pres.id,
                    "title": "Tema API (Coros)"
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: Some("Pista separada y añadida al show.".to_string()),
            tool_calls: vec![],
        },
    ]));

    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
    );
    let app = create_router(state.clone());

    let payload = json!({
        "message": "Separa coros y agrégalo al show",
        "presentation_id": pres.id,
        "staged_audio_id": reg_entry.staged_audio_id
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

    let actions = json["actions_taken"].as_array().unwrap();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0]["tool"], "separate_vocals");
    assert_eq!(actions[1]["tool"], "add_track_from_staged");
    assert!(json["staged_audio_id"].is_null());

    let items = state.item_repo.list_by_presentation(&pres.id).unwrap();
    assert_eq!(items.len(), 1);
    match &items[0].payload {
        shirley_show::domain::show_item::ShowItemPayload::Track(t) => {
            assert_eq!(t.title, "Tema API (Coros)");
        }
        _ => panic!("expected track"),
    }
    assert_eq!(
        fake_sep
            .execution_count
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn test_chat_api_separation_polarformer_absent_returns_friendly_error_no_phantom_track() {
    let (_app, state, _tmp) = setup_test_app();
    let fake_sep = Arc::new(FakeVocalSeparator::failing(
        VocalSeparationError::InstrumentalUnavailable,
    ));
    let state = state.with_vocal_separator(fake_sep);
    let pres = state.create_presentation.execute("Show Polar").unwrap();

    let staged_src = state.storage.staging_dir().join("polar_src.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-polar-src".to_string(),
        staged_src,
        "Tema Polar".to_string(),
        180,
        5000000,
    );

    let provider = Arc::new(MultiStepMockProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_polar".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "instrumental",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: Some(
                "El modo instrumental no está configurado en este servidor. Puedes optar por la versión con coros o conservar el audio en staging.".to_string(),
            ),
            tool_calls: vec![],
        },
    ]));

    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
    );
    let app = create_router(state.clone());

    let payload = json!({
        "message": "Separa instrumental",
        "presentation_id": pres.id,
        "staged_audio_id": reg_entry.staged_audio_id
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

    let actions = json["actions_taken"].as_array().unwrap();
    assert_eq!(actions.len(), 0, "No actions should be recorded on failure");

    let items = state.item_repo.list_by_presentation(&pres.id).unwrap();
    assert_eq!(items.len(), 0, "Zero phantom tracks persisted");
}

#[tokio::test]
async fn test_chat_api_separation_duplicate_call_ignored_single_gpu_execution() {
    let (_app, state, _tmp) = setup_test_app();
    let fake_sep = Arc::new(FakeVocalSeparator::new());
    let state = state.with_vocal_separator(fake_sep.clone());
    let pres = state.create_presentation.execute("Show Dedup API").unwrap();

    let staged_src = state.storage.staging_dir().join("dedup_api.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-dedup-api".to_string(),
        staged_src,
        "Tema Dedup API".to_string(),
        180,
        5000000,
    );

    let provider = Arc::new(MultiStepMockProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![
                ToolCall {
                    id: "call_d1".to_string(),
                    name: "separate_vocals".to_string(),
                    arguments: json!({
                        "mode": "chorus",
                        "staged_audio_id": reg_entry.staged_audio_id
                    })
                    .to_string(),
                },
                ToolCall {
                    id: "call_d2".to_string(),
                    name: "separate_vocals".to_string(),
                    arguments: json!({
                        "mode": "chorus",
                        "staged_audio_id": reg_entry.staged_audio_id
                    })
                    .to_string(),
                },
            ],
        },
        CompletionResponse {
            content: Some("Listo.".to_string()),
            tool_calls: vec![],
        },
    ]));

    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
    );
    let app = create_router(state);

    let payload = json!({
        "message": "Separa coros dos veces",
        "presentation_id": pres.id,
        "staged_audio_id": reg_entry.staged_audio_id
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        fake_sep
            .execution_count
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn test_chat_api_separation_timeout_preserves_prior_actions_and_releases_gpu() {
    let (_app, state, _tmp) = setup_test_app();
    let fake_sep = Arc::new(FakeVocalSeparator::with_delay(Duration::from_millis(500)));
    let state = state.with_vocal_separator(fake_sep);
    let pres = state
        .create_presentation
        .execute("Show Timeout API")
        .unwrap();

    let staged_src = state.storage.staging_dir().join("timeout_api.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-timeout-api".to_string(),
        staged_src,
        "Tema Timeout".to_string(),
        180,
        5000000,
    );

    let provider = Arc::new(MultiStepMockProvider::new(vec![
        // Step 1: Add a note (commits action)
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_note".to_string(),
                name: "add_note".to_string(),
                arguments: json!({
                    "presentation_id": pres.id,
                    "text": "Nota antes de separar"
                })
                .to_string(),
            }],
        },
        // Step 2: separate_vocals takes 500ms
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_timeout".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
    ]));

    // Limit timeout to 150ms: note succeeds, separation times out
    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_millis(150),
        separation_timeout: Duration::from_millis(150),
    };

    let state =
        state.with_agent_provider(Some(provider), limits, "openai/gpt-oss-120b".to_string());
    let app = create_router(state.clone());

    let payload = json!({
        "message": "Agrega nota y separa coros",
        "presentation_id": pres.id,
        "staged_audio_id": reg_entry.staged_audio_id
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

    assert_eq!(json["notice"]["code"], "agent_timeout");
    let actions = json["actions_taken"].as_array().unwrap();
    assert_eq!(
        actions.len(),
        1,
        "Prior committed note action must be preserved"
    );
    assert_eq!(actions[0]["tool"], "add_note");
    // GPU permit must be available (released)
    assert_eq!(state.gpu_semaphore.available_permits(), 1);
}

#[tokio::test]
async fn test_chat_api_multipart_image_magic_byte_classification_jpeg_png_webp() {
    let (_app, state, _tmp) = setup_test_app();
    let provider = Arc::new(MockProvider {
        response: CompletionResponse {
            content: Some("Imagen recibida.".to_string()),
            tool_calls: vec![],
        },
    });
    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
    );

    // 1. JPEG
    let app1 = create_router(state.clone());
    let boundary = "----WebKitFormBoundaryJpegTest";
    let jpeg_bytes = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
    ];
    let body_bytes = build_multipart(
        boundary,
        &[("message", "Usa este afiche")],
        &[("file", "afiche.jpg", "image/jpeg", &jpeg_bytes)],
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
    let res = app1.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "JPEG must be accepted");

    // 2. PNG
    let app2 = create_router(state.clone());
    let boundary = "----WebKitFormBoundaryPngTest";
    let png_bytes = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    ];
    let body_bytes = build_multipart(
        boundary,
        &[("message", "Afiche PNG")],
        &[("file", "afiche.png", "image/png", &png_bytes)],
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
    let res = app2.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "PNG must be accepted");

    // 3. WebP
    let app3 = create_router(state.clone());
    let boundary = "----WebKitFormBoundaryWebpTest";
    let mut webp_bytes = Vec::new();
    webp_bytes.extend_from_slice(b"RIFF");
    webp_bytes.extend_from_slice(&[0x14, 0x00, 0x00, 0x00]); // 20 bytes payload length
    webp_bytes.extend_from_slice(b"WEBP");
    webp_bytes.extend_from_slice(b"VP8 ");
    webp_bytes.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04]);
    let body_bytes = build_multipart(
        boundary,
        &[("message", "Afiche WebP")],
        &[("file", "afiche.webp", "image/webp", &webp_bytes)],
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
    let res = app3.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "WebP must be accepted");
}

#[tokio::test]
async fn test_chat_api_multipart_spoofed_extension_mime_rejected_400() {
    let (app, _, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundarySpoofTest";
    // Declared as PNG with .png and image/png, but content is arbitrary ASCII
    let fake_bytes = b"THIS IS NOT A VALID PNG FILE AT ALL. 1234567890";
    let body_bytes = build_multipart(
        boundary,
        &[("message", "Afiche spoofed")],
        &[("file", "malicious.png", "image/png", fake_bytes)],
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

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Spoofed image content must be rejected with 400"
    );
}

#[tokio::test]
async fn test_chat_api_multipart_image_exceeding_10_mib_rejected_400() {
    let (app, _, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundaryLargeImg";
    // Valid JPEG header but 10 MiB + 1 byte
    let max_bytes = 10 * 1024 * 1024;
    let mut large_img = vec![0u8; max_bytes + 1];
    large_img[0..3].copy_from_slice(&[0xFF, 0xD8, 0xFF]);

    let body_bytes = build_multipart(
        boundary,
        &[("message", "Afiche gigante")],
        &[("file", "gigante.jpg", "image/jpeg", &large_img)],
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

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Image exceeding 10 MiB must be rejected with 400"
    );
}

#[tokio::test]
async fn test_chat_api_multipart_unsupported_file_rejected_400() {
    let (app, _, _tmp) = setup_test_app();

    let boundary = "----WebKitFormBoundaryDocPdf";
    let pdf_bytes = b"%PDF-1.4 dummy pdf content";
    let body_bytes = build_multipart(
        boundary,
        &[("message", "Documento no permitido")],
        &[("file", "doc.pdf", "application/pdf", pdf_bytes)],
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

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "Unsupported file must be rejected with 400"
    );
}

#[tokio::test]
async fn test_chat_api_multipart_image_upload_subir_afiche_e2e_persistence_and_cleanup() {
    use shirley_show::ports::ticketing::{EventRepository, PosterStorage};
    use shirley_show::use_cases::ticketing::CreateEventInput;

    let (_app, state, _tmp) = setup_test_app();

    // 1. Create active event
    let event = state
        .ticketing
        .create_event
        .execute(CreateEventInput {
            nombre: "Concierto Gran Afiche".to_string(),
            fecha_hora: "2026-11-20T21:00:00Z".to_string(),
            lugar: "Teatro Municipal".to_string(),
            tipos_de_entrada: vec![shirley_show::domain::ticketing::TicketType {
                nombre: "VIP".to_string(),
                precio: shirley_show::domain::ticketing::Money::from_soles(50.0).unwrap(),
                capacidad: 200,
            }],
        })
        .unwrap();

    // 2. Setup mock provider to invoke subir_afiche
    let provider = Arc::new(MockProvider {
        response: CompletionResponse {
            content: Some("Afiche subido con éxito.".to_string()),
            tool_calls: vec![ToolCall {
                id: "call_subir_e2e".to_string(),
                name: "subir_afiche".to_string(),
                arguments: json!({ "evento_id": event.id }).to_string(),
            }],
        },
    });

    let state = state.with_agent_provider(
        Some(provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
    );
    let app = create_router(state.clone());

    let boundary = "----WebKitFormBoundaryE2EPoster";
    let jpeg_bytes = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        0x48, 0x00, 0x48, 0x00, 0x00,
    ];

    let body_bytes = build_multipart(
        boundary,
        &[
            ("event_id", &event.id),
            ("message", "Coloca esta imagen como afiche"),
        ],
        &[("file", "afiche_e2e.jpg", "image/jpeg", &jpeg_bytes)],
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

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let actions = json["actions_taken"].as_array().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["tool"], "subir_afiche");
    assert_eq!(actions[0]["status"], "success");
    assert_eq!(actions[0]["event_id"], event.id);

    // Verify disk storage persistence: event now points to poster and poster storage holds the bytes
    let updated_event = state
        .ticketing
        .repo
        .get_event(&event.id)
        .unwrap()
        .expect("Event must exist");
    assert!(updated_event.imagen.is_some(), "Event must have poster set");
    let filename = updated_event.imagen.unwrap();
    let stored = state
        .ticketing
        .poster_storage
        .get_poster(&filename)
        .unwrap();
    assert!(stored.is_some(), "Poster file must exist in poster storage");
    let (data, format) = stored.unwrap();
    assert_eq!(format, shirley_show::ports::ticketing::ImageFormat::Jpeg);
    assert_eq!(data, jpeg_bytes);

    // 3. Turn-scoped cleanup of unconsumed images:
    // Request with image, but provider issues no tool calls.
    let text_provider = Arc::new(MockProvider {
        response: CompletionResponse {
            content: Some("Solo charlando, no usé la imagen.".to_string()),
            tool_calls: vec![],
        },
    });
    let state_unconsumed = state.with_agent_provider(
        Some(text_provider),
        AgentLimits::default(),
        "openai/gpt-oss-120b".to_string(),
    );
    let app2 = create_router(state_unconsumed.clone());

    let boundary2 = "----WebKitFormBoundaryUnconsumed";
    let body_bytes2 = build_multipart(
        boundary2,
        &[("message", "Hola")],
        &[(
            "file",
            "unused.png",
            "image/png",
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        )],
    );
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/agent/chat")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary2),
        )
        .body(Body::from(body_bytes2))
        .unwrap();
    let res2 = app2.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
}
