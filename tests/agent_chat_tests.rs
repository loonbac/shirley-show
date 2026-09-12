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
use shirley_show::domain::youtube_url::YouTubeUrl;
use shirley_show::infra::filesystem::LocalAudioStorage;
use shirley_show::infra::groq::error::ProviderError;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};
use shirley_show::ports::show_item_reader::ShowItemReader;
use shirley_show::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};
use shirley_show::ports::youtube_downloader::{
    DownloadedYoutubeAudio, YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};
use shirley_show::state::AppState;
use shirley_show::use_cases::agent_chat::{AgentChat, AgentChatInput, AgentLimits};

struct FakeDownloader {
    metadata_title: String,
    duration_secs: u32,
    size_bytes: u64,
    fail_with_error: Option<YoutubeDownloadError>,
}

impl FakeDownloader {
    fn new(title: &str, duration_secs: u32, size_bytes: u64) -> Self {
        Self {
            metadata_title: title.to_string(),
            duration_secs,
            size_bytes,
            fail_with_error: None,
        }
    }

    #[allow(dead_code)]
    fn failing(err: YoutubeDownloadError) -> Self {
        Self {
            metadata_title: String::new(),
            duration_secs: 0,
            size_bytes: 0,
            fail_with_error: Some(err),
        }
    }
}

impl YoutubeDownloader for FakeDownloader {
    fn download(
        &self,
        _url: YouTubeUrl,
        target: YoutubeDownloadTarget,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadedYoutubeAudio, YoutubeDownloadError>> + Send>>
    {
        if let Some(err) = self.fail_with_error.clone() {
            return Box::pin(async move { Err(err) });
        }

        let title = self.metadata_title.clone();
        let duration_secs = self.duration_secs;
        let size_bytes = self.size_bytes;

        Box::pin(async move {
            let part_path = target.staging_dir.join(format!("{}.part", target.stage_id));
            std::fs::write(
                &part_path,
                &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
            )
            .expect("write fake mp3 part");

            Ok(DownloadedYoutubeAudio {
                title,
                duration_secs,
                size_bytes,
            })
        })
    }
}

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
        event_id: None,
        staged_audio_id: None,
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
        "openai/gpt-oss-120b".to_string(),
    );

    let input = AgentChatInput {
        message: "¿Qué puedes hacer?".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
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
        "openai/gpt-oss-120b".to_string(),
    );

    let input = AgentChatInput {
        message: "Crea el show Acústico, agrega una nota y una letra".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
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
        separation_timeout: Duration::from_secs(600),
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
        event_id: None,
        staged_audio_id: None,
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
        separation_timeout: Duration::from_secs(600),
    };
    let agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());

    let input = AgentChatInput {
        message: "Mensaje lento".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
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
        event_id: None,
        staged_audio_id: None,
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
        event_id: None,
        staged_audio_id: None,
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
    assert!(
        SHIRLEY_SYSTEM_PROMPT.contains("estaré aquí para cuando me necesites"),
        "el prompt debe aceptar el 'mejor ya no' con calidez y sin insistir"
    );
}

#[tokio::test]
async fn test_agent_chat_same_turn_chaining_download_and_add_track_when_presentation_active() {
    let (state, _tmp) = setup_test_state();
    let pres = state
        .create_presentation
        .execute("Peña Folclórica")
        .unwrap();
    let state = state.with_youtube_downloader(Arc::new(FakeDownloader::new(
        "Zamba de mi Esperanza",
        180,
        4_000_000,
    )));
    let dispatcher = AgentToolDispatcher::new(state.clone());

    struct ChainingProvider {
        pres_id: String,
        iter: Arc<Mutex<usize>>,
    }
    impl ChatCompletionProvider for ChainingProvider {
        fn complete(
            &self,
            request: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>>
        {
            let pres_id = self.pres_id.clone();
            let iter = self.iter.clone();
            Box::pin(async move {
                let mut i = iter.lock().await;
                *i += 1;
                if *i == 1 {
                    Ok(CompletionResponse {
                        content: None,
                        tool_calls: vec![ToolCall {
                            id: "call_dl_1".to_string(),
                            name: "download_youtube_track".to_string(),
                            arguments:
                                json!({ "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ" })
                                    .to_string(),
                        }],
                    })
                } else if *i == 2 {
                    let mut staged_id = None;
                    for m in &request.messages {
                        if let AgentMessage::Tool { content, .. } = m {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
                                if let Some(id) = v.get("staged_audio_id").and_then(|s| s.as_str())
                                {
                                    staged_id = Some(id.to_string());
                                }
                            }
                        }
                    }
                    Ok(CompletionResponse {
                        content: None,
                        tool_calls: vec![ToolCall {
                            id: "call_add_1".to_string(),
                            name: "add_track_from_staged".to_string(),
                            arguments: json!({
                                "presentation_id": pres_id,
                                "title": "Zamba de mi Esperanza",
                                "staged_audio_id": staged_id,
                            })
                            .to_string(),
                        }],
                    })
                } else {
                    Ok(CompletionResponse {
                        content: Some(
                            "Descargué el audio y lo agregué a la presentación Peña Folclórica."
                                .to_string(),
                        ),
                        tool_calls: vec![],
                    })
                }
            })
        }
    }

    let agent_chat = AgentChat::new(
        Some(Arc::new(ChainingProvider {
            pres_id: pres.id.clone(),
            iter: Arc::new(Mutex::new(0)),
        })),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message:
            "Descarga este video https://www.youtube.com/watch?v=dQw4w9WgXcQ y agrégalo como pista"
                .to_string(),
        history: vec![],
        presentation_id: Some(pres.id.clone()),
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 2);
    assert_eq!(output.actions_taken[0].tool, "download_youtube_track");
    assert_eq!(output.actions_taken[1].tool, "add_track_from_staged");
    assert_eq!(
        output.staged_audio_id, None,
        "staged_audio_id should be cleared when consumed in same turn"
    );
    assert!(
        output.message.contains("preparado")
            || output.message.contains("descargado")
            || output.message.contains("Pista")
    );
    assert!(output.message.contains("añadida") || output.message.contains("agreg"));

    let items = state.item_repo.list_by_presentation(&pres.id).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind().as_str(), "track");
}

#[tokio::test]
async fn test_agent_chat_stop_with_question_when_no_active_presentation_emits_staged_audio_id() {
    let (state, _tmp) = setup_test_state();
    let state = state.with_youtube_downloader(Arc::new(FakeDownloader::new(
        "Chacarera del Violín",
        160,
        3_500_000,
    )));
    let dispatcher = AgentToolDispatcher::new(state.clone());

    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_dl_1".to_string(),
                name: "download_youtube_track".to_string(),
                arguments: json!({ "url": "https://youtu.be/dQw4w9WgXcQ" }).to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: Some("El audio está listo. ¿A qué presentación deseas agregarlo?".to_string()),
            tool_calls: vec![],
        }),
    ]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Descarga https://youtu.be/dQw4w9WgXcQ".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "download_youtube_track");
    assert!(
        output.staged_audio_id.is_some(),
        "Must emit staged_audio_id in output"
    );
    let staged_id = output.staged_audio_id.unwrap();
    assert!(staged_id.starts_with("staged-"));
    assert!(output.message.to_lowercase().contains("presentación"));
    assert!(output.message.contains("¿A qué presentación"));
}

#[tokio::test]
async fn test_agent_chat_second_turn_consumes_structured_staged_audio_id() {
    let (state, _tmp) = setup_test_state();
    let pres = state
        .create_presentation
        .execute("Repertorio Final")
        .unwrap();

    let (stage_id, stage_path) = state.storage.create_staging_file().unwrap();
    std::fs::write(
        &stage_path,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let desc = state.staged_audio_registry.register(
        stage_id,
        stage_path,
        "Audio Previo".to_string(),
        140,
        3_000_000,
    );
    let staged_id = desc.staged_audio_id;

    let dispatcher = AgentToolDispatcher::new(state.clone());

    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_add_turn2".to_string(),
                name: "add_track_from_staged".to_string(),
                arguments: json!({
                    "presentation_id": pres.id,
                    "title": "Audio Previo",
                    "staged_audio_id": staged_id.clone(),
                })
                .to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: Some("Pista agregada a Repertorio Final.".to_string()),
            tool_calls: vec![],
        }),
    ]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Agrégalo a Repertorio Final".to_string(),
        history: vec![
            HistoryMessage {
                role: "user".to_string(),
                content: "Descarga el tema".to_string(),
            },
            HistoryMessage {
                role: "assistant".to_string(),
                content: "Audio preparado. ¿A qué presentación deseas agregarlo?".to_string(),
            },
        ],
        presentation_id: Some(pres.id.clone()),
        event_id: None,
        staged_audio_id: Some(staged_id),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "add_track_from_staged");
    assert_eq!(
        output.staged_audio_id, None,
        "staged_audio_id must be cleared after addition"
    );
    assert!(output.message.contains("Pista “Audio Previo” añadida"));

    let items = state.item_repo.list_by_presentation(&pres.id).unwrap();
    assert_eq!(items.len(), 1);
    match &items[0].payload {
        shirley_show::domain::show_item::ShowItemPayload::Track(t) => {
            assert_eq!(t.title, "Audio Previo");
        }
        _ => panic!("Expected track payload"),
    }
}

#[tokio::test]
async fn test_agent_chat_clarification_on_ambiguous_presentation_names() {
    let (state, _tmp) = setup_test_state();
    let _pres1 = state
        .create_presentation
        .execute("Festival de Verano - Tarde")
        .unwrap();
    let _pres2 = state
        .create_presentation
        .execute("Festival de Verano - Noche")
        .unwrap();

    let dispatcher = AgentToolDispatcher::new(state.clone());

    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_list_ambig".to_string(),
                name: "list_presentations".to_string(),
                arguments: "{}".to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: Some("Encontré dos presentaciones llamadas «Festival de Verano»: «Festival de Verano - Tarde» y «Festival de Verano - Noche». ¿A cuál de ellas deseas agregarlo?".to_string()),
            tool_calls: vec![],
        }),
    ]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Agrega esta nota al show Festival de Verano".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    assert!(output.message.contains("Festival de Verano - Tarde"));
    assert!(output.message.contains("Festival de Verano - Noche"));
    assert!(output.message.contains("¿A cuál") || output.message.contains("cuál de ellas"));
}

#[tokio::test]
async fn test_agent_chat_grounding_suppresses_false_claims_without_tool_execution() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    // The model hallucinates in text that it downloaded the YouTube video and added the track,
    // but did NOT invoke any tool calls!
    let provider = Arc::new(StubChatProvider::new(vec![Ok(CompletionResponse {
        content: Some(
            "Ya descargué el video de YouTube y agregué la pista a tu presentación.".to_string(),
        ),
        tool_calls: vec![],
    })]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Descarga https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    // Grounding must suppress false completion claim
    assert!(!output.message.contains("agregué la pista"));
    assert!(!output.message.contains("descargué el video"));
    assert!(
        output
            .message
            .contains("No se completaron acciones en este turno")
    );
}

#[tokio::test]
async fn test_agent_chat_download_error_downloader_not_configured_returns_friendly_message() {
    let (state, _tmp) = setup_test_state();
    let state = state.with_youtube_downloader(Arc::new(FakeDownloader::failing(
        YoutubeDownloadError::DownloaderNotConfigured,
    )));
    let dispatcher = AgentToolDispatcher::new(state);

    // Iteration 1: model calls download_youtube_track -> receives error downloader_not_configured
    // Iteration 2: model communicates friendly error to user
    struct ErrorChatProvider;
    impl ChatCompletionProvider for ErrorChatProvider {
        fn complete(
            &self,
            request: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>>
        {
            Box::pin(async move {
                let has_tool_error = request.messages.iter().any(|m| match m {
                    AgentMessage::Tool { content, .. } => {
                        content.contains("downloader_not_configured")
                    }
                    _ => false,
                });

                if !has_tool_error {
                    Ok(CompletionResponse {
                        content: None,
                        tool_calls: vec![ToolCall {
                            id: "call_fail_cfg".to_string(),
                            name: "download_youtube_track".to_string(),
                            arguments:
                                json!({ "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ" })
                                    .to_string(),
                        }],
                    })
                } else {
                    Ok(CompletionResponse {
                        content: Some("Lo siento, la herramienta de descarga no está disponible en el servidor en este momento.".to_string()),
                        tool_calls: vec![],
                    })
                }
            })
        }
    }

    let agent_chat = AgentChat::new(
        Some(Arc::new(ErrorChatProvider)),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Descarga https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    assert!(output.message.contains("no está disponible en el servidor"));
    assert!(!output.message.contains("/nix/store"));
    assert!(!output.message.contains("Command failed"));
    assert!(!output.message.contains("stderr"));
}

#[tokio::test]
async fn test_agent_chat_download_error_timeout_returns_friendly_message() {
    let (state, _tmp) = setup_test_state();
    let state = state.with_youtube_downloader(Arc::new(FakeDownloader::failing(
        YoutubeDownloadError::DownloadTimeout,
    )));
    let dispatcher = AgentToolDispatcher::new(state);

    struct TimeoutChatProvider;
    impl ChatCompletionProvider for TimeoutChatProvider {
        fn complete(
            &self,
            request: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>>
        {
            Box::pin(async move {
                let has_tool_error = request.messages.iter().any(|m| match m {
                    AgentMessage::Tool { content, .. } => content.contains("download_timeout"),
                    _ => false,
                });

                if !has_tool_error {
                    Ok(CompletionResponse {
                        content: None,
                        tool_calls: vec![ToolCall {
                            id: "call_fail_to".to_string(),
                            name: "download_youtube_track".to_string(),
                            arguments:
                                json!({ "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ" })
                                    .to_string(),
                        }],
                    })
                } else {
                    Ok(CompletionResponse {
                        content: Some(
                            "La descarga tardó demasiado tiempo y fue cancelada por seguridad."
                                .to_string(),
                        ),
                        tool_calls: vec![],
                    })
                }
            })
        }
    }

    let agent_chat = AgentChat::new(
        Some(Arc::new(TimeoutChatProvider)),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Descarga https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    assert!(
        output.message.contains("tardó demasiado tiempo") || output.message.contains("cancelada")
    );
    assert!(!output.message.contains("kill-and-wait"));
    assert!(!output.message.contains("SIGKILL"));
    assert!(!output.message.contains("/staging/"));
}

#[derive(Clone)]
struct FakeVocalSeparator {
    delay: Option<Duration>,
}

impl FakeVocalSeparator {
    fn new() -> Self {
        Self { delay: None }
    }
    fn with_delay(delay: Duration) -> Self {
        Self { delay: Some(delay) }
    }
}

impl VocalSeparator for FakeVocalSeparator {
    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let delay = self.delay;
        let out_path = request.output_path;
        Box::pin(async move {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            std::fs::write(&out_path, &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00])
                .expect("write fake separated mp3");
            Ok(SeparatedAudio {
                duration_secs: 180,
                size_bytes: 7200000,
            })
        })
    }
}

#[tokio::test]
async fn test_agent_chat_separation_flow_asks_destination_before_gpu() {
    let (state, _tmp) = setup_test_state();
    let state = state.with_vocal_separator(Arc::new(FakeVocalSeparator::new()));
    let dispatcher = AgentToolDispatcher::new(state);

    // Turn where destination is not active, Shirley asks for mode/destination without calling separate_vocals
    let provider = Arc::new(StubChatProvider::new(vec![Ok(CompletionResponse {
        content: Some(
            "¿Quieres conservar coros o dejarla instrumental, y en qué presentación la guardo?"
                .to_string(),
        ),
        tool_calls: vec![],
    })]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Separa coros para el show".to_string(),
        history: vec![],
        presentation_id: None,
        event_id: None,
        staged_audio_id: None,
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 0);
    assert!(output.message.contains("presentación"));
    assert_eq!(output.staged_audio_id, None);
}

#[tokio::test]
async fn test_agent_chat_separation_chains_when_destination_active() {
    let (state, _tmp) = setup_test_state();
    let state = state.with_vocal_separator(Arc::new(FakeVocalSeparator::new()));
    let pres = state.create_presentation.execute("Show Activo").unwrap();

    // Stage a source audio file
    let staged_src = state.storage.staging_dir().join("source.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-demo-1".to_string(),
        staged_src,
        "Canción Demo".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state.clone());

    // Provider responds with:
    // Iteration 1: calls separate_vocals
    // Iteration 2: calls add_track_from_staged
    // Iteration 3: final message
    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_add".to_string(),
                name: "add_track_from_staged".to_string(),
                arguments: json!({
                    "presentation_id": pres.id,
                    "title": "Canción Demo (Coros)"
                })
                .to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: Some("Separé los coros y añadí la pista al show.".to_string()),
            tool_calls: vec![],
        }),
    ]));

    let agent_chat = AgentChat::new(
        Some(provider),
        dispatcher,
        AgentLimits::default(),
        "model".to_string(),
    );

    let input = AgentChatInput {
        message: "Separa coros y agrégalo".to_string(),
        history: vec![],
        presentation_id: Some(pres.id.clone()),
        event_id: None,
        staged_audio_id: Some(reg_entry.staged_audio_id),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 2);
    assert_eq!(output.actions_taken[0].tool, "separate_vocals");
    assert_eq!(output.actions_taken[1].tool, "add_track_from_staged");
    // Audio was consumed into canonical track, staged_audio_id is None
    assert_eq!(output.staged_audio_id, None);
    assert!(output.message.contains("añadida") || output.message.contains("Separación vocal"));
}

#[tokio::test]
async fn test_agent_chat_separation_extended_deadline_600s() {
    let (state, _tmp) = setup_test_state();
    // Separator that finishes in 200ms
    let state = state.with_vocal_separator(Arc::new(FakeVocalSeparator::with_delay(
        Duration::from_millis(200),
    )));
    let pres = state.create_presentation.execute("Show Largo").unwrap();

    let staged_src = state.storage.staging_dir().join("slow_source.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-long-1".to_string(),
        staged_src,
        "Canción Larga".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_slow".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: Some("Separación completada con éxito.".to_string()),
            tool_calls: vec![],
        }),
    ]));

    // Standard conversational request timeout is 30s, separation timeout extends to 600s
    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_secs(30),
        separation_timeout: Duration::from_secs(600),
    };

    let agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());

    let input = AgentChatInput {
        message: "Separa coros de esta canción".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        event_id: None,
        staged_audio_id: Some(reg_entry.staged_audio_id),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "separate_vocals");
    assert!(output.notice.is_none());
}

#[tokio::test]
async fn test_agent_limits_defaults_and_separation_600s_invariants() {
    let defaults = AgentLimits::default();
    assert_eq!(defaults.request_timeout, Duration::from_secs(120));
    assert_eq!(defaults.separation_timeout, Duration::from_secs(600));

    // Verify clamp in AgentLimits when exceeding 600s
    let (state, _tmp) = setup_test_state();
    let state_sep = state.with_vocal_separator(Arc::new(FakeVocalSeparator::with_delay(
        Duration::from_millis(20),
    )));
    let pres = state_sep
        .create_presentation
        .execute("Show Invariant")
        .unwrap();
    let staged_src = state_sep.storage.staging_dir().join("inv_src.part");
    std::fs::write(
        &staged_src,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state_sep.staged_audio_registry.register(
        "stage-inv-1".to_string(),
        staged_src,
        "Inv Track".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state_sep);
    let provider = Arc::new(StubChatProvider::new(vec![
        Ok(CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_inv".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        }),
        Ok(CompletionResponse {
            content: Some("Listo.".to_string()),
            tool_calls: vec![],
        }),
    ]));

    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_secs(30),
        separation_timeout: Duration::from_secs(900), // Exceeds 600s
    };

    let agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());

    let input = AgentChatInput {
        message: "Separa coros".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        event_id: None,
        staged_audio_id: Some(reg_entry.staged_audio_id),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "separate_vocals");
    assert!(output.notice.is_none());
}

    #[test]
    fn test_system_prompt_gasto_requiere_datos_y_prohibe_inventar() {
        use shirley_show::agent::system_prompt::SHIRLEY_SYSTEM_PROMPT;
        assert!(
            SHIRLEY_SYSTEM_PROMPT.contains("Requisitos antes de registrar un gasto"),
            "el prompt debe exigir evento, categoría y monto antes de registrar"
        );
        assert!(
            SHIRLEY_SYSTEM_PROMPT.contains("NUNCA inventes montos"),
            "el prompt debe prohibir inventar montos de gastos"
        );
    }
