use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use shirley_show::config::{
    DEFAULT_GROQ_BASE_URL, DEFAULT_GROQ_MODEL, DEFAULT_OPENCODE_BASE_URL, DEFAULT_OPENCODE_MODEL,
    GroqConfig, LlmProviderKind,
};
use shirley_show::domain::agent_message::AgentMessage;
use shirley_show::domain::agent_tool::{ToolCall, ToolDefinition};
use shirley_show::infra::groq::client::{
    CURRENT_SESSION_ID, DEFAULT_MAX_COMPLETION_TOKENS, GroqClient, HEADER_OPENCODE_SESSION,
    USER_AGENT_SHIRLEY,
};
use shirley_show::infra::groq::error::ProviderError;
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Default)]
struct MockServerState {
    last_headers: Arc<Mutex<Option<HeaderMap>>>,
    last_body: Arc<Mutex<Option<Value>>>,
    response_status: Arc<Mutex<StatusCode>>,
    response_body: Arc<Mutex<Value>>,
    delay: Arc<Mutex<Option<Duration>>>,
    request_count: Arc<Mutex<usize>>,
    responses_queue: Arc<Mutex<VecDeque<(StatusCode, Value, Option<String>)>>>,
}

async fn mock_chat_handler(
    State(state): State<MockServerState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    *state.last_headers.lock().await = Some(headers);
    *state.last_body.lock().await = Some(body);

    let mut count = state.request_count.lock().await;
    *count += 1;
    drop(count);

    if let Some(d) = *state.delay.lock().await {
        tokio::time::sleep(d).await;
    }

    let mut queue = state.responses_queue.lock().await;
    if let Some((status, resp, retry_after)) = queue.pop_front() {
        let mut resp_headers = HeaderMap::new();
        if let Some(ra) = retry_after {
            resp_headers.insert(axum::http::header::RETRY_AFTER, ra.parse().unwrap());
        }
        return (status, resp_headers, Json(resp));
    }
    drop(queue);

    let status = *state.response_status.lock().await;
    let resp = state.response_body.lock().await.clone();
    (status, HeaderMap::new(), Json(resp))
}

async fn start_mock_server() -> (SocketAddr, MockServerState) {
    let state = MockServerState {
        last_headers: Arc::new(Mutex::new(None)),
        last_body: Arc::new(Mutex::new(None)),
        response_status: Arc::new(Mutex::new(StatusCode::OK)),
        response_body: Arc::new(Mutex::new(json!({
            "id": "chatcmpl-mock-123",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "¡Hola! Soy Ressy."
                    },
                    "finish_reason": "stop"
                }
            ]
        }))),
        delay: Arc::new(Mutex::new(None)),
        request_count: Arc::new(Mutex::new(0)),
        responses_queue: Arc::new(Mutex::new(VecDeque::new())),
    };

    let app = Router::new()
        .route("/chat/completions", post(mock_chat_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, state)
}

#[tokio::test]
async fn test_groq_client_request_serialization_and_auth_header() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("gsk_test_secret_key_12345".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };

    let client = GroqClient::new(config).expect("client creation");

    let request = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "Eres Ressy, asistente de escenario.".to_string(),
        messages: vec![
            AgentMessage::User {
                content: "Hola Ressy".to_string(),
            },
            AgentMessage::Assistant {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "call_abc123".to_string(),
                    name: "list_presentations".to_string(),
                    arguments: "{}".to_string(),
                }],
            },
            AgentMessage::Tool {
                tool_call_id: "call_abc123".to_string(),
                content: "[]".to_string(),
            },
        ],
        tools: vec![ToolDefinition {
            name: "list_presentations".to_string(),
            description: "Lista las presentaciones disponibles".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }],
    };

    let response = client
        .complete(request)
        .await
        .expect("successful completion");
    assert_eq!(response.content.as_deref(), Some("¡Hola! Soy Ressy."));
    assert!(response.tool_calls.is_empty());

    // Check headers received by mock server
    let headers = state.last_headers.lock().await.clone().unwrap();
    let auth = headers.get("authorization").unwrap().to_str().unwrap();
    assert_eq!(auth, "Bearer gsk_test_secret_key_12345");

    let ua = headers.get("user-agent").unwrap().to_str().unwrap();
    assert_eq!(ua, USER_AGENT_SHIRLEY);

    assert!(headers.get(HEADER_OPENCODE_SESSION).is_some());

    // Check payload received by mock server
    let body = state.last_body.lock().await.clone().unwrap();
    assert_eq!(body["model"], "openai/gpt-oss-120b");
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["max_completion_tokens"], DEFAULT_MAX_COMPLETION_TOKENS);

    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(
        messages[0]["content"],
        "Eres Ressy, asistente de escenario."
    );
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "Hola Ressy");
    assert_eq!(messages[2]["role"], "assistant");
    assert_eq!(messages[2]["tool_calls"][0]["id"], "call_abc123");
    assert_eq!(messages[2]["tool_calls"][0]["type"], "function");
    assert_eq!(
        messages[2]["tool_calls"][0]["function"]["name"],
        "list_presentations"
    );
    assert_eq!(messages[3]["role"], "tool");
    assert_eq!(messages[3]["tool_call_id"], "call_abc123");
    assert_eq!(messages[3]["content"], "[]");

    let tools = body["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], "function");
    assert_eq!(tools[0]["function"]["name"], "list_presentations");
    assert_eq!(
        tools[0]["function"]["description"],
        "Lista las presentaciones disponibles"
    );
}

#[tokio::test]
async fn test_opencode_provider_request_headers_and_session_forwarding() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("opencode_live_token_777".to_string()),
        base_url: format!("http://{}", addr),
        model: "deepseek-v4-flash".to_string(),
        provider: LlmProviderKind::OpenCode,
    };

    let client = GroqClient::new(config).expect("client creation");

    let request = CompletionRequest {
        model: "deepseek-v4-flash".to_string(),
        system_prompt: "Eres Ressy en OpenCode Go.".to_string(),
        messages: vec![AgentMessage::User {
            content: "Hola desde OpenCode".to_string(),
        }],
        tools: vec![],
    };

    let stable_session = "session-conversation-uuid-999".to_string();

    let response = CURRENT_SESSION_ID
        .scope(stable_session.clone(), client.complete(request))
        .await
        .expect("successful completion");
    assert_eq!(response.content.as_deref(), Some("¡Hola! Soy Ressy."));

    let headers = state.last_headers.lock().await.clone().unwrap();
    assert_eq!(
        headers.get("authorization").unwrap().to_str().unwrap(),
        "Bearer opencode_live_token_777"
    );
    assert_eq!(
        headers.get("user-agent").unwrap().to_str().unwrap(),
        "shirley-show/1.0"
    );
    assert_eq!(
        headers
            .get(HEADER_OPENCODE_SESSION)
            .unwrap()
            .to_str()
            .unwrap(),
        "session-conversation-uuid-999"
    );

    let body = state.last_body.lock().await.clone().unwrap();
    assert_eq!(body["model"], "deepseek-v4-flash");
    assert_eq!(body["max_completion_tokens"], DEFAULT_MAX_COMPLETION_TOKENS);
}

#[tokio::test]
async fn test_ephemeral_session_generated_when_context_missing() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("any_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "deepseek-v4-flash".to_string(),
        provider: LlmProviderKind::OpenCode,
    };

    let client = GroqClient::new(config).unwrap();
    let request = CompletionRequest {
        model: "deepseek-v4-flash".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "ping".to_string(),
        }],
        tools: vec![],
    };

    // Complete without setting CURRENT_SESSION_ID
    let _ = client.complete(request).await.unwrap();

    let headers = state.last_headers.lock().await.clone().unwrap();
    let session_header = headers.get(HEADER_OPENCODE_SESSION);
    assert!(session_header.is_some());
    let session_str = session_header.unwrap().to_str().unwrap();
    // Must be a non-empty generated UUID v4
    assert!(uuid::Uuid::parse_str(session_str).is_ok());
}

#[tokio::test]
async fn test_groq_client_tool_call_response_parsing() {
    let (addr, state) = start_mock_server().await;

    *state.response_body.lock().await = json!({
        "id": "chatcmpl-mock-456",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_create_1",
                            "type": "function",
                            "function": {
                                "name": "create_presentation",
                                "arguments": "{\"name\":\"Show Acústico\"}"
                            }
                        }
                    ]
                },
                "finish_reason": "tool_calls"
            }
        ]
    });

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };

    let client = GroqClient::new(config).unwrap();
    let request = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "crea show".to_string(),
        }],
        tools: vec![],
    };

    let response = client.complete(request).await.expect("parse tool call");
    assert_eq!(response.content, None);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "call_create_1");
    assert_eq!(response.tool_calls[0].name, "create_presentation");
    assert_eq!(
        response.tool_calls[0].arguments,
        "{\"name\":\"Show Acústico\"}"
    );
}

#[tokio::test]
async fn test_groq_client_base_url_normalization() {
    let (addr, _) = start_mock_server().await;

    // Base URL with multiple trailing slashes
    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}///", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };

    let client = GroqClient::new(config).unwrap();
    let request = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "ping".to_string(),
        }],
        tools: vec![],
    };

    let response = client.complete(request).await;
    assert!(
        response.is_ok(),
        "Base url trailing slashes should be normalized cleanly"
    );
}

#[tokio::test]
async fn test_groq_client_http_error_mappings() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };
    let client = GroqClient::new(config).unwrap();

    let make_req = || CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "test".to_string(),
        }],
        tools: vec![],
    };

    // 401 Unauthorized -> Authentication
    *state.response_status.lock().await = StatusCode::UNAUTHORIZED;
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Authentication));
    assert_eq!(err.code(), "provider_auth");

    // 429 Too Many Requests -> RateLimited
    *state.response_status.lock().await = StatusCode::TOO_MANY_REQUESTS;
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::RateLimited));
    assert_eq!(err.code(), "provider_rate_limited");

    // 500 Internal Server Error -> Unavailable
    *state.response_status.lock().await = StatusCode::INTERNAL_SERVER_ERROR;
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Unavailable));
    assert_eq!(err.code(), "provider_unavailable");

    // 503 Service Unavailable -> Unavailable
    *state.response_status.lock().await = StatusCode::SERVICE_UNAVAILABLE;
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Unavailable));
    assert_eq!(err.code(), "provider_unavailable");

    // 400 Bad Request / 404 Not Found -> Rejected / MalformedResponse -> provider_error
    *state.response_status.lock().await = StatusCode::BAD_REQUEST;
    let err = client.complete(make_req()).await.unwrap_err();
    assert_eq!(err.code(), "provider_error");
}

#[tokio::test]
async fn test_groq_client_malformed_response() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };
    let client = GroqClient::new(config).unwrap();

    let make_req = || CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "test".to_string(),
        }],
        tools: vec![],
    };

    // Empty choices array
    *state.response_status.lock().await = StatusCode::OK;
    *state.response_body.lock().await = json!({
        "id": "chatcmpl-empty",
        "choices": []
    });
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::MalformedResponse(_)));
    assert_eq!(err.code(), "provider_error");

    // Choice without message content or tool calls
    *state.response_body.lock().await = json!({
        "id": "chatcmpl-nocontent",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": []
                }
            }
        ]
    });
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::MalformedResponse(_)));
    assert_eq!(err.code(), "provider_error");
}

#[tokio::test]
async fn test_groq_client_timeout_mapping() {
    let (addr, state) = start_mock_server().await;

    // Set delay longer than client timeout
    *state.delay.lock().await = Some(Duration::from_millis(200));

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };

    // Build client with tiny timeout for test
    let client = GroqClient::with_timeout(config, Duration::from_millis(50)).unwrap();

    let req = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "test".to_string(),
        }],
        tools: vec![],
    };

    let err = client.complete(req).await.unwrap_err();
    assert!(matches!(err, ProviderError::Timeout));
    assert_eq!(err.code(), "agent_timeout");
}

#[test]
fn test_groq_config_validation_and_trimming() {
    // Normalization of base_url and trimming of api_key for Groq
    let mut config = GroqConfig {
        api_key: Some("   gsk_xyz123   ".to_string()),
        base_url: "https://api.groq.com/openai/v1/".to_string(),
        model: "  openai/gpt-oss-120b  ".to_string(),
        provider: LlmProviderKind::Groq,
    };
    config.normalize();

    assert_eq!(config.api_key.as_deref(), Some("gsk_xyz123"));
    assert_eq!(config.base_url, "https://api.groq.com/openai/v1");
    assert_eq!(config.model, "openai/gpt-oss-120b");

    // Whitespace-only api_key becomes None
    let mut config2 = GroqConfig {
        api_key: Some("   \t  \n".to_string()),
        base_url: "https://api.groq.com/openai/v1".to_string(),
        model: "model".to_string(),
        provider: LlmProviderKind::Groq,
    };
    config2.normalize();
    assert_eq!(config2.api_key, None);
}

#[test]
fn test_opencode_config_validation_and_trimming() {
    // Normalization of base_url and trimming of api_key for OpenCode Go
    let mut config = GroqConfig {
        api_key: Some("   opencode_secret_key_1   ".to_string()),
        base_url: "https://opencode.ai/zen/go/v1///".to_string(),
        model: "  deepseek-v4-flash  ".to_string(),
        provider: LlmProviderKind::OpenCode,
    };
    config.normalize();

    assert_eq!(config.api_key.as_deref(), Some("opencode_secret_key_1"));
    assert_eq!(config.base_url, "https://opencode.ai/zen/go/v1");
    assert_eq!(config.model, "deepseek-v4-flash");
    assert_eq!(config.provider_name(), "OpenCode Go");
    assert!(config.is_opencode());

    // Empty base_url defaults to OpenCode base url
    let mut config_empty = GroqConfig {
        api_key: None,
        base_url: "   ".to_string(),
        model: "".to_string(),
        provider: LlmProviderKind::OpenCode,
    };
    config_empty.normalize();
    assert_eq!(config_empty.base_url, DEFAULT_OPENCODE_BASE_URL);
    assert_eq!(config_empty.model, DEFAULT_OPENCODE_MODEL);
}

#[tokio::test]
async fn test_groq_client_retries_twice_on_429_then_succeeds() {
    let (addr, state) = start_mock_server().await;

    // Queue 2 rate limits (429), then 200 OK
    *state.responses_queue.lock().await = VecDeque::from(vec![
        (
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "error": { "message": "Rate limit exceeded" } }),
            Some("0.01".to_string()),
        ),
        (
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "error": { "message": "Rate limit exceeded" } }),
            None,
        ),
    ]);

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };
    let client = GroqClient::new(config).unwrap();

    let request = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "hola".to_string(),
        }],
        tools: vec![],
    };

    let response = client
        .complete(request)
        .await
        .expect("successful completion after 2 retries");
    assert_eq!(response.content.as_deref(), Some("¡Hola! Soy Ressy."));
    // Exactly 3 requests: initial attempt + 2 retries
    assert_eq!(*state.request_count.lock().await, 3);
}

#[tokio::test]
async fn test_groq_client_retries_twice_on_5xx_then_succeeds() {
    let (addr, state) = start_mock_server().await;

    // Queue 2 server errors (503 and 500), then default 200 OK
    *state.responses_queue.lock().await = VecDeque::from(vec![
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": { "message": "Backend service overloaded" } }),
            None,
        ),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "message": "Internal error" } }),
            None,
        ),
    ]);

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };
    let client = GroqClient::new(config).unwrap();

    let request = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "hola".to_string(),
        }],
        tools: vec![],
    };

    let response = client
        .complete(request)
        .await
        .expect("successful completion after 2 retries");
    assert_eq!(response.content.as_deref(), Some("¡Hola! Soy Ressy."));
    // Exactly 3 requests: initial attempt + 2 retries
    assert_eq!(*state.request_count.lock().await, 3);
}

#[tokio::test]
async fn test_groq_client_no_retry_on_400_or_401() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };
    let client = GroqClient::new(config).unwrap();

    let make_req = || CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "test".to_string(),
        }],
        tools: vec![],
    };

    // 401 Unauthorized should fail immediately without retry (1 request)
    *state.response_status.lock().await = StatusCode::UNAUTHORIZED;
    *state.request_count.lock().await = 0;
    let err = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Authentication));
    assert_eq!(*state.request_count.lock().await, 1);

    // 400 Bad Request should fail immediately without retry (1 request)
    *state.response_status.lock().await = StatusCode::BAD_REQUEST;
    *state.request_count.lock().await = 0;
    let err2 = client.complete(make_req()).await.unwrap_err();
    assert!(matches!(err2, ProviderError::Rejected(_)));
    assert_eq!(*state.request_count.lock().await, 1);
}

#[tokio::test]
async fn test_groq_client_max_retries_exceeded_on_429() {
    let (addr, state) = start_mock_server().await;

    *state.response_status.lock().await = StatusCode::TOO_MANY_REQUESTS;
    *state.response_body.lock().await = json!({
        "error": { "message": "Rate limit continuous" }
    });

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "openai/gpt-oss-120b".to_string(),
        provider: LlmProviderKind::Groq,
    };
    let client = GroqClient::new(config).unwrap();

    let request = CompletionRequest {
        model: "openai/gpt-oss-120b".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User {
            content: "test".to_string(),
        }],
        tools: vec![],
    };

    let err = client.complete(request).await.unwrap_err();
    assert!(matches!(err, ProviderError::RateLimited));
    // 1 initial attempt + 3 retries = 4 total requests
    assert_eq!(*state.request_count.lock().await, 4);
}

#[test]
fn test_provider_selection_env_hierarchy_opencode_first() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        std::env::set_var("OPENCODE_API_KEY", "opencode_token_alpha");
        std::env::set_var("GROQ_API_KEY", "groq_token_beta");
        std::env::remove_var("OPENCODE_MODEL");
        std::env::remove_var("OPENCODE_BASE_URL");
        std::env::remove_var("GROQ_MODEL");
        std::env::remove_var("GROQ_BASE_URL");

        let config = GroqConfig::from_env();
        assert_eq!(config.provider, LlmProviderKind::OpenCode);
        assert_eq!(config.api_key.as_deref(), Some("opencode_token_alpha"));
        assert_eq!(config.model, DEFAULT_OPENCODE_MODEL);
        assert_eq!(config.base_url, DEFAULT_OPENCODE_BASE_URL);

        // Clean up
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("GROQ_API_KEY");
    }
}

#[test]
fn test_provider_selection_fallback_to_groq_when_opencode_missing() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::set_var("GROQ_API_KEY", "groq_token_delta");
        std::env::remove_var("OPENCODE_MODEL");
        std::env::remove_var("OPENCODE_BASE_URL");
        std::env::remove_var("GROQ_MODEL");
        std::env::remove_var("GROQ_BASE_URL");

        let config = GroqConfig::from_env();
        assert_eq!(config.provider, LlmProviderKind::Groq);
        assert_eq!(config.api_key.as_deref(), Some("groq_token_delta"));
        assert_eq!(config.model, DEFAULT_GROQ_MODEL);
        assert_eq!(config.base_url, DEFAULT_GROQ_BASE_URL);

        // Clean up
        std::env::remove_var("GROQ_API_KEY");
    }
}

#[test]
fn test_provider_selection_whitespace_opencode_key_falls_back_to_groq() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        std::env::set_var("OPENCODE_API_KEY", "   \t  \n ");
        std::env::set_var("GROQ_API_KEY", "groq_token_fallback");

        let config = GroqConfig::from_env();
        assert_eq!(config.provider, LlmProviderKind::Groq);
        assert_eq!(config.api_key.as_deref(), Some("groq_token_fallback"));

        // Clean up
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("GROQ_API_KEY");
    }
}

#[test]
fn test_provider_selection_unconfigured_when_no_keys() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("GROQ_API_KEY");
        std::env::remove_var("OPENCODE_MODEL");
        std::env::remove_var("OPENCODE_BASE_URL");

        let config = GroqConfig::from_env();
        assert_eq!(config.api_key, None);
        assert_eq!(config.provider, LlmProviderKind::OpenCode);
        assert_eq!(config.model, DEFAULT_OPENCODE_MODEL);
        assert_eq!(config.base_url, DEFAULT_OPENCODE_BASE_URL);
    }
}

#[test]
fn test_provider_selection_custom_model_and_base_url() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        std::env::set_var("OPENCODE_API_KEY", "opencode_custom_key");
        std::env::set_var("OPENCODE_MODEL", "deepseek-custom-exp");
        std::env::set_var("OPENCODE_BASE_URL", "https://custom.opencode.ai/zen/v1/");

        let config = GroqConfig::from_env();
        assert_eq!(config.provider, LlmProviderKind::OpenCode);
        assert_eq!(config.api_key.as_deref(), Some("opencode_custom_key"));
        assert_eq!(config.model, "deepseek-custom-exp");
        assert_eq!(config.base_url, "https://custom.opencode.ai/zen/v1");

        // Clean up
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("OPENCODE_MODEL");
        std::env::remove_var("OPENCODE_BASE_URL");
    }
}

#[test]
fn test_groq_config_debug_redacts_api_key() {
    let config = GroqConfig {
        api_key: Some("super_secret_raw_key_never_leak".to_string()),
        base_url: "https://opencode.ai/zen/go/v1".to_string(),
        model: "deepseek-v4-flash".to_string(),
        provider: LlmProviderKind::OpenCode,
    };

    let debug_output = format!("{:?}", config);
    assert!(debug_output.contains("[REDACTED]"));
    assert!(!debug_output.contains("super_secret_raw_key_never_leak"));
}

#[test]
fn test_parse_null_tool_calls_as_empty() {
    use shirley_show::infra::groq::dto::GroqChatCompletionResponse;
    let raw = r#"{"id":"x","choices":[{"message":{"role":"assistant","content":"hola","tool_calls":null}}]}"#;
    let parsed: GroqChatCompletionResponse = serde_json::from_str(raw).unwrap();
    assert!(parsed.choices[0].message.tool_calls.is_empty());
}
