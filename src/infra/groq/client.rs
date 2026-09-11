use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use reqwest::{Client, StatusCode};

use crate::config::GroqConfig;
use crate::domain::agent_message::{AgentMessage, CompletionResponse};
use crate::domain::agent_tool::{ToolCall, ToolDefinition};
use crate::infra::groq::dto::{
    GroqChatCompletionRequest, GroqChatCompletionResponse, GroqFunctionCall,
    GroqFunctionDefinition, GroqMessage, GroqTool, GroqToolCall,
};
use crate::infra::groq::error::ProviderError;
use crate::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};

pub const DEFAULT_GROQ_TIMEOUT: Duration = Duration::from_secs(30);

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

            let mut wire_messages = Vec::with_capacity(request.messages.len() + 1);
            wire_messages.push(GroqMessage::System {
                content: request.system_prompt,
            });

            for msg in request.messages {
                match msg {
                    AgentMessage::User { content } => {
                        wire_messages.push(GroqMessage::User { content });
                    }
                    AgentMessage::Assistant {
                        content,
                        tool_calls,
                    } => {
                        let wire_tool_calls = tool_calls
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
                Some("auto".to_string())
            } else {
                None
            };

            let body = GroqChatCompletionRequest {
                model: request.model,
                messages: wire_messages,
                tools: wire_tools,
                tool_choice,
            };

            let mut req_builder = client.post(&endpoint).json(&body);
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
            match status {
                StatusCode::OK => {
                    let parsed: GroqChatCompletionResponse = response.json().await.map_err(|_| {
                        ProviderError::MalformedResponse("Invalid response JSON schema".to_string())
                    })?;

                    let choice = parsed.choices.into_iter().next().ok_or_else(|| {
                        ProviderError::MalformedResponse("Choices array is empty".to_string())
                    })?;

                    let mut tool_calls = Vec::new();
                    for tc in choice.message.tool_calls {
                        if tc.id.is_empty() || tc.function.name.is_empty() {
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
                        return Err(ProviderError::MalformedResponse(
                            "Response contains neither content nor tool calls".to_string(),
                        ));
                    }

                    Ok(CompletionResponse {
                        content,
                        tool_calls,
                    })
                }
                StatusCode::UNAUTHORIZED => Err(ProviderError::Authentication),
                StatusCode::TOO_MANY_REQUESTS => Err(ProviderError::RateLimited),
                s if s.is_server_error() => Err(ProviderError::Unavailable),
                _ => Err(ProviderError::Rejected(format!(
                    "Provider returned HTTP {}",
                    status.as_u16()
                ))),
            }
        })
    }
}
