use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use shirley_show::agent::tool_dispatcher::AgentToolDispatcher;
use shirley_show::domain::agent_message::{AgentMessage, CompletionResponse};
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::groq::error::ProviderError;
use shirley_show::ports::chat_completion_provider::{ChatCompletionProvider, CompletionRequest};
use shirley_show::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};
use shirley_show::state::AppState;
use shirley_show::use_cases::agent_chat::{AgentChat, AgentChatInput, AgentLimits};
use tempfile::TempDir;

#[derive(Clone)]
struct MockRemoteVocalSeparator {
    probe_online: bool,
    fail_wake_timeout: bool,
    signal_wake_wait: bool,
    delay: Option<Duration>,
}

impl MockRemoteVocalSeparator {
    fn new(probe_online: bool) -> Self {
        Self {
            probe_online,
            fail_wake_timeout: false,
            signal_wake_wait: !probe_online,
            delay: None,
        }
    }

    fn failing_wake_timeout() -> Self {
        Self {
            probe_online: true,
            fail_wake_timeout: true,
            signal_wake_wait: false,
            delay: None,
        }
    }

    fn with_delay(delay: Duration) -> Self {
        Self {
            probe_online: true,
            fail_wake_timeout: false,
            signal_wake_wait: false,
            delay: Some(delay),
        }
    }
}

impl VocalSeparator for MockRemoteVocalSeparator {
    fn probe(&self, _timeout: Duration) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let online = self.probe_online;
        Box::pin(async move { online })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let fail_wake_timeout = self.fail_wake_timeout;
        let signal_wake_wait = self.signal_wake_wait;
        let delay = self.delay;
        let out_path = request.output_path;
        Box::pin(async move {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            if signal_wake_wait {
                return Err(VocalSeparationError::WakeWait);
            }
            if fail_wake_timeout {
                return Err(VocalSeparationError::PcWakeTimeout);
            }
            std::fs::write(&out_path, [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00])
                .expect("write mock separated mp3");
            Ok(SeparatedAudio {
                duration_secs: 180,
                size_bytes: 7200000,
            })
        })
    }
}

struct SimpleMockChatProvider {
    responses: Vec<CompletionResponse>,
}

impl SimpleMockChatProvider {
    fn new(responses: Vec<CompletionResponse>) -> Self {
        Self { responses }
    }
}

impl ChatCompletionProvider for SimpleMockChatProvider {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CompletionResponse, ProviderError>> + Send>> {
        let mut resp_idx = 0;
        for msg in &request.messages {
            if matches!(msg, AgentMessage::Tool { .. }) {
                resp_idx += 1;
            }
        }
        let chosen = if resp_idx < self.responses.len() {
            self.responses[resp_idx].clone()
        } else {
            CompletionResponse {
                content: Some("Operación finalizada.".to_string()),
                tool_calls: vec![],
            }
        };
        Box::pin(async move { Ok(chosen) })
    }
}

fn setup_test_state() -> (AppState, TempDir) {
    let temp = tempfile::tempdir().expect("create temp dir");
    let pool = shirley_show::infra::sqlite::init_pool(":memory:").expect("init sqlite memory pool");
    shirley_show::infra::sqlite::migrate_pool(&pool).expect("migrate sqlite");

    let audio_dir = temp.path().join("audio");
    std::fs::create_dir_all(audio_dir.join("files")).unwrap();
    std::fs::create_dir_all(audio_dir.join("staging")).unwrap();
    let storage = Arc::new(LocalAudioStorage::new(audio_dir));

    let state = AppState::new_with_items(pool, storage);
    (state, temp)
}

#[tokio::test]
async fn test_agent_chat_remote_pc_offline_returns_wake_wait_notice_and_contractual_copy() {
    let (state, _tmp) = setup_test_state();
    let separator = Arc::new(MockRemoteVocalSeparator::new(false));
    let state = state.with_vocal_separator(separator);
    let pres = state.create_presentation.execute("Show Acústico").unwrap();

    let staged_src = state.storage.staging_dir().join("source_track.part");
    std::fs::write(
        &staged_src,
        [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-offline-1".to_string(),
        staged_src,
        "Canción Acústica".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(SimpleMockChatProvider::new(vec![CompletionResponse {
        content: None,
        tool_calls: vec![ToolCall {
            id: "call_sep_offline".to_string(),
            name: "separate_vocals".to_string(),
            arguments: json!({
                "mode": "chorus",
                "staged_audio_id": reg_entry.staged_audio_id
            })
            .to_string(),
        }],
    }]));

    let limits = AgentLimits::default();
    let mut agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());
    agent_chat = agent_chat.with_remote_vocal_enabled(true);

    let input = AgentChatInput {
        message: "Separa coros para el show".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        staged_audio_id: Some(reg_entry.staged_audio_id.clone()),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;

    assert!(
        output.notice.is_some(),
        "notice must be present on offline probe"
    );
    let notice = output.notice.unwrap();
    assert_eq!(notice.code, "wake_wait");
    assert!(notice.retryable);

    assert_eq!(
        output.staged_audio_id,
        Some(reg_entry.staged_audio_id),
        "staged_audio_id must be preserved across wake_wait"
    );

    let expected_wait_copy = "Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo.";
    let expected_idle_copy =
        "Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.";

    assert!(
        output.message.contains(expected_wait_copy),
        "Message must contain exact wait copy: «{}», got: «{}»",
        expected_wait_copy,
        output.message
    );
    assert!(
        output.message.contains(expected_idle_copy),
        "Message must contain idle shutdown copy: «{}», got: «{}»",
        expected_idle_copy,
        output.message
    );

    assert_eq!(
        output.actions_taken.len(),
        0,
        "No actions should be taken while offline"
    );
}

#[tokio::test]
async fn test_agent_chat_remote_pc_wake_timeout_returns_give_up_copy() {
    let (state, _tmp) = setup_test_state();
    let separator = Arc::new(MockRemoteVocalSeparator::failing_wake_timeout());
    let state = state.with_vocal_separator(separator);
    let pres = state.create_presentation.execute("Show Timeout").unwrap();

    let staged_src = state.storage.staging_dir().join("source_timeout.part");
    std::fs::write(
        &staged_src,
        [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-timeout-1".to_string(),
        staged_src,
        "Canción Timeout".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(SimpleMockChatProvider::new(vec![CompletionResponse {
        content: None,
        tool_calls: vec![ToolCall {
            id: "call_sep_timeout".to_string(),
            name: "separate_vocals".to_string(),
            arguments: json!({
                "mode": "chorus",
                "staged_audio_id": reg_entry.staged_audio_id
            })
            .to_string(),
        }],
    }]));

    let limits = AgentLimits::default();
    let mut agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());
    agent_chat = agent_chat.with_remote_vocal_enabled(true);

    let input = AgentChatInput {
        message: "Separa coros".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        staged_audio_id: Some(reg_entry.staged_audio_id.clone()),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;

    assert!(
        output.notice.is_some(),
        "notice must be present on wake timeout"
    );
    let notice = output.notice.unwrap();
    assert_eq!(notice.code, "pc_wake_timeout");
    assert!(notice.retryable);

    assert_eq!(
        output.staged_audio_id,
        Some(reg_entry.staged_audio_id),
        "staged_audio_id must be preserved on timeout"
    );

    let expected_give_up = "No pude conectarme con la PC de Joshua después de 10 minutos. Cuando esté encendida, vuelve a pedirme la separación.";
    assert!(
        output.message.contains(expected_give_up),
        "Message must contain exact give-up copy: «{}», got: «{}»",
        expected_give_up,
        output.message
    );
}

#[tokio::test]
async fn test_agent_chat_remote_separation_success_appends_idle_shutdown_notice() {
    let (state, _tmp) = setup_test_state();
    let separator = Arc::new(MockRemoteVocalSeparator::new(true));
    let state = state.with_vocal_separator(separator);
    let pres = state.create_presentation.execute("Show Éxito").unwrap();

    let staged_src = state.storage.staging_dir().join("source_success.part");
    std::fs::write(
        &staged_src,
        [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-success-1".to_string(),
        staged_src,
        "Canción Exitosa".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(SimpleMockChatProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_ok".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: Some("Separación lista.".to_string()),
            tool_calls: vec![],
        },
    ]));

    let limits = AgentLimits::default();
    let mut agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string());
    agent_chat = agent_chat.with_remote_vocal_enabled(true);

    let input = AgentChatInput {
        message: "Separa coros".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        staged_audio_id: Some(reg_entry.staged_audio_id),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;

    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "separate_vocals");
    assert!(output.notice.is_none());

    let expected_idle_copy =
        "Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo.";
    assert!(
        output.message.contains(expected_idle_copy),
        "Successful remote separation message must contain idle shutdown notice: «{}», got: «{}»",
        expected_idle_copy,
        output.message
    );
}

#[tokio::test]
async fn test_agent_chat_composed_deadline_1500s_not_clamped_to_600s() {
    let (state, _tmp) = setup_test_state();
    let separator = Arc::new(MockRemoteVocalSeparator::with_delay(Duration::from_millis(
        50,
    )));
    let state = state.with_vocal_separator(separator);
    let pres = state.create_presentation.execute("Show 1500s").unwrap();

    let staged_src = state.storage.staging_dir().join("source_1500.part");
    std::fs::write(
        &staged_src,
        [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-1500-1".to_string(),
        staged_src,
        "Canción 1500s".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(SimpleMockChatProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_sep_1500".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: Some("Listo.".to_string()),
            tool_calls: vec![],
        },
    ]));

    let limits = AgentLimits {
        max_iterations: 5,
        request_timeout: Duration::from_secs(30),
        separation_timeout: Duration::from_secs(1500),
    };

    let agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string())
        .with_remote_vocal_enabled(true);

    assert_eq!(
        agent_chat.limits.separation_timeout,
        Duration::from_secs(1500)
    );

    let input = AgentChatInput {
        message: "Separa coros".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        staged_audio_id: Some(reg_entry.staged_audio_id),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;
    assert_eq!(output.actions_taken.len(), 1);
    assert_eq!(output.actions_taken[0].tool, "separate_vocals");
    assert!(output.notice.is_none());
}

#[derive(Clone)]
struct DynamicProbeSeparator {
    is_online: Arc<std::sync::atomic::AtomicBool>,
}

impl VocalSeparator for DynamicProbeSeparator {
    fn probe(&self, _timeout: Duration) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let online = self.is_online.load(std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move { online })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let is_online = self.is_online.clone();
        let out_path = request.output_path;
        Box::pin(async move {
            if !is_online.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(VocalSeparationError::WakeWait);
            }
            std::fs::write(&out_path, [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00])
                .expect("write mock separated mp3");
            Ok(SeparatedAudio {
                duration_secs: 180,
                size_bytes: 7200000,
            })
        })
    }
}

#[tokio::test]
async fn test_agent_chat_remote_ux_multi_turn_offline_then_boot_success() {
    let (state, _tmp) = setup_test_state();
    let is_online = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let separator = Arc::new(DynamicProbeSeparator {
        is_online: is_online.clone(),
    });
    let state = state.with_vocal_separator(separator);
    let pres = state
        .create_presentation
        .execute("Show Multiturno")
        .unwrap();

    let staged_src = state.storage.staging_dir().join("source_multi.part");
    std::fs::write(
        &staged_src,
        [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-multi-1".to_string(),
        staged_src,
        "Canción Multiturno".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    // Turn 1: PC is offline -> returns wake_wait notice
    let provider_turn1 = Arc::new(SimpleMockChatProvider::new(vec![CompletionResponse {
        content: None,
        tool_calls: vec![ToolCall {
            id: "call_t1".to_string(),
            name: "separate_vocals".to_string(),
            arguments: json!({
                "mode": "chorus",
                "staged_audio_id": reg_entry.staged_audio_id
            })
            .to_string(),
        }],
    }]));

    let limits = AgentLimits::default();
    let agent_chat_t1 = AgentChat::new(
        Some(provider_turn1),
        dispatcher.clone(),
        limits.clone(),
        "model".to_string(),
    )
    .with_remote_vocal_enabled(true);

    let input_t1 = AgentChatInput {
        message: "Separa coros para el show".to_string(),
        history: vec![],
        presentation_id: Some(pres.id.clone()),
        staged_audio_id: Some(reg_entry.staged_audio_id.clone()),
        staged_audio: None,
        staged_filename: None,
    };

    let output_t1 = agent_chat_t1.execute(input_t1).await;

    // Verify Turn 1 produces wake_wait notice and preserves staged audio
    assert_eq!(
        output_t1.notice.as_ref().map(|n| n.code.as_str()),
        Some("wake_wait")
    );
    assert_eq!(
        output_t1.staged_audio_id,
        Some(reg_entry.staged_audio_id.clone())
    );
    assert!(
        output_t1
            .message
            .contains("Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda")
    );
    assert_eq!(output_t1.actions_taken.len(), 0);

    // User turns on Joshua's PC
    is_online.store(true, std::sync::atomic::Ordering::SeqCst);

    // Turn 2: Follow-up turn after PC boot
    let provider_turn2 = Arc::new(SimpleMockChatProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_t2".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "chorus",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: Some("Listo, separé las voces.".to_string()),
            tool_calls: vec![],
        },
    ]));

    let agent_chat_t2 = AgentChat::new(
        Some(provider_turn2),
        dispatcher,
        limits,
        "model".to_string(),
    )
    .with_remote_vocal_enabled(true);

    let input_t2 = AgentChatInput {
        message: "Ya encendí la PC, continúa".to_string(),
        history: vec![
            shirley_show::domain::agent_message::HistoryMessage {
                role: "user".to_string(),
                content: "Separa coros para el show".to_string(),
            },
            shirley_show::domain::agent_message::HistoryMessage {
                role: "assistant".to_string(),
                content: output_t1.message,
            },
        ],
        presentation_id: Some(pres.id),
        staged_audio_id: output_t1.staged_audio_id,
        staged_audio: None,
        staged_filename: None,
    };

    let output_t2 = agent_chat_t2.execute(input_t2).await;

    // Turn 2 completes successfully: notice is cleared, action recorded, and idle shutdown notice appended
    assert!(
        output_t2.notice.is_none(),
        "notice must be cleared after successful turn"
    );
    assert_eq!(output_t2.actions_taken.len(), 1);
    assert_eq!(output_t2.actions_taken[0].tool, "separate_vocals");
    assert!(
        output_t2.message.contains("Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo."),
        "Turn 2 success must append idle shutdown copy, got: «{}»",
        output_t2.message
    );
}

#[derive(Clone)]
struct PcBootsMidWaitSeparator {
    waited_and_booted: Arc<std::sync::atomic::AtomicBool>,
}

impl VocalSeparator for PcBootsMidWaitSeparator {
    fn probe(&self, _timeout: Duration) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        // Initial probe returns false (PC currently off)
        Box::pin(async move { false })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        let waited_and_booted = self.waited_and_booted.clone();
        let out_path = request.output_path;
        Box::pin(async move {
            // Simulates backoff wait where Joshua's PC boots at simulated minute 3
            tokio::time::sleep(Duration::from_millis(50)).await;
            waited_and_booted.store(true, std::sync::atomic::Ordering::SeqCst);
            std::fs::write(&out_path, [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00])
                .expect("write mock separated mp3");
            Ok(SeparatedAudio {
                duration_secs: 180,
                size_bytes: 7200000,
            })
        })
    }
}

#[tokio::test]
async fn test_agent_chat_remote_pc_boots_mid_wait_same_turn_completes_separation() {
    let (state, _tmp) = setup_test_state();
    let waited_and_booted = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let separator = Arc::new(PcBootsMidWaitSeparator {
        waited_and_booted: waited_and_booted.clone(),
    });
    let state = state.with_vocal_separator(separator);
    let pres = state
        .create_presentation
        .execute("Show Espera Real")
        .unwrap();

    let staged_src = state.storage.staging_dir().join("source_boot_mid.part");
    std::fs::write(
        &staged_src,
        [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let reg_entry = state.staged_audio_registry.register(
        "stage-boot-mid-1".to_string(),
        staged_src,
        "Canción Espera Real".to_string(),
        180,
        7200000,
    );

    let dispatcher = AgentToolDispatcher::new(state);

    let provider = Arc::new(SimpleMockChatProvider::new(vec![
        CompletionResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: "call_boot_mid".to_string(),
                name: "separate_vocals".to_string(),
                arguments: json!({
                    "mode": "instrumental",
                    "staged_audio_id": reg_entry.staged_audio_id
                })
                .to_string(),
            }],
        },
        CompletionResponse {
            content: Some("Separación instrumental lista en el mismo turno.".to_string()),
            tool_calls: vec![],
        },
    ]));

    let limits = AgentLimits::default();
    let agent_chat = AgentChat::new(Some(provider), dispatcher, limits, "model".to_string())
        .with_remote_vocal_enabled(true);

    let input = AgentChatInput {
        message: "Separa instrumental para el show".to_string(),
        history: vec![],
        presentation_id: Some(pres.id),
        staged_audio_id: Some(reg_entry.staged_audio_id.clone()),
        staged_audio: None,
        staged_filename: None,
    };

    let output = agent_chat.execute(input).await;

    // The SAME turn must complete the separation when PC boots mid-wait:
    // 1. Must NOT abort early with wake_wait notice
    assert!(
        output.notice.is_none(),
        "Expected no error/wait notice on same-turn completion, got: {:?}",
        output.notice
    );
    // 2. Action must be recorded with success
    assert_eq!(
        output.actions_taken.len(),
        1,
        "Must execute separate_vocals in same turn"
    );
    assert_eq!(output.actions_taken[0].tool, "separate_vocals");
    assert_eq!(output.actions_taken[0].status, "success");
    // 3. Staged audio ID must be set to the resulting separated audio
    assert!(output.staged_audio_id.is_some());
    assert_ne!(
        output.staged_audio_id.as_ref(),
        Some(&reg_entry.staged_audio_id)
    );
    // 4. Separator must have performed the wait and completed
    assert!(waited_and_booted.load(std::sync::atomic::Ordering::SeqCst));
}
