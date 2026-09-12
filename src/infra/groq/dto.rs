use serde::{Deserialize, Serialize};

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let opt = Option::<T>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Serialize)]
pub struct GroqChatCompletionRequest {
    pub model: String,
    pub messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<GroqTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum GroqMessage {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        tool_calls: Vec<GroqToolCall>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: GroqFunctionDefinition,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqFunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: GroqFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct GroqChatCompletionResponse {
    pub id: Option<String>,
    pub choices: Vec<GroqChoice>,
}

#[derive(Debug, Deserialize)]
pub struct GroqChoice {
    pub index: Option<i32>,
    pub message: GroqChoiceMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GroqChoiceMessage {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub tool_calls: Vec<GroqToolCall>,
}
