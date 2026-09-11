use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;

use shirley_show::agent::tool_dispatcher::AgentToolDispatcher;
use shirley_show::agent::tool_schemas::get_tool_definitions;
use shirley_show::agent::turn_context::{StagedAudioHandle, TurnToolContext};
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::infra::filesystem::{LocalAudioStorage, StagingFileGuard};
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::state::AppState;

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
fn test_tool_schemas_strictly_six_tools_with_exact_signatures() {
    let tools = get_tool_definitions();
    assert_eq!(tools.len(), 6, "Must expose exactly 6 tools");

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"create_presentation"));
    assert!(names.contains(&"list_presentations"));
    assert!(names.contains(&"get_presentation"));
    assert!(names.contains(&"add_lyrics"));
    assert!(names.contains(&"add_note"));
    assert!(names.contains(&"add_track_from_staged"));

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

    let result = dispatcher.dispatch(&destructive_call, &mut context);
    assert_eq!(result["error"], "unknown_tool");

    let unknown_call = ToolCall {
        id: "call_sql_1".to_string(),
        name: "execute_sql".to_string(),
        arguments: json!({ "query": "DROP TABLE presentations;" }).to_string(),
    };

    let result2 = dispatcher.dispatch(&unknown_call, &mut context);
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
        }).to_string(),
    };
    let result = dispatcher.dispatch(&bad_call, &mut context);
    assert_eq!(result["error"], "unauthorized_presentation_id");
    assert!(context.actions_taken.is_empty());

    // 2. Calling add_note with authorized ID should succeed
    let good_call = ToolCall {
        id: "call_good_id".to_string(),
        name: "add_note".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "text": "Nota autorizada"
        }).to_string(),
    };
    let result_good = dispatcher.dispatch(&good_call, &mut context);
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
        }).to_string(),
    };
    let res1 = dispatcher.dispatch(&call1, &mut context);
    assert_eq!(res1["kind"], "lyrics");
    assert_eq!(res1["position"], 0);
    assert_eq!(context.actions_taken.len(), 1);

    // Duplicate call with SAME call_id
    let res_dup_id = dispatcher.dispatch(&call1, &mut context);
    assert_eq!(res_dup_id["warning"], "duplicate_call_ignored");
    assert_eq!(context.actions_taken.len(), 1, "Must not append duplicate action");

    // Duplicate call with DIFFERENT call_id but EQUIVALENT arguments (whitespace/key order invariant)
    let call2_equiv = ToolCall {
        id: "call_lyr_2".to_string(),
        name: "add_lyrics".to_string(),
        arguments: format!(
            "{{\"text\":\"Coro: Alabanza\",\"presentation_id\":\"{}\"}}",
            pres.id
        ),
    };
    let res_dup_sig = dispatcher.dispatch(&call2_equiv, &mut context);
    assert_eq!(res_dup_sig["warning"], "duplicate_call_ignored");
    assert_eq!(context.actions_taken.len(), 1);

    // Read tool deduplication / caching
    let list_call1 = ToolCall {
        id: "call_list_1".to_string(),
        name: "list_presentations".to_string(),
        arguments: "{}".to_string(),
    };
    let list_res1 = dispatcher.dispatch(&list_call1, &mut context);
    assert!(list_res1.is_array());

    let list_call2 = ToolCall {
        id: "call_list_2".to_string(),
        name: "list_presentations".to_string(),
        arguments: "{}".to_string(),
    };
    let list_res2 = dispatcher.dispatch(&list_call2, &mut context);
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
    let create_res = dispatcher.dispatch(&create_call, &mut context);
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
        }).to_string(),
    };
    let note_res = dispatcher.dispatch(&note_call, &mut context);
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
        }).to_string(),
    };
    let lyrics_res = dispatcher.dispatch(&lyrics_call, &mut context);
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
    std::fs::write(&stage_path, &[0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00]).unwrap();
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
        }).to_string(),
    };
    let res = dispatcher.dispatch(&track_call, &mut context);
    assert_eq!(res["kind"], "track");
    assert_eq!(res["title"], "Pista 1 Guitarra");
    assert_eq!(res["position"], 0);

    // Staged audio should now be Consumed
    assert!(matches!(context.staged_audio, Some(StagedAudioHandle::Consumed)));

    // 2. Calling add_track_from_staged again in same turn should fail with audio_already_consumed
    let track_call_2 = ToolCall {
        id: "call_trk_2".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Segunda Pista"
        }).to_string(),
    };
    let res2 = dispatcher.dispatch(&track_call_2, &mut context);
    assert_eq!(res2["error"], "audio_already_consumed");

    // 3. Calling add_track_from_staged without any staged audio
    let mut empty_context = TurnToolContext::new(Some(pres.id.clone()), None);
    let track_call_no_audio = ToolCall {
        id: "call_trk_3".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({
            "presentation_id": pres.id,
            "title": "Pista Fantasma"
        }).to_string(),
    };
    let res3 = dispatcher.dispatch(&track_call_no_audio, &mut empty_context);
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
    let res1 = dispatcher.dispatch(&empty_name, &mut context);
    assert_eq!(res1["error"], "validation_error");

    // Empty text in add_lyrics
    let empty_lyrics = ToolCall {
        id: "c2".to_string(),
        name: "add_lyrics".to_string(),
        arguments: json!({ "presentation_id": pres.id, "text": " \n \t " }).to_string(),
    };
    let res2 = dispatcher.dispatch(&empty_lyrics, &mut context);
    assert_eq!(res2["error"], "validation_error");

    // Empty text in add_note
    let empty_note = ToolCall {
        id: "c3".to_string(),
        name: "add_note".to_string(),
        arguments: json!({ "presentation_id": pres.id, "text": "" }).to_string(),
    };
    let res3 = dispatcher.dispatch(&empty_note, &mut context);
    assert_eq!(res3["error"], "validation_error");

    // Empty title in add_track_from_staged
    let empty_track = ToolCall {
        id: "c4".to_string(),
        name: "add_track_from_staged".to_string(),
        arguments: json!({ "presentation_id": pres.id, "title": "   " }).to_string(),
    };
    let res4 = dispatcher.dispatch(&empty_track, &mut context);
    assert_eq!(res4["error"], "validation_error");
}
