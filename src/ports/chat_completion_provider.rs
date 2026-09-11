use std::future::Future;
use std::pin::Pin;

use crate::domain::agent_message::{AgentMessage, CompletionResponse};
use crate::domain::agent_tool::ToolDefinition;
use crate::infra::groq::error::ProviderError;

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<ToolDefinition>,
}

pub trait ChatCompletionProvider: Send + Sync {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>>;
}
