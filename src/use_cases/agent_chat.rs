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
    pub separation_timeout: Duration,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            request_timeout: Duration::from_secs(120),
            separation_timeout: Duration::from_secs(600),
        }
    }
}

pub struct AgentChatInput {
    pub message: String,
    pub history: Vec<HistoryMessage>,
    pub presentation_id: Option<String>,
    pub event_id: Option<String>,
    pub staged_audio_id: Option<String>,
    pub staged_audio: Option<StagedAudioHandle>,
    pub staged_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChatOutput {
    pub message: String,
    pub actions_taken: Vec<ToolAction>,
    pub active_presentation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_audio_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<AgentNotice>,
}

pub struct AgentChat {
    provider: Option<Arc<dyn ChatCompletionProvider>>,
    dispatcher: AgentToolDispatcher,
    pub limits: AgentLimits,
    model: String,
    pub remote_vocal_enabled: bool,
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
            remote_vocal_enabled: false,
        }
    }

    pub fn with_remote_vocal_enabled(mut self, enabled: bool) -> Self {
        self.remote_vocal_enabled = enabled;
        self
    }

    pub fn set_remote_vocal_enabled(&mut self, enabled: bool) {
        self.remote_vocal_enabled = enabled;
    }

    pub fn is_remote_vocal_enabled(&self) -> bool {
        self.remote_vocal_enabled
    }

    pub async fn execute(&self, input: AgentChatInput) -> AgentChatOutput {
        self.execute_dynamic(input, tokio::time::Instant::now())
            .await
    }

    pub async fn execute_with_timeout(
        &self,
        input: AgentChatInput,
        timeout_duration: Duration,
    ) -> AgentChatOutput {
        let deadline = tokio::time::Instant::now() + timeout_duration;
        self.execute_until(input, deadline).await
    }

    pub async fn execute_until(
        &self,
        input: AgentChatInput,
        deadline: tokio::time::Instant,
    ) -> AgentChatOutput {
        self.execute_dynamic_with_bounds(input, tokio::time::Instant::now(), Some(deadline))
            .await
    }

    pub async fn execute_dynamic(
        &self,
        input: AgentChatInput,
        start_time: tokio::time::Instant,
    ) -> AgentChatOutput {
        self.execute_dynamic_with_bounds(input, start_time, None)
            .await
    }

    pub async fn execute_dynamic_with_bounds(
        &self,
        input: AgentChatInput,
        start_time: tokio::time::Instant,
        fixed_deadline: Option<tokio::time::Instant>,
    ) -> AgentChatOutput {
        let provider = match self.provider.as_ref() {
            Some(p) => p.clone(),
            None => {
                return AgentChatOutput {
                    message: "Para hablar conmigo y gestionar tus shows con IA, necesitas configurar la variable de entorno GROQ_API_KEY en el servidor. Todas las funciones manuales siguen operando con normalidad.".to_string(),
                    actions_taken: vec![],
                    active_presentation_id: input.presentation_id,
                    active_event_id: input.event_id,
                    staged_audio_id: None,
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
        let initial_event_id = input.event_id.clone();
        let user_message = input.message.trim().to_string();

        let context_prompt = build_context_system_prompt(
            initial_presentation_id.as_deref(),
            initial_event_id.as_deref(),
            has_staged_audio,
            staged_filename.as_deref(),
            input.staged_audio_id.as_deref(),
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
            active_event_id: initial_event_id.clone(),
            staged_audio_id: input.staged_audio_id.clone(),
        }));
        let tracker_for_timeout = tracker.clone();

        let mut turn_context = TurnToolContext::with_event_context(
            initial_presentation_id,
            initial_event_id,
            input.staged_audio,
            input.staged_audio_id,
            Some(tracker),
        );

        let mut effective_budget = Duration::from_secs(30).min(self.limits.request_timeout);
        let mut current_deadline = fixed_deadline.unwrap_or_else(|| start_time + effective_budget);

        for iter in 0..self.limits.max_iterations {
            let now = tokio::time::Instant::now();
            if now >= current_deadline {
                let committed = tracker_for_timeout.lock().unwrap();
                let timeout_secs = effective_budget.as_secs();
                let timeout_msg = format!(
                    "La solicitud tardó más del límite de tiempo ({timeout_secs} s) y fue cancelada."
                );
                return AgentChatOutput {
                    message: compose_grounded_message(
                        &committed.actions_taken,
                        Some(&timeout_msg),
                        committed.staged_audio_id.as_deref(),
                    ),
                    actions_taken: committed.actions_taken.clone(),
                    active_presentation_id: committed.active_presentation_id.clone(),
                    active_event_id: committed.active_event_id.clone(),
                    staged_audio_id: committed.staged_audio_id.clone(),
                    notice: Some(AgentNotice {
                        code: "agent_timeout".to_string(),
                        retryable: true,
                    }),
                };
            }

            let req = CompletionRequest {
                model: self.model.clone(),
                system_prompt: full_system_prompt.clone(),
                messages: conversation_messages.clone(),
                tools: get_tool_definitions(),
            };

            let remaining = current_deadline.saturating_duration_since(now);
            let response = match tokio::time::timeout(remaining, provider.complete(req)).await {
                Ok(Ok(resp)) => resp,
                Ok(Err(err)) => {
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
                            turn_context.staged_audio_id.as_deref(),
                        )
                    };
                    return AgentChatOutput {
                        message,
                        actions_taken: turn_context.actions_taken,
                        active_presentation_id: turn_context.active_presentation_id,
                    active_event_id: turn_context.active_event_id.clone(),
                        staged_audio_id: turn_context.staged_audio_id,
                        notice: Some(notice),
                    };
                }
                Err(_) => {
                    let committed = tracker_for_timeout.lock().unwrap();
                    let timeout_secs = effective_budget.as_secs();
                    let timeout_msg = format!(
                        "La solicitud tardó más del límite de tiempo ({timeout_secs} s) y fue cancelada."
                    );
                    return AgentChatOutput {
                        message: compose_grounded_message(
                            &committed.actions_taken,
                            Some(&timeout_msg),
                            committed.staged_audio_id.as_deref(),
                        ),
                        actions_taken: committed.actions_taken.clone(),
                        active_presentation_id: committed.active_presentation_id.clone(),
                    active_event_id: committed.active_event_id.clone(),
                        staged_audio_id: committed.staged_audio_id.clone(),
                        notice: Some(AgentNotice {
                            code: "agent_timeout".to_string(),
                            retryable: true,
                        }),
                    };
                }
            };

            if !response.tool_calls.is_empty() {
                if fixed_deadline.is_none() {
                    let has_separate = response
                        .tool_calls
                        .iter()
                        .any(|tc| tc.name == "separate_vocals");
                    let has_download = response
                        .tool_calls
                        .iter()
                        .any(|tc| tc.name == "download_youtube_track");

                    if has_separate {
                        let target = self.limits.separation_timeout;
                        if target > effective_budget {
                            effective_budget = target;
                            current_deadline = start_time + target;
                        }
                    } else if has_download {
                        let target = Duration::from_secs(120).min(self.limits.request_timeout);
                        if target > effective_budget {
                            effective_budget = target;
                            current_deadline = start_time + target;
                        }
                    }
                }

                conversation_messages.push(AgentMessage::Assistant {
                    content: response.content.clone(),
                    tool_calls: response.tool_calls.clone(),
                });

                for tc in response.tool_calls {
                    let now = tokio::time::Instant::now();
                    if now >= current_deadline {
                        let committed = tracker_for_timeout.lock().unwrap();
                        let timeout_secs = effective_budget.as_secs();
                        let timeout_msg = format!(
                            "La solicitud tardó más del límite de tiempo ({timeout_secs} s) y fue cancelada."
                        );
                        return AgentChatOutput {
                            message: compose_grounded_message(
                                &committed.actions_taken,
                                Some(&timeout_msg),
                                committed.staged_audio_id.as_deref(),
                            ),
                            actions_taken: committed.actions_taken.clone(),
                            active_presentation_id: committed.active_presentation_id.clone(),
                    active_event_id: committed.active_event_id.clone(),
                            staged_audio_id: committed.staged_audio_id.clone(),
                            notice: Some(AgentNotice {
                                code: "agent_timeout".to_string(),
                                retryable: true,
                            }),
                        };
                    }

                    let remaining = current_deadline.saturating_duration_since(now);
                    let res = match tokio::time::timeout(
                        remaining,
                        self.dispatcher.dispatch(&tc, &mut turn_context),
                    )
                    .await
                    {
                        Ok(val) => val,
                        Err(_) => {
                            let committed = tracker_for_timeout.lock().unwrap();
                            let timeout_secs = effective_budget.as_secs();
                            let timeout_msg = format!(
                                "La solicitud tardó más del límite de tiempo ({timeout_secs} s) y fue cancelada."
                            );
                            return AgentChatOutput {
                                message: compose_grounded_message(
                                    &committed.actions_taken,
                                    Some(&timeout_msg),
                                    committed.staged_audio_id.as_deref(),
                                ),
                                actions_taken: committed.actions_taken.clone(),
                                active_presentation_id: committed.active_presentation_id.clone(),
                    active_event_id: committed.active_event_id.clone(),
                                staged_audio_id: committed.staged_audio_id.clone(),
                                notice: Some(AgentNotice {
                                    code: "agent_timeout".to_string(),
                                    retryable: true,
                                }),
                            };
                        }
                    };

                    if tc.name == "separate_vocals"
                        && let Some(err_code) = res.get("error").and_then(|v| v.as_str())
                    {
                        if err_code == "wake_wait" {
                            let msg = res.get("message").and_then(|v| v.as_str()).unwrap_or(
                                    "Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo. Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.",
                                );
                            return AgentChatOutput {
                                message: msg.to_string(),
                                actions_taken: turn_context.actions_taken,
                                active_presentation_id: turn_context.active_presentation_id,
                    active_event_id: turn_context.active_event_id.clone(),
                                staged_audio_id: turn_context.staged_audio_id,
                                notice: Some(AgentNotice {
                                    code: "wake_wait".to_string(),
                                    retryable: true,
                                }),
                            };
                        } else if err_code == "pc_wake_timeout" {
                            let msg = "No pude conectarme con la PC de Joshua después de 10 minutos. Cuando esté encendida, vuelve a pedirme la separación.";
                            return AgentChatOutput {
                                message: msg.to_string(),
                                actions_taken: turn_context.actions_taken,
                                active_presentation_id: turn_context.active_presentation_id,
                    active_event_id: turn_context.active_event_id.clone(),
                                staged_audio_id: turn_context.staged_audio_id,
                                notice: Some(AgentNotice {
                                    code: "pc_wake_timeout".to_string(),
                                    retryable: true,
                                }),
                            };
                        }
                    }

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
                        Some(
                            "Alcancé el límite de pasos permitidos para este turno. Si necesitas continuar, indícamelo en un nuevo mensaje.",
                        ),
                        turn_context.staged_audio_id.as_deref(),
                    );
                    return AgentChatOutput {
                        message,
                        actions_taken: turn_context.actions_taken,
                        active_presentation_id: turn_context.active_presentation_id,
                    active_event_id: turn_context.active_event_id.clone(),
                        staged_audio_id: turn_context.staged_audio_id,
                        notice: Some(notice),
                    };
                }
            } else {
                let mut message = if turn_context.actions_taken.is_empty() {
                    let raw = response
                        .content
                        .unwrap_or_else(|| "No pude generar una respuesta.".to_string());
                    sanitize_ungrounded_claims(&raw)
                } else {
                    compose_grounded_message(
                        &turn_context.actions_taken,
                        None,
                        turn_context.staged_audio_id.as_deref(),
                    )
                };

                if self.remote_vocal_enabled
                    && turn_context
                        .actions_taken
                        .iter()
                        .any(|a| a.tool == "separate_vocals" && a.status == "success")
                {
                    message.push_str("\n\nCuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.");
                }

                return AgentChatOutput {
                    message,
                    actions_taken: turn_context.actions_taken,
                    active_presentation_id: turn_context.active_presentation_id,
                    active_event_id: turn_context.active_event_id.clone(),
                    staged_audio_id: turn_context.staged_audio_id,
                    notice: None,
                };
            }
        }

        let mut message = compose_grounded_message(
            &turn_context.actions_taken,
            None,
            turn_context.staged_audio_id.as_deref(),
        );

        if self.remote_vocal_enabled
            && turn_context
                .actions_taken
                .iter()
                .any(|a| a.tool == "separate_vocals" && a.status == "success")
        {
            message.push_str("\n\nCuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.");
        }

        AgentChatOutput {
            message,
            actions_taken: turn_context.actions_taken,
            active_presentation_id: turn_context.active_presentation_id,
                    active_event_id: turn_context.active_event_id.clone(),
            staged_audio_id: turn_context.staged_audio_id,
            notice: None,
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

fn compose_grounded_message(
    actions_taken: &[ToolAction],
    extra_notice: Option<&str>,
    staged_audio_id: Option<&str>,
) -> String {
    if actions_taken.is_empty() {
        if let Some(notice) = extra_notice {
            return notice.to_string();
        }
        return "No se completaron acciones en este turno.".to_string();
    }

    let summaries: Vec<&str> = actions_taken.iter().map(|a| a.summary.as_str()).collect();
    let actions_text = summaries.join(". ") + ".";

    let with_staged_question = if staged_audio_id.is_some() {
        format!(
            "{}\n\nEl audio está preparado y caduca en 15 minutos. ¿A qué presentación deseas agregarlo?",
            actions_text
        )
    } else {
        actions_text
    };

    if let Some(notice) = extra_notice {
        format!("{}\n\n{}", with_staged_question, notice)
    } else {
        with_staged_question
    }
}

fn sanitize_ungrounded_claims(content: &str) -> String {
    let lower = content.to_lowercase();
    let false_claim_patterns = [
        "descargué",
        "he descargado",
        "audio descargado",
        "video descargado",
        "separé las voces",
        "he separado las voces",
        "separé los coros",
        "voces separadas",
        "agregué la pista",
        "he agregado la pista",
        "pista agregada",
        "creé la presentación",
        "he creado la presentación",
        "presentación creada",
        "añadí la pista",
        "he añadido la pista",
    ];

    if false_claim_patterns
        .iter()
        .any(|pattern| lower.contains(pattern))
    {
        "No se completaron acciones en este turno. Si deseas realizar una acción como descargar un audio o gestionar una presentación, por favor indícamelo.".to_string()
    } else {
        content.to_string()
    }
}
