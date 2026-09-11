use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Mutex;

use shirley_show::agent::tool_dispatcher::AgentToolDispatcher;
use shirley_show::domain::agent_message::{AgentMessage, CompletionResponse, HistoryMessage};
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::infra::filesystem::LocalAudioStorage;
use shirley_show::infra::groq::error::ProviderError;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};
use shirley_show::state::AppState;
use shirley_show::use_cases::agent_chat::{AgentChat, AgentChatInput, AgentLimits};

#[derive(Clone, Default)]
struct StubChatProvider {
    responses: Arc<Mutex<VecDeque<Result<CompletionResponse, ProviderError>>>>,
    recorded_requests: Arc<Mutex<Vec<CompletionRequest>>>,
    delay: Option<Duration>,
}

impl StubChatProvider {
    fn new(responses: Vec<Result<CompletionResponse, ProviderError>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            recorded_requests: Arc::new(Mutex::new(Vec::new())),
            delay: None,
        }
    }

    fn with_delay(delay: Duration) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            recorded_requests: Arc::new(Mutex::new(Vec::new())),
            delay: Some(delay),
        }
    }
}

impl ChatCompletionProvider for StubChatProvider {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>> {
        let responses = self.responses.clone();
        let recorded = self.recorded_requests.clone();
        let delay = self.delay;

        Box::pin(async move {
            recorded.lock().await.push(request);
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }

            let mut lock = responses.lock().await;
            lock.pop_front().unwrap_or_else(|| {
                Ok(CompletionResponse {
                    content: Some("Default fallback".to_string()),
                    tool_calls: vec![],
                })
            })
        })
    }
}

fn setup_test_state() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let audio_dir = temp_dir.path().join("audio");
    std::fs::create_dir_all(audio_dir.join("files")).unwrap();
    std::fs::create_dir_all(audio_dir.join("staging")).unwrap();

    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(audio_dir));
    let state = AppState::new_with_items(pool, storage);
    (state, temp_dir)
}

#[tokio::test]
async fn test_agent_chat_unconfigured_provider() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    let agent_chat = AgentChat::new(
        None,
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Hola Ressy".to_string(),
        history: vec![],
        presentation_id: Some("pres-1".to_string()),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    assert_eq!(output.active_presentation_id.as_deref(), Some("pres-1"));
    assert!(output.notice.is_some());
    let notice = output.notice.unwrap();
    assert_eq!(notice.code, "agent_unconfigured");
    assert_eq!(notice.retryable, false);
    assert!(output.message.contains("GROQ_API_KEY"));
}

#[tokio::test]
async fn test_agent_chat_single_turn_informational() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(StubChatProvider::new(vec![Ok(CompletionResponse {
        content: Some("¡Hola! Puedo crear presentaciones y agregar canciones.".to_string()),
        tool_calls: vec![],
    })]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "llama-3.3-70b-versatile".to_string(),
    );

    let input = AgentChatInput {
        message: "¿Qué puedes hacer?".to_string(),
        history: vec![],
        presentation_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    assert_eq!(
        output.message,
        "¡Hola! Puedo crear presentaciones y agregar canciones."
    );
    assert!(output.notice.is_none());
}

#[tokio::test]
async fn test_agent_chat_multi_iteration_tool_execution_chain() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    // Iteration 1: call create_presentation
    // Iteration 2: call add_note and add_lyrics with new ID
    // Iteration 3: final text
    struct ChainedProvider {
        iter: Arc<Mutex<usize>>,
    }
    impl ChatCompletionProvider for ChainedProvider {
        fn complete(
            &self,
            request: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>>
        {
            let iter = self.iter.clone();
            Box::pin(async move {
                let mut i = iter.lock().await;
                *i += 1;
                if *i == 1 {
                    Ok(CompletionResponse {
                        content: None,
                        tool_calls: vec![ToolCall {
                            id: "call_c1".to_string(),
                            name: "create_presentation".to_string(),
                            arguments: json!({ "name": "Show Acústico" }).to_string(),
                        }],
                    })
                } else if *i == 2 {
                    // Extract new presentation ID from tool messages
                    let mut created_id = "unknown".to_string();
                    for m in &request.messages {
                        if let AgentMessage::Tool { content, .. } = m {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
                                if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                                    created_id = id.to_string();
                                }
                            }
                        }
                    }
                    Ok(CompletionResponse {
                        content: None,
                        tool_calls: vec![
                            ToolCall {
                                id: "call_n1".to_string(),
                                name: "add_note".to_string(),
                                arguments:
                                    json!({ "presentation_id": created_id, "text": "Afinar en Re" })
                                        .to_string(),
                            },
                            ToolCall {
                                id: "call_l1".to_string(),
                                name: "add_lyrics".to_string(),
                                arguments:
                                    json!({ "presentation_id": created_id, "text": "Estrofa 1" })
                                        .to_string(),
                            },
                        ],
                    })
                } else {
                    Ok(CompletionResponse {
                        content: Some("Listo, show creado con nota y letra.".to_string()),
                        tool_calls: vec![],
                    })
                }
            })
        }
    }

    let agent_chat = AgentChat::new(
        Some(Arc::new(ChainedProvider {
            iter: Arc::new(Mutex::new(0)),
        })),
        dispatcher,
        AgentLimits::default(),
        "llama-3.3-70b-versatile".to_string(),
    );

    let input = AgentChatInput {
        message: "Crea el show Acústico, agrega una nota y una letra".to_string(),
        history: vec![],
        presentation_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 3);
    assert_eq!(output.actions_taken[0].tool, "create_presentation");
    assert_eq!(output.actions_taken[1].tool, "add_note");
    assert_eq!(output.actions_taken[2].tool, "add_lyrics");
    assert!(output.active_presentation_id.is_some());
    // Grounding: message reflects confirmed actions
    assert!(
        output
            .message
            .contains("Presentación “Show Acústico” creada")
    );
}

#[tokio::test]
async fn test_agent_chat_iteration_limit_5() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    // Provider that always returns a tool call to list_presentations
    struct InfiniteProvider;
    impl ChatCompletionProvider for InfiniteProvider {
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>>
        {
            Box::pin(async {
                Ok(CompletionResponse {
                    content: None,
                    tool_calls: vec![ToolCall {
                        id: "call_list".to_string(),
                        name: "list_presentations".to_string(),
                        arguments: "{}".to_string(),
                    }],
                })
            })
        }
    }

    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_secs(10),
    };
    let agent_chat = AgentChat::new(
        Some(Arc::new(InfiniteProvider)),
        dispatcher,
        limits,
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Loop infinito".to_string(),
        history: vec![],
        presentation_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert!(output.notice.is_some());
    let notice = output.notice.unwrap();
    assert_eq!(notice.code, "iteration_limit");
    assert_eq!(notice.retryable, true);
}

#[tokio::test]
async fn test_agent_chat_deadline_timeout() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    // Provider that delays 200ms
    let provider = Arc::new(StubChatProvider::with_delay(Duration::from_millis(200)));

    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_millis(50),
    };
    let agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());

    let input = AgentChatInput {
        message: "Mensaje lento".to_string(),
        history: vec![],
        presentation_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert!(output.notice.is_some());
    let notice = output.notice.unwrap();
    assert_eq!(notice.code, "agent_timeout");
    assert_eq!(notice.retryable, true);
}

#[tokio::test]
async fn test_agent_chat_partial_success_preserved_on_provider_error() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    // Iteration 1: create presentation
    // Iteration 2: provider error (503)
    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_c1".to_string(),
                name: "create_presentation".to_string(),
                arguments: json!({ "name": "Show Permanente" }).to_string(),
            }],
        }),
        Err(ProviderError::Unavailable),
    ]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Crea el show".to_string(),
        history: vec![],
        presentation_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "create_presentation");
    assert!(output.notice.is_some());
    assert_eq!(output.notice.unwrap().code, "provider_unavailable");
    // Committed action is preserved in message summary
    assert!(
        output
            .message
            .contains("Presentación “Show Permanente” creada")
    );
}

#[tokio::test]
async fn test_agent_chat_history_truncation_strict_char_cap_and_system_note() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(StubChatProvider::new(vec![Ok(CompletionResponse {
        content: Some("Respuesta".to_string()),
        tool_calls: vec![],
    })]));

    let agent_chat = AgentChat::new(
        Some(provider.clone()),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let huge_content = "á".repeat(33_000);
    let input_huge = AgentChatInput {
        message: "Hola".to_string(),
        history: vec![HistoryMessage {
            role: "user".to_string(),
            content: huge_content,
        }],
        presentation_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let _ = agent_chat.execute(input_huge).await;
    let requests = provider.recorded_requests.lock().await;
    let msgs = &requests[0].messages;

    assert_eq!(msgs.len(), 2);
    match &msgs[0] {
        AgentMessage::System { content } => {
            assert!(content.contains("fueron podados"));
        }
        other => panic!(
            "Expected AgentMessage::System for pruning note, got {:?}",
            other
        ),
    }
    match &msgs[1] {
        AgentMessage::User { content } => {
            assert_eq!(content, "Hola");
        }
        other => panic!("Expected current user message, got {:?}", other),
    }
}

#[test]
fn test_system_prompt_never_assumes_intent_and_never_fetches_lyrics() {
    use shirley_show::agent::system_prompt::SHIRLEY_SYSTEM_PROMPT;
    assert!(
        SHIRLEY_SYSTEM_PROMPT.contains("NUNCA busques ni inventes letras"),
        "el prompt debe prohibir buscar/inventar letras"
    );
    assert!(
        SHIRLEY_SYSTEM_PROMPT.contains("NUNCA sobreentiendas su intención"),
        "el prompt debe exigir preguntar ante la duda"
    );
    assert!(
        SHIRLEY_SYSTEM_PROMPT.contains("NO ejecutes nada y pregúntale"),
        "el prompt debe exigir título y presentación antes de agregar"
    );
}
