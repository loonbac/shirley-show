use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use rand::Rng;
use reqwest::{Client, StatusCode};

use crate::config::GroqConfig;
use crate::domain::agent_message::{AgentMessage, CompletionResponse};
use crate::domain::agent_tool::{ToolCall, ToolDefinition};
use crate::infra::groq::dto::{
    GroqChatCompletionResponse, GroqFunctionCall, GroqFunctionDefinition, GroqMessage, GroqTool,
    GroqToolCall,
};
use crate::infra::groq::error::ProviderError;
use crate::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};

pub const DEFAULT_GROQ_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_GROQ_RETRIES: u32 = 3;
pub const DEFAULT_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
pub const DEFAULT_MAX_COMPLETION_TOKENS: u32 = 800;
pub const USER_AGENT_SHIRLEY: &str = "shirley-show/1.0";
pub const HEADER_OPENCODE_SESSION: &str = "x-opencode-session";

tokio::task_local! {
    pub static CURRENT_SESSION_ID: String;
}

#[derive(Debug, serde::Serialize)]
struct GroqChatCompletionWireRequest {
    pub model: String,
    pub messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<GroqTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    pub max_completion_tokens: u32,
}

fn parse_retry_after(response: &reqwest::Response) -> Option<Duration> {
    let header_val = response.headers().get(reqwest::header::RETRY_AFTER)?;
    let s = header_val.to_str().ok()?.trim();
    if let Ok(seconds) = s.parse::<f64>() {
        if seconds >= 0.0 {
            return Some(Duration::from_secs_f64(seconds));
        }
    }
    if let Ok(seconds) = s.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    if let Ok(date) = chrono::DateTime::parse_from_rfc2822(s) {
        let now = chrono::Utc::now();
        if let Ok(diff) = (date.with_timezone(&chrono::Utc) - now).to_std() {
            return Some(diff);
        }
    }
    None
}

fn calculate_backoff(attempt: u32, retry_after: Option<Duration>) -> Duration {
    if let Some(duration) = retry_after {
        return duration;
    }
    let base_ms = DEFAULT_INITIAL_BACKOFF
        .as_millis()
        .saturating_mul(1u128 << attempt.min(6)) as u64;
    let jitter_ms = rand::thread_rng().gen_range(0..=50);
    Duration::from_millis(base_ms.saturating_add(jitter_ms))
}

#[derive(Clone)]
pub struct GroqClient {
    client: Client,
    config: GroqConfig,
}

impl GroqClient {
    pub fn new(config: GroqConfig) -> Result<Self, ProviderError> {
        Self::with_timeout(config, DEFAULT_GROQ_TIMEOUT)
    }

    pub fn with_timeout(mut config: GroqConfig, timeout: Duration) -> Result<Self, ProviderError> {
        config.normalize();
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(USER_AGENT_SHIRLEY)
            .build()
            .map_err(|e| ProviderError::Rejected(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self { client, config })
    }

    pub fn config(&self) -> &GroqConfig {
        &self.config
    }
}

impl ChatCompletionProvider for GroqClient {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>> {
        let client = self.client.clone();
        let base_url = self.config.base_url.clone();
        let api_key = self.config.api_key.clone();

        Box::pin(async move {
            let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

            let session_id = CURRENT_SESSION_ID
                .try_with(|id| id.clone())
                .unwrap_or_else(|_| {
                    // Generar un ID de sesión efímero cuando no se recibe en el contexto del turno.
                    // Documentación: Generar una sesión efímera por request es menos óptimo para
                    // la reutilización de caché de prefijos (prefix caching) en OpenCode Go / vLLM.
                    uuid::Uuid::new_v4().to_string()
                });

            let force_first_tool = !request
                .messages
                .iter()
                .any(|message| matches!(message, AgentMessage::Tool { .. }));
            let mut wire_messages = Vec::with_capacity(request.messages.len() + 1);
            wire_messages.push(GroqMessage::System {
                content: request.system_prompt,
            });

            for msg in request.messages {
                match msg {
                    AgentMessage::System { content } => {
                        wire_messages.push(GroqMessage::System { content });
                    }
                    AgentMessage::User { content } => {
                        wire_messages.push(GroqMessage::User { content });
                    }
                    AgentMessage::Assistant {
                        content,
                        tool_calls,
                    } => {
                        let wire_tool_calls: Vec<GroqToolCall> = tool_calls
                            .into_iter()
                            .map(|tc| GroqToolCall {
                                id: tc.id,
                                tool_type: "function".to_string(),
                                function: GroqFunctionCall {
                                    name: tc.name,
                                    arguments: tc.arguments,
                                },
                            })
                            .collect();
                        // OpenCode Go requires `content` to be present on an assistant
                        // message that carries tool calls; other OpenAI-compatible APIs
                        // also accept the empty string.
                        let content = if wire_tool_calls.is_empty() {
                            content
                        } else {
                            Some(content.unwrap_or_default())
                        };
                        wire_messages.push(GroqMessage::Assistant {
                            content,
                            tool_calls: wire_tool_calls,
                        });
                    }
                    AgentMessage::Tool {
                        tool_call_id,
                        content,
                    } => {
                        wire_messages.push(GroqMessage::Tool {
                            tool_call_id,
                            content,
                        });
                    }
                }
            }

            let wire_tools: Vec<GroqTool> = request
                .tools
                .into_iter()
                .map(|t: ToolDefinition| GroqTool {
                    tool_type: "function".to_string(),
                    function: GroqFunctionDefinition {
                        name: t.name,
                        description: t.description,
                        parameters: t.parameters,
                    },
                })
                .collect();

            let tool_choice = if !wire_tools.is_empty() {
                Some(
                    if force_first_tool && wire_tools.len() <= 3 {
                        "required"
                    } else {
                        "auto"
                    }
                    .to_string(),
                )
            } else {
                None
            };

            let body = GroqChatCompletionWireRequest {
                model: request.model,
                messages: wire_messages,
                tools: wire_tools,
                tool_choice,
                max_completion_tokens: DEFAULT_MAX_COMPLETION_TOKENS,
            };

            let mut retries: u32 = 0;
            loop {
                let mut req_builder = client
                    .post(&endpoint)
                    .header(reqwest::header::USER_AGENT, USER_AGENT_SHIRLEY)
                    .header(HEADER_OPENCODE_SESSION, &session_id)
                    .json(&body);
                if let Some(ref key) = api_key {
                    req_builder = req_builder.bearer_auth(key);
                }

                let response = req_builder.send().await.map_err(|e| {
                    if e.is_timeout() {
                        ProviderError::Timeout
                    } else if e.is_connect() {
                        ProviderError::Unavailable
                    } else {
                        ProviderError::Rejected("Network request failed".to_string())
                    }
                })?;

                let status = response.status();
                if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                    && retries < MAX_GROQ_RETRIES
                {
                    let retry_after = parse_retry_after(&response);
                    let backoff = calculate_backoff(retries, retry_after);
                    retries += 1;
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                match status {
                    StatusCode::OK => {
                        let raw_body = response.text().await.map_err(|_| {
                            ProviderError::MalformedResponse(
                                "Invalid response body".to_string(),
                            )
                        })?;
                        let parsed: GroqChatCompletionResponse =
                            serde_json::from_str(&raw_body).map_err(|error| {
                                eprintln!(
                                    "OpenCode response schema error: {error}; body: {}",
                                    raw_body.chars().take(2_000).collect::<String>()
                                );
                                ProviderError::MalformedResponse(
                                    "Invalid response JSON schema".to_string(),
                                )
                            })?;

                        let choice = parsed.choices.into_iter().next().ok_or_else(|| {
                            ProviderError::MalformedResponse("Choices array is empty".to_string())
                        })?;

                        let mut tool_calls = Vec::new();
                        for tc in choice.message.tool_calls {
                            if tc.id.is_empty() || tc.function.name.is_empty() {
                                eprintln!(
                                    "OpenCode invalid tool call: id={:?}, name={:?}, arguments={:?}",
                                    tc.id, tc.function.name, tc.function.arguments
                                );
                                return Err(ProviderError::MalformedResponse(
                                    "Invalid tool call attributes".to_string(),
                                ));
                            }
                            tool_calls.push(ToolCall {
                                id: tc.id,
                                name: tc.function.name,
                                arguments: tc.function.arguments,
                            });
                        }

                        let content = choice
                            .message
                            .content
                            .map(|c| c.trim().to_string())
                            .filter(|c| !c.is_empty());

                        if content.is_none() && tool_calls.is_empty() {
                            eprintln!("OpenCode response contained neither text nor tool calls");
                            return Err(ProviderError::MalformedResponse(
                                "Response contains neither content nor tool calls".to_string(),
                            ));
                        }

                        return Ok(CompletionResponse {
                            content,
                            tool_calls,
                        });
                    }
                    StatusCode::UNAUTHORIZED => return Err(ProviderError::Authentication),
                    StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::RateLimited),
                    s if s.is_server_error() => return Err(ProviderError::Unavailable),
                    _ => {
                        let body = response.text().await.unwrap_or_default();
                        eprintln!(
                            "OpenCode returned HTTP {}: {}",
                            status.as_u16(),
                            body.chars().take(2_000).collect::<String>()
                        );
                        return Err(ProviderError::Rejected(format!(
                            "Provider returned HTTP {}",
                            status.as_u16()
                        )));
                    }
                }
            }
        })
    }
}
