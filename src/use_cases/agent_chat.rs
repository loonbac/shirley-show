use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::agent::system_prompt::{SHIRLEY_SYSTEM_PROMPT, build_context_system_prompt};
use crate::agent::tool_dispatcher::AgentToolDispatcher;
use crate::agent::tool_schemas::get_tool_definitions;
use crate::agent::turn_context::{CommittedTurnState, StagedAudioHandle, TurnToolContext};
use crate::domain::agent_message::{AgentMessage, HistoryMessage};
use crate::domain::agent_tool::{AgentNotice, ToolAction};
use crate::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};

#[derive(Debug, Clone)]
pub struct AgentLimits {
    pub max_iterations: usize,
    pub request_timeout: Duration,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            request_timeout: Duration::from_secs(30),
        }
    }
}

pub struct AgentChatInput {
    pub message: String,
    pub history: Vec<HistoryMessage>,
    pub presentation_id: Option<String>,
    pub staged_audio: Option<StagedAudioHandle>,
    pub staged_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatOutput {
    pub message: String,
    pub actions_taken: Vec<ToolAction>,
    pub active_presentation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<AgentNotice>,
}

pub struct AgentChat {
    provider: Option<Arc<dyn ChatCompletionProvider>>,
    dispatcher: AgentToolDispatcher,
    pub limits: AgentLimits,
    model: String,
}

impl AgentChat {
    pub fn new(
        provider: Option<Arc<dyn ChatCompletionProvider>>,
        dispatcher: AgentToolDispatcher,
        limits: AgentLimits,
        model: String,
    ) -> Self {
        Self {
            provider,
            dispatcher,
            limits,
            model,
        }
    }

    pub async fn execute(&self, input: AgentChatInput) -> AgentChatOutput {
        self.execute_with_timeout(input, self.limits.request_timeout)
            .await
    }

    pub async fn execute_with_timeout(
        &self,
        input: AgentChatInput,
        timeout_duration: Duration,
    ) -> AgentChatOutput {
        let provider = match self.provider.as_ref() {
            Some(p) => p.clone(),
            None => {
                return AgentChatOutput {
                    message: "Para hablar conmigo y gestionar tus shows con IA, necesitas configurar la variable de entorno GROQ_API_KEY en el servidor. Todas las funciones manuales siguen operando con normalidad.".to_string(),
                    actions_taken: vec![],
                    active_presentation_id: input.presentation_id,
                    notice: Some(AgentNotice {
                        code: "agent_unconfigured".to_string(),
                        retryable: false,
                    }),
                };
            }
        };

        let has_staged_audio = input.staged_audio.is_some();
        let staged_filename = input.staged_filename.clone();
        let initial_presentation_id = input.presentation_id.clone();
        let user_message = input.message.trim().to_string();

        let context_prompt = build_context_system_prompt(
            initial_presentation_id.as_deref(),
            has_staged_audio,
            staged_filename.as_deref(),
        );
        let full_system_prompt = format!("{}\n\n{}", SHIRLEY_SYSTEM_PROMPT, context_prompt);

        let (mut conversation_messages, was_pruned) = prune_history(input.history);
        if was_pruned {
            conversation_messages.insert(
                0,
                AgentMessage::System {
                    content: "[Nota del sistema: Mensajes anteriores de la conversación fueron podados para respetar el límite de contexto]"
                        .to_string(),
                },
            );
        }
        conversation_messages.push(AgentMessage::User {
            content: user_message,
        });

        let tracker = Arc::new(Mutex::new(CommittedTurnState {
            actions_taken: Vec::new(),
            active_presentation_id: initial_presentation_id.clone(),
        }));
        let tracker_for_timeout = tracker.clone();

        let mut turn_context = TurnToolContext::with_tracker(
            initial_presentation_id,
            input.staged_audio,
            Some(tracker),
        );

        let timeout_result = tokio::time::timeout(timeout_duration, async {
            for iter in 0..self.limits.max_iterations {
                let req = CompletionRequest {
                    model: self.model.clone(),
                    system_prompt: full_system_prompt.clone(),
                    messages: conversation_messages.clone(),
                    tools: get_tool_definitions(),
                };

                let response = match provider.complete(req).await {
                    Ok(resp) => resp,
                    Err(err) => {
                        let notice = AgentNotice {
                            code: err.code().to_string(),
                            retryable: err.retryable(),
                        };
                        let message = if turn_context.actions_taken.is_empty() {
                            err.friendly_message().to_string()
                        } else {
                            compose_grounded_message(
                                &turn_context.actions_taken,
                                Some(err.friendly_message()),
                            )
                        };
                        return AgentChatOutput {
                            message,
                            actions_taken: turn_context.actions_taken,
                            active_presentation_id: turn_context.active_presentation_id,
                            notice: Some(notice),
                        };
                    }
                };

                if !response.tool_calls.is_empty() {
                    conversation_messages.push(AgentMessage::Assistant {
                        content: response.content.clone(),
                        tool_calls: response.tool_calls.clone(),
                    });

                    for tc in response.tool_calls {
                        let res = self.dispatcher.dispatch(&tc, &mut turn_context);
                        conversation_messages.push(AgentMessage::Tool {
                            tool_call_id: tc.id,
                            content: res.to_string(),
                        });
                    }

                    if iter + 1 == self.limits.max_iterations {
                        let notice = AgentNotice {
                            code: "iteration_limit".to_string(),
                            retryable: true,
                        };
                        let message = compose_grounded_message(
                            &turn_context.actions_taken,
                            Some("Alcancé el límite de pasos permitidos para este turno. Si necesitas continuar, indícamelo en un nuevo mensaje."),
                        );
                        return AgentChatOutput {
                            message,
                            actions_taken: turn_context.actions_taken,
                            active_presentation_id: turn_context.active_presentation_id,
                            notice: Some(notice),
                        };
                    }
                } else {
                    let message = if turn_context.actions_taken.is_empty() {
                        response
                            .content
                            .unwrap_or_else(|| "No pude generar una respuesta.".to_string())
                    } else {
                        compose_grounded_message(&turn_context.actions_taken, None)
                    };

                    return AgentChatOutput {
                        message,
                        actions_taken: turn_context.actions_taken,
                        active_presentation_id: turn_context.active_presentation_id,
                        notice: None,
                    };
                }
            }

            AgentChatOutput {
                message: compose_grounded_message(&turn_context.actions_taken, None),
                actions_taken: turn_context.actions_taken,
                active_presentation_id: turn_context.active_presentation_id,
                notice: None,
            }
        })
        .await;

        match timeout_result {
            Ok(output) => output,
            Err(_) => {
                let committed = tracker_for_timeout.lock().unwrap();
                AgentChatOutput {
                    message: compose_grounded_message(
                        &committed.actions_taken,
                        Some("La solicitud tardó más del límite de tiempo (30 s) y fue cancelada."),
                    ),
                    actions_taken: committed.actions_taken.clone(),
                    active_presentation_id: committed.active_presentation_id.clone(),
                    notice: Some(AgentNotice {
                        code: "agent_timeout".to_string(),
                        retryable: true,
                    }),
                }
            }
        }
    }
}

fn prune_history(history: Vec<HistoryMessage>) -> (Vec<AgentMessage>, bool) {
    let history_len = history.len();
    let mut valid_messages = Vec::new();
    for msg in history {
        match msg.role.to_lowercase().as_str() {
            "user" => valid_messages.push(AgentMessage::User {
                content: msg.content,
            }),
            "assistant" => valid_messages.push(AgentMessage::Assistant {
                content: Some(msg.content),
                tool_calls: vec![],
            }),
            _ => {}
        }
    }

    let mut char_count = 0;
    let mut selected_rev = Vec::new();
    let mut pruned = false;

    for msg in valid_messages.into_iter().rev() {
        let len = match &msg {
            AgentMessage::User { content } => content.chars().count(),
            AgentMessage::Assistant { content, .. } => {
                content.as_ref().map(|c| c.chars().count()).unwrap_or(0)
            }
            _ => 0,
        };

        if selected_rev.len() >= 20 || char_count + len > 32_000 {
            pruned = true;
            break;
        }

        char_count += len;
        selected_rev.push(msg);
    }

    selected_rev.reverse();
    let was_pruned = pruned || selected_rev.len() < history_len;
    (selected_rev, was_pruned)
}

fn compose_grounded_message(actions_taken: &[ToolAction], extra_notice: Option<&str>) -> String {
    if actions_taken.is_empty() {
        if let Some(notice) = extra_notice {
            return notice.to_string();
        }
        return "No se completaron acciones en este turno.".to_string();
    }

    let summaries: Vec<&str> = actions_taken.iter().map(|a| a.summary.as_str()).collect();
    let actions_text = summaries.join(". ") + ".";

    if let Some(notice) = extra_notice {
        format!("{}\n\n{}", actions_text, notice)
    } else {
        actions_text
    }
}
