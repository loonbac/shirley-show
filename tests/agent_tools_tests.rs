use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;

use shirley_show::agent::tool_dispatcher::AgentToolDispatcher;
use shirley_show::agent::tool_schemas::get_tool_definitions;
use shirley_show::agent::turn_context::{StagedAudioHandle, TurnToolContext};
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::domain::youtube_url::YouTubeUrl;
use shirley_show::infra::filesystem::{LocalAudioStorage, StagingFileGuard};
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};
use shirley_show::ports::youtube_downloader::{
    DownloadedYoutubeAudio, YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};
use shirley_show::state::AppState;

#[derive(Clone)]
struct FakeVocalSeparator {
    fail_with_error: Arc<std::sync::Mutex<Option<VocalSeparationError>>>,
}

impl FakeVocalSeparator {
    fn new() -> Self {
        Self {
            fail_with_error: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl VocalSeparator for FakeVocalSeparator {
    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        if let Some(err) = self.fail_with_error.lock().unwrap().clone() {
            return Box::pin(async move { Err(err) });
        }
        let out_path = request.output_path;
        Box::pin(async move {
            std::fs::write(&out_path, &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00])
                .expect("write fake separated mp3");
            Ok(SeparatedAudio {
                duration_secs: 180,
                size_bytes: 7200000,
            })
        })
    }
}

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
            std::fs::write(&part_path, b"dummy audio mp3 content").expect("write fake mp3 part");

            Ok(DownloadedYoutubeAudio {
                title,
                duration_secs,
                size_bytes,
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

#[test]
fn test_tool_schemas_strictly_eight_tools_with_exact_signatures() {
    let tools = get_tool_definitions();
    assert_eq!(tools.len(), 8, "Must expose exactly 8 tools");

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"create_presentation"));
    assert!(names.contains(&"list_presentations"));
    assert!(names.contains(&"get_presentation"));
    assert!(names.contains(&"add_lyrics"));
    assert!(names.contains(&"add_note"));
    assert!(names.contains(&"add_track_from_staged"));
    assert!(names.contains(&"download_youtube_track"));
    assert!(names.contains(&"separate_vocals"));

    for tool in &tools {
        let params = &tool.parameters;
        assert_eq!(params["type"], "object");
        assert_eq!(params["additionalProperties"], false);

        match tool.name.as_str() {
            "create_presentation" => {
                let required = params["required"].as_array().unwrap();
                assert!(required.contains(&json!("name")));
            }
            "list_presentations" => {
                assert!(params["properties"].as_object().unwrap().is_empty());
            }
            "get_presentation" => {
                let required = params["required"].as_array().unwrap();
                assert!(required.contains(&json!("presentation_id")));
            }
            "add_lyrics" | "add_note" => {
                let required = params["required"].as_array().unwrap();
                assert!(required.contains(&json!("presentation_id")));
                assert!(required.contains(&json!("text")));
            }
            "add_track_from_staged" => {
                let required = params["required"].as_array().unwrap();
                assert!(required.contains(&json!("presentation_id")));
                assert!(required.contains(&json!("title")));
                assert!(!required.contains(&json!("staged_audio_id")));
                let props = params["properties"].as_object().unwrap();
                assert!(props.contains_key("presentation_id"));
                assert!(props.contains_key("title"));
                assert!(props.contains_key("staged_audio_id"));
            }
            "download_youtube_track" => {
                let required = params["required"].as_array().unwrap();
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("url")));
                let props = params["properties"].as_object().unwrap();
                assert!(props.contains_key("url"));
                assert!(props.contains_key("title"));
            }
            "separate_vocals" => {
                let required = params["required"].as_array().unwrap();
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("mode")));
                let props = params["properties"].as_object().unwrap();
                assert!(props.contains_key("mode"));
                assert!(props.contains_key("staged_audio_id"));
                assert!(props.contains_key("track_id"));
                assert!(props.contains_key("youtube_url"));
                let mode_enum = props["mode"]["enum"].as_array().unwrap();
                assert!(mode_enum.contains(&json!("chorus")));
                assert!(mode_enum.contains(&json!("instrumental")));
            }
            _ => panic!("Unexpected tool name: {}", tool.name),
        }
    }
}

#[tokio::test]
async fn test_dispatcher_rejects_unknown_or_destructive_tools() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default();

    let destructive_call = ToolCall {
        id: "call_del_1".to_string(),
        name: "delete_presentation".to_string(),
        arguments: json!({ "presentation_id": "pres-1" }).to_string(),
    };

    let result = dispatcher.dispatch(&destructive_call, &mut context).await;
    assert_eq!(result["error"], "unknown_tool");

    let unknown_call = ToolCall {
        id: "call_sql_1".to_string(),
        name: "execute_sql".to_string(),
        arguments: json!({ "query": "DROP TABLE presentations;" }).to_string(),
    };

    let result2 = dispatcher.dispatch(&unknown_call, &mut context).await;
    assert_eq!(result2["error"], "unknown_tool");
    assert!(context.actions_taken.is_empty());
}

#[tokio::test]
async fn test_dispatcher_enforces_presentation_id_integrity() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Oficial").unwrap();

    let dispatcher = AgentToolDispatcher::new(state);
    // Context only knows about pres.id
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    // 1. Calling add_note with fabricated ID should fail
    let bad_call = ToolCall {
        id: "call_bad_id".to_string(),
        name: "add_note".to_string(),
        arguments: json!({
            "presentation_id": "pres-invented-999",
            "text": "Nota fantasma"
        })
        .to_string(),
    };
    let result = dispatcher.dispatch(&bad_call, &mut context).await;
    assert_eq!(result["error"], "unauthorized_presentation_id");
    assert!(context.actions_taken.is_empty());

    // 2. Calling add_note with authorized ID should succeed
    let good_call = ToolCall {
        id: "call_good_id".to_string(),
        name: "add_note".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "text": "Nota autorizada"
        })
        .to_string(),
    };
    let result_good = dispatcher.dispatch(&good_call, &mut context).await;
    assert_eq!(result_good["kind"], "note");
    assert_eq!(result_good["presentation_id"], pres.id);
    assert_eq!(context.actions_taken.len(), 1);
    assert_eq!(context.actions_taken[0].tool, "add_note");
}

#[tokio::test]
async fn test_dispatcher_deduplication_mutation_and_read_cache() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Ensayo").unwrap();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    // Call add_lyrics
    let call1 = ToolCall {
        id: "call_lyr_1".to_string(),
        name: "add_lyrics".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "text": "Coro: Alabanza"
        })
        .to_string(),
    };
    let res1 = dispatcher.dispatch(&call1, &mut context).await;
    assert_eq!(res1["kind"], "lyrics");
    assert_eq!(res1["position"], 0);
    assert_eq!(context.actions_taken.len(), 1);

    // Duplicate call with SAME call_id
    let res_dup_id = dispatcher.dispatch(&call1, &mut context).await;
    assert_eq!(res_dup_id["warning"], "duplicate_call_ignored");
    assert_eq!(
        context.actions_taken.len(),
        1,
        "Must not append duplicate action"
    );

    // Duplicate call with DIFFERENT call_id but EQUIVALENT arguments (whitespace/key order invariant)
    let call2_equiv = ToolCall {
        id: "call_lyr_2".to_string(),
        name: "add_lyrics".to_string(),
        arguments: format!(
            "{{\"text\":\"Coro: Alabanza\",\"presentation_id\":\"{}\"}}",
            pres.id
        ),
    };
    let res_dup_sig = dispatcher.dispatch(&call2_equiv, &mut context).await;
    assert_eq!(res_dup_sig["warning"], "duplicate_call_ignored");
    assert_eq!(context.actions_taken.len(), 1);

    // Read tool deduplication / caching
    let list_call1 = ToolCall {
        id: "call_list_1".to_string(),
        name: "list_presentations".to_string(),
        arguments: "{}".to_string(),
    };
    let list_res1 = dispatcher.dispatch(&list_call1, &mut context).await;
    assert!(list_res1.is_array());

    let list_call2 = ToolCall {
        id: "call_list_2".to_string(),
        name: "list_presentations".to_string(),
        arguments: "{}".to_string(),
    };
    let list_res2 = dispatcher.dispatch(&list_call2, &mut context).await;
    assert_eq!(list_res1, list_res2);
}

#[tokio::test]
async fn test_dispatcher_chained_presentation_creation_and_item_addition() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default(); // initially no known presentations

    // 1. Create presentation
    let create_call = ToolCall {
        id: "call_create_1".to_string(),
        name: "create_presentation".to_string(),
        arguments: json!({ "name": "Show Nuevo" }).to_string(),
    };
    let create_res = dispatcher.dispatch(&create_call, &mut context).await;
    let new_id = create_res["id"].as_str().unwrap().to_string();
    assert_eq!(create_res["title"], "Show Nuevo");
    assert!(context.known_presentation_ids.contains(&new_id));
    assert_eq!(context.actions_taken.len(), 1);

    // 2. Add note to newly created presentation in the same turn
    let note_call = ToolCall {
        id: "call_note_1".to_string(),
        name: "add_note".to_string(),
        arguments: json!({
            "presentation_id": new_id,
            "text": "Comenzar en Mi menor"
        })
        .to_string(),
    };
    let note_res = dispatcher.dispatch(&note_call, &mut context).await;
    assert_eq!(note_res["kind"], "note");
    assert_eq!(note_res["position"], 0);
    assert_eq!(context.actions_taken.len(), 2);

    // 3. Add lyrics
    let lyrics_call = ToolCall {
        id: "call_lyr_1".to_string(),
        name: "add_lyrics".to_string(),
        arguments: json!({
            "presentation_id": new_id,
            "text": "Estrofa 1..."
        })
        .to_string(),
    };
    let lyrics_res = dispatcher.dispatch(&lyrics_call, &mut context).await;
    assert_eq!(lyrics_res["kind"], "lyrics");
    assert_eq!(lyrics_res["position"], 1);
    assert_eq!(context.actions_taken.len(), 3);
}

#[tokio::test]
async fn test_dispatcher_staged_audio_track_lifecycle() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Musical").unwrap();

    // Create a real staged file
    let (stage_id, stage_path) = state.storage.create_staging_file().unwrap();
    // Write valid MP3 dummy bytes (with sync frame)
    std::fs::write(
        &stage_path,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();
    let guard = StagingFileGuard::new(state.storage.clone(), stage_id.clone(), stage_path.clone());
    let staged_audio = StagedAudioHandle::Available {
        guard,
        format: AudioFormat::Mp3,
        byte_size: 8,
    };

    let dispatcher = AgentToolDispatcher::new(state.clone());
    let mut context = TurnToolContext::new(Some(pres.id.clone()), Some(staged_audio));

    // 1. add_track_from_staged should succeed and consume the audio
    let track_call = ToolCall {
        id: "call_trk_1".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Pista 1 Guitarra"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&track_call, &mut context).await;
    assert_eq!(res["kind"], "track");
    assert_eq!(res["title"], "Pista 1 Guitarra");
    assert_eq!(res["position"], 0);

    // Staged audio should now be Consumed
    assert!(matches!(
        context.staged_audio,
        Some(StagedAudioHandle::Consumed)
    ));

    // 2. Calling add_track_from_staged again in same turn should fail with audio_already_consumed
    let track_call_2 = ToolCall {
        id: "call_trk_2".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Segunda Pista"
        })
        .to_string(),
    };
    let res2 = dispatcher.dispatch(&track_call_2, &mut context).await;
    assert_eq!(res2["error"], "audio_already_consumed");

    // 3. Calling add_track_from_staged without any staged audio
    let mut empty_context = TurnToolContext::new(Some(pres.id.clone()), None);
    let track_call_no_audio = ToolCall {
        id: "call_trk_3".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Pista Fantasma"
        })
        .to_string(),
    };
    let res3 = dispatcher
        .dispatch(&track_call_no_audio, &mut empty_context)
        .await;
    assert_eq!(res3["error"], "no_audio_staged");
}

#[tokio::test]
async fn test_dispatcher_validation_empty_arguments() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Val").unwrap();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    // Empty name in create_presentation
    let empty_name = ToolCall {
        id: "c1".to_string(),
        name: "create_presentation".to_string(),
        arguments: json!({ "name": "   " }).to_string(),
    };
    let res1 = dispatcher.dispatch(&empty_name, &mut context).await;
    assert_eq!(res1["error"], "validation_error");

    // Empty text in add_lyrics
    let empty_lyrics = ToolCall {
        id: "c2".to_string(),
        name: "add_lyrics".to_string(),
        arguments: json!({ "presentation_id": pres.id, "text": " \n \t " }).to_string(),
    };
    let res2 = dispatcher.dispatch(&empty_lyrics, &mut context).await;
    assert_eq!(res2["error"], "validation_error");

    // Empty text in add_note
    let empty_note = ToolCall {
        id: "c3".to_string(),
        name: "add_note".to_string(),
        arguments: json!({ "presentation_id": pres.id, "text": "" }).to_string(),
    };
    let res3 = dispatcher.dispatch(&empty_note, &mut context).await;
    assert_eq!(res3["error"], "validation_error");

    // Empty title in add_track_from_staged
    let empty_track = ToolCall {
        id: "c4".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({ "presentation_id": pres.id, "title": "   " }).to_string(),
    };
    let res4 = dispatcher.dispatch(&empty_track, &mut context).await;
    assert_eq!(res4["error"], "validation_error");
}

#[tokio::test]
async fn test_dispatcher_rejects_unknown_fields_in_tool_arguments() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Val").unwrap();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default();

    let call_create = ToolCall {
        id: "call_extra_1".to_string(),
        name: "create_presentation".to_string(),
        arguments: json!({ "name": "Show Nuevo", "extra_prop": "malicious" }).to_string(),
    };
    let res = dispatcher.dispatch(&call_create, &mut context).await;
    assert_eq!(res["error"], "validation_error");
    assert_eq!(res["message"], "Parámetros de herramienta inválidos");

    let call_get = ToolCall {
        id: "call_extra_2".to_string(),
        name: "get_presentation".to_string(),
        arguments: json!({ "presentation_id": pres.id, "foo": "bar" }).to_string(),
    };
    let res = dispatcher.dispatch(&call_get, &mut context).await;
    assert_eq!(res["error"], "validation_error");
    assert_eq!(res["message"], "Parámetros de herramienta inválidos");

    let call_lyrics = ToolCall {
        id: "call_extra_3".to_string(),
        name: "add_lyrics".to_string(),
        arguments: json!({ "presentation_id": pres.id, "text": "Letra", "extra": 123 }).to_string(),
    };
    let res = dispatcher.dispatch(&call_lyrics, &mut context).await;
    assert_eq!(res["error"], "validation_error");
    assert_eq!(res["message"], "Parámetros de herramienta inválidos");

    let call_track = ToolCall {
        id: "call_extra_4".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({ "presentation_id": pres.id, "title": "Track", "bogus": true })
            .to_string(),
    };
    let res = dispatcher.dispatch(&call_track, &mut context).await;
    assert_eq!(res["error"], "validation_error");
    assert_eq!(res["message"], "Parámetros de herramienta inválidos");

    let call_yt = ToolCall {
        id: "call_extra_5".to_string(),
        name: "download_youtube_track".to_string(),
        arguments: json!({ "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ", "extra": 123 })
            .to_string(),
    };
    let res_yt = dispatcher.dispatch(&call_yt, &mut context).await;
    assert_eq!(res_yt["error"], "validation_error");
    assert_eq!(res_yt["message"], "Parámetros de herramienta inválidos");
}

#[tokio::test]
async fn test_dispatcher_download_youtube_track_success_and_deduplication() {
    let (state, _tmp) = setup_test_state();
    let state = state.with_youtube_downloader(Arc::new(FakeDownloader::new(
        "Zamba de mi Esperanza",
        180,
        4_500_000,
    )));
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default();

    let yt_call = ToolCall {
        id: "call_yt_1".to_string(),
        name: "download_youtube_track".to_string(),
        arguments: json!({
            "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "title": "Mi Zamba Personalizada"
        })
        .to_string(),
    };

    let res = dispatcher.dispatch(&yt_call, &mut context).await;
    assert!(res["staged_audio_id"].as_str().is_some());
    let staged_id = res["staged_audio_id"].as_str().unwrap().to_string();
    assert!(staged_id.starts_with("staged-"));
    assert_eq!(res["title"], "Mi Zamba Personalizada");
    assert_eq!(res["duration_secs"], 180);
    assert_eq!(res["size_bytes"], 4_500_000);

    // Context should now carry the staged_audio_id capability
    assert_eq!(context.staged_audio_id.as_deref(), Some(staged_id.as_str()));
    assert_eq!(context.actions_taken.len(), 1);
    assert_eq!(context.actions_taken[0].tool, "download_youtube_track");
    assert_eq!(context.actions_taken[0].presentation_id, None);
    assert_eq!(context.actions_taken[0].status, "success");

    // Check payload details in ToolAction
    let details = context.actions_taken[0].details.as_ref().unwrap();
    assert_eq!(details["status"], "success");
    assert_eq!(details["title"], "Mi Zamba Personalizada");
    assert_eq!(details["byte_size"], 4_500_000);
    assert_eq!(details["format"], "mp3");

    // Second call in same turn with same call ID MUST be deduplicated
    let dup_res = dispatcher.dispatch(&yt_call, &mut context).await;
    assert_eq!(dup_res["warning"], "duplicate_call_ignored");
    assert_eq!(context.actions_taken.len(), 1);

    // Second call with different ID but same arguments MUST also be deduplicated
    let yt_call_diff_id = ToolCall {
        id: "call_yt_2".to_string(),
        name: "download_youtube_track".to_string(),
        arguments: json!({
            "title": "Mi Zamba Personalizada",
            "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        })
        .to_string(),
    };
    let dup_res_sig = dispatcher.dispatch(&yt_call_diff_id, &mut context).await;
    assert_eq!(dup_res_sig["warning"], "duplicate_call_ignored");
    assert_eq!(context.actions_taken.len(), 1);
}

#[tokio::test]
async fn test_dispatcher_download_youtube_track_chained_with_add_track_from_staged() {
    let (state, _tmp) = setup_test_state();
    let pres = state
        .create_presentation
        .execute("Repertorio Peña")
        .unwrap();
    let state = state.with_youtube_downloader(Arc::new(FakeDownloader::new(
        "Chacarera del 55",
        150,
        3_800_000,
    )));
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    // 1. Download YouTube track
    let yt_call = ToolCall {
        id: "call_yt_1".to_string(),
        name: "download_youtube_track".to_string(),
        arguments: json!({
            "url": "https://youtu.be/dQw4w9WgXcQ"
        })
        .to_string(),
    };
    let yt_res = dispatcher.dispatch(&yt_call, &mut context).await;
    let staged_id = yt_res["staged_audio_id"].as_str().unwrap().to_string();
    assert_eq!(context.staged_audio_id.as_deref(), Some(staged_id.as_str()));
    assert_eq!(context.actions_taken.len(), 1);

    // 2. Chained add_track_from_staged without explicit staged_audio_id (inherited from turn capability)
    let add_call = ToolCall {
        id: "call_add_1".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Chacarera del 55"
        })
        .to_string(),
    };
    let add_res = dispatcher.dispatch(&add_call, &mut context).await;
    assert_eq!(add_res["kind"], "track");
    assert_eq!(add_res["title"], "Chacarera del 55");
    assert_eq!(add_res["position"], 0);

    // Staged audio ID should now be consumed / cleared from active context capability
    assert_eq!(context.staged_audio_id, None);
    assert_eq!(context.actions_taken.len(), 2);
    assert_eq!(context.actions_taken[0].tool, "download_youtube_track");
    assert_eq!(context.actions_taken[1].tool, "add_track_from_staged");
    assert_eq!(context.actions_taken[1].presentation_id, Some(pres.id));

    // Attempting to consume again should fail with audio_already_consumed
    let add_call_again = ToolCall {
        id: "call_add_2".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": context.actions_taken[1].presentation_id.as_ref().unwrap(),
            "title": "Otra Pista",
            "staged_audio_id": staged_id
        })
        .to_string(),
    };
    let add_again_res = dispatcher.dispatch(&add_call_again, &mut context).await;
    assert_eq!(add_again_res["error"], "audio_already_consumed");
}

#[tokio::test]
async fn test_dispatcher_add_track_from_staged_with_explicit_staged_audio_id() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Domingo").unwrap();

    // Register an audio file directly in registry
    let (stage_id, stage_path) = state.storage.create_staging_file().unwrap();
    std::fs::write(
        &stage_path,
        &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00],
    )
    .unwrap();

    let descriptor = state.staged_audio_registry.register(
        stage_id,
        stage_path,
        "Tema Registrado".to_string(),
        210,
        2_500_000,
    );

    let dispatcher = AgentToolDispatcher::new(state);
    // Context initialized with the staged_audio_id capability from previous turn
    let mut context = TurnToolContext::with_staged_audio_id(
        Some(pres.id.clone()),
        None,
        Some(descriptor.staged_audio_id.clone()),
    );

    let add_call = ToolCall {
        id: "call_explicit_staged".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Tema Registrado",
            "staged_audio_id": descriptor.staged_audio_id
        })
        .to_string(),
    };

    let add_res = dispatcher.dispatch(&add_call, &mut context).await;
    assert_eq!(add_res["kind"], "track");
    assert_eq!(add_res["title"], "Tema Registrado");
    assert_eq!(context.staged_audio_id, None);
    assert_eq!(context.actions_taken.len(), 1);
}

#[tokio::test]
async fn test_dispatcher_add_track_from_staged_rejects_unauthorized_staged_audio_id() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Seguro").unwrap();
    let dispatcher = AgentToolDispatcher::new(state);
    // Context does NOT know this staged_audio_id
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    let bad_staged_call = ToolCall {
        id: "call_bad_staged".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Pista No Autorizada",
            "staged_audio_id": "staged-unauthorized-999"
        })
        .to_string(),
    };

    let res = dispatcher.dispatch(&bad_staged_call, &mut context).await;
    assert_eq!(res["error"], "staged_audio_not_found");
    assert!(context.actions_taken.is_empty());
}

#[tokio::test]
async fn test_dispatcher_separate_vocals_rejected_without_confirmed_destination() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    // Context with NO confirmed destination (None)
    let mut context = TurnToolContext::default();

    let call = ToolCall {
        id: "call_sep_no_dest".to_string(),
        name: "separate_vocals".to_string(),
        arguments: json!({
            "mode": "chorus",
            "youtube_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        })
        .to_string(),
    };

    let res = dispatcher.dispatch(&call, &mut context).await;
    assert_eq!(res["error"], "destination_required");
    assert!(context.actions_taken.is_empty());
}

#[tokio::test]
async fn test_dispatcher_separate_vocals_rejects_invalid_mode() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Destino").unwrap();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    let call = ToolCall {
        id: "call_sep_bad_mode".to_string(),
        name: "separate_vocals".to_string(),
        arguments: json!({
            "mode": "acapella",
            "youtube_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        })
        .to_string(),
    };

    let res = dispatcher.dispatch(&call, &mut context).await;
    assert_eq!(res["error"], "validation_error");
    assert!(res["message"].as_str().unwrap().contains("chorus"));
    assert!(context.actions_taken.is_empty());
}

#[tokio::test]
async fn test_dispatcher_separate_vocals_rejects_unauthorized_track_id() {
    let (state, _tmp) = setup_test_state();
    let pres = state.create_presentation.execute("Show Destino").unwrap();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::new(Some(pres.id.clone()), None);

    let call = ToolCall {
        id: "call_sep_unauth_track".to_string(),
        name: "separate_vocals".to_string(),
        arguments: json!({
            "mode": "chorus",
            "track_id": "track-unauthorized-999"
        })
        .to_string(),
    };

    let res = dispatcher.dispatch(&call, &mut context).await;
    assert_eq!(res["error"], "unauthorized_track_id");
    assert!(context.actions_taken.is_empty());
}

#[tokio::test]
async fn test_dispatcher_separate_vocals_success_and_chained_with_add_track_from_staged() {
    let (state, _tmp) = setup_test_state();
    let fake_separator = Arc::new(FakeVocalSeparator::new());
    let state = state.with_vocal_separator(fake_separator);

    let pres = state.create_presentation.execute("Show En Vivo").unwrap();

    // Stage a source audio in registry
    let staged_source_part = state.storage.staging_dir().join("src123.part");
    std::fs::write(&staged_source_part, &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00]).unwrap();
    let reg_entry = state
        .staged_audio_registry
        .register(
            "stage-src-1".to_string(),
            staged_source_part,
            "Canción Fuente".to_string(),
            180,
            5000000,
        );

    let dispatcher = AgentToolDispatcher::new(state.clone());
    let mut context = TurnToolContext::with_staged_audio_id(
        Some(pres.id.clone()),
        None,
        Some(reg_entry.staged_audio_id.clone()),
    );

    // 1. Dispatch separate_vocals
    let sep_call = ToolCall {
        id: "call_sep_1".to_string(),
        name: "separate_vocals".to_string(),
        arguments: json!({
            "mode": "chorus",
            "staged_audio_id": reg_entry.staged_audio_id
        })
        .to_string(),
    };

    let sep_res = dispatcher.dispatch(&sep_call, &mut context).await;
    assert!(sep_res.get("staged_audio_id").is_some());
    let new_staged_id = sep_res["staged_audio_id"].as_str().unwrap();
    assert_eq!(context.actions_taken.len(), 1);
    assert_eq!(context.actions_taken[0].tool, "separate_vocals");
    assert_eq!(context.staged_audio_id.as_deref(), Some(new_staged_id));

    // 2. Chain add_track_from_staged using the separated staged_audio_id
    let add_call = ToolCall {
        id: "call_add_chained".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Canción Fuente (Coros)"
        })
        .to_string(),
    };

    let add_res = dispatcher.dispatch(&add_call, &mut context).await;
    assert_eq!(add_res["kind"], "track");
    assert_eq!(context.actions_taken.len(), 2);
    assert_eq!(context.actions_taken[1].tool, "add_track_from_staged");
    assert_eq!(context.staged_audio_id, None);
}

#[tokio::test]
async fn test_dispatcher_separate_vocals_deduplication_ignored() {
    let (state, _tmp) = setup_test_state();
    let fake_separator = Arc::new(FakeVocalSeparator::new());
    let state = state.with_vocal_separator(fake_separator);
    let pres = state.create_presentation.execute("Show Dedup").unwrap();

    let staged_part = state.storage.staging_dir().join("dedup.part");
    std::fs::write(&staged_part, &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00]).unwrap();
    let reg_entry = state
        .staged_audio_registry
        .register(
            "stage-dedup-1".to_string(),
            staged_part,
            "Tema Dedup".to_string(),
            120,
            3000000,
        );

    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::with_staged_audio_id(
        Some(pres.id.clone()),
        None,
        Some(reg_entry.staged_audio_id.clone()),
    );

    let call = ToolCall {
        id: "call_sep_d1".to_string(),
        name: "separate_vocals".to_string(),
        arguments: json!({
            "mode": "chorus",
            "staged_audio_id": reg_entry.staged_audio_id
        })
        .to_string(),
    };

    let res1 = dispatcher.dispatch(&call, &mut context).await;
    assert!(res1.get("staged_audio_id").is_some());
    assert_eq!(context.actions_taken.len(), 1);

    // Identical duplicate call
    let call_dup = ToolCall {
        id: "call_sep_d2".to_string(),
        name: "separate_vocals".to_string(),
        arguments: json!({
            "mode": "chorus",
            "staged_audio_id": reg_entry.staged_audio_id
        })
        .to_string(),
    };

    let res2 = dispatcher.dispatch(&call_dup, &mut context).await;
    assert_eq!(res2["warning"], "duplicate_call_ignored");
    assert_eq!(context.actions_taken.len(), 1);
}
