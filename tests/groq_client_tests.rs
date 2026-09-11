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

use shirley_show::config::GroqConfig;
use shirley_show::domain::agent_message::AgentMessage;
use shirley_show::domain::agent_tool::{ToolCall, ToolDefinition};
use shirley_show::infra::groq::client::GroqClient;
use shirley_show::infra::groq::error::ProviderError;
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};

#[derive(Clone, Default)]
struct MockServerState {
    last_headers: Arc<Mutex<Option<HeaderMap>>>,
    last_body: Arc<Mutex<Option<Value>>>,
    response_status: Arc<Mutex<StatusCode>>,
    response_body: Arc<Mutex<Value>>,
    delay: Arc<Mutex<Option<Duration>>>,
}

async fn mock_chat_handler(
    State(state): State<MockServerState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    *state.last_headers.lock().await = Some(headers);
    *state.last_body.lock().await = Some(body);

    if let Some(d) = *state.delay.lock().await {
        tokio::time::sleep(d).await;
    }

    let status = *state.response_status.lock().await;
    let resp = state.response_body.lock().await.clone();
    (status, Json(resp))
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
                        "content": "¡Hola! Soy Shirley."
                    },
                    "finish_reason": "stop"
                }
            ]
        }))),
        delay: Arc::new(Mutex::new(None)),
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
        model: "llama-3.3-70b-versatile".to_string(),
    };

    let client = GroqClient::new(config).expect("client creation");

    let request = CompletionRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        system_prompt: "Eres Shirley, asistente de escenario.".to_string(),
        messages: vec![
            AgentMessage::User {
                content: "Hola Shirley".to_string(),
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

    let response = client.complete(request).await.expect("successful completion");
    assert_eq!(response.content.as_deref(), Some("¡Hola! Soy Shirley."));
    assert!(response.tool_calls.is_empty());

    // Check headers received by mock server
    let headers = state.last_headers.lock().await.clone().unwrap();
    let auth = headers.get("authorization").unwrap().to_str().unwrap();
    assert_eq!(auth, "Bearer gsk_test_secret_key_12345");

    // Check payload received by mock server
    let body = state.last_body.lock().await.clone().unwrap();
    assert_eq!(body["model"], "llama-3.3-70b-versatile");
    assert_eq!(body["tool_choice"], "auto");

    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "Eres Shirley, asistente de escenario.");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "Hola Shirley");
    assert_eq!(messages[2]["role"], "assistant");
    assert_eq!(messages[2]["tool_calls"][0]["id"], "call_abc123");
    assert_eq!(messages[2]["tool_calls"][0]["type"], "function");
    assert_eq!(messages[2]["tool_calls"][0]["function"]["name"], "list_presentations");
    assert_eq!(messages[3]["role"], "tool");
    assert_eq!(messages[3]["tool_call_id"], "call_abc123");
    assert_eq!(messages[3]["content"], "[]");

    let tools = body["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], "function");
    assert_eq!(tools[0]["function"]["name"], "list_presentations");
    assert_eq!(tools[0]["function"]["description"], "Lista las presentaciones disponibles");
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
        model: "llama-3.3-70b-versatile".to_string(),
    };

    let client = GroqClient::new(config).unwrap();
    let request = CompletionRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User { content: "crea show".to_string() }],
        tools: vec![],
    };

    let response = client.complete(request).await.expect("parse tool call");
    assert_eq!(response.content, None);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "call_create_1");
    assert_eq!(response.tool_calls[0].name, "create_presentation");
    assert_eq!(response.tool_calls[0].arguments, "{\"name\":\"Show Acústico\"}");
}

#[tokio::test]
async fn test_groq_client_base_url_normalization() {
    let (addr, _) = start_mock_server().await;

    // Base URL with multiple trailing slashes
    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}///", addr),
        model: "llama-3.3-70b-versatile".to_string(),
    };

    let client = GroqClient::new(config).unwrap();
    let request = CompletionRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User { content: "ping".to_string() }],
        tools: vec![],
    };

    let response = client.complete(request).await;
    assert!(response.is_ok(), "Base url trailing slashes should be normalized cleanly");
}

#[tokio::test]
async fn test_groq_client_http_error_mappings() {
    let (addr, state) = start_mock_server().await;

    let config = GroqConfig {
        api_key: Some("test_key".to_string()),
        base_url: format!("http://{}", addr),
        model: "llama-3.3-70b-versatile".to_string(),
    };
    let client = GroqClient::new(config).unwrap();

    let make_req = || CompletionRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User { content: "test".to_string() }],
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
        model: "llama-3.3-70b-versatile".to_string(),
    };
    let client = GroqClient::new(config).unwrap();

    let make_req = || CompletionRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User { content: "test".to_string() }],
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
        model: "llama-3.3-70b-versatile".to_string(),
    };

    // Build client with tiny timeout for test
    let client = GroqClient::with_timeout(config, Duration::from_millis(50)).unwrap();

    let req = CompletionRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        system_prompt: "sys".to_string(),
        messages: vec![AgentMessage::User { content: "test".to_string() }],
        tools: vec![],
    };

    let err = client.complete(req).await.unwrap_err();
    assert!(matches!(err, ProviderError::Timeout));
    assert_eq!(err.code(), "agent_timeout");
}

#[test]
fn test_groq_config_validation_and_trimming() {
    // Normalization of base_url and trimming of api_key
    let mut config = GroqConfig {
        api_key: Some("   gsk_xyz123   ".to_string()),
        base_url: "https://api.groq.com/openai/v1/".to_string(),
        model: "  llama-3.3-70b-versatile  ".to_string(),
    };
    config.normalize();

    assert_eq!(config.api_key.as_deref(), Some("gsk_xyz123"));
    assert_eq!(config.base_url, "https://api.groq.com/openai/v1");
    assert_eq!(config.model, "llama-3.3-70b-versatile");

    // Whitespace-only api_key becomes None
    let mut config2 = GroqConfig {
        api_key: Some("   \t  \n".to_string()),
        base_url: "https://api.groq.com/openai/v1".to_string(),
        model: "model".to_string(),
    };
    config2.normalize();
    assert_eq!(config2.api_key, None);
}
