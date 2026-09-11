use std::sync::Arc;
use tempfile::TempDir;

use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::domain::move_direction::MoveDirection;
use shirley_show::domain::presentation::Presentation;
use shirley_show::domain::show_item_kind::ShowItemKind;
use shirley_show::error::AppError;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use shirley_show::infra::sqlite::connect::{DbPool, init_pool};
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use shirley_show::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use shirley_show::ports::cleanup_queue::CleanupQueue;
use shirley_show::ports::presentation_writer::PresentationWriter;
use shirley_show::use_cases::add_lyrics::AddLyrics;
use shirley_show::use_cases::add_note::AddNote;
use shirley_show::use_cases::delete_item::DeleteItem;
use shirley_show::use_cases::open_track_audio::OpenTrackAudio;
use shirley_show::use_cases::reorder_item::ReorderItem;
use shirley_show::use_cases::update_show_item::UpdateShowItem;
use shirley_show::use_cases::upload_track::UploadTrack;

struct TestContext {
    pres_repo: Arc<SqlitePresentationRepository>,
    item_repo: Arc<SqliteShowItemRepository>,
    storage: Arc<LocalAudioStorage>,
    cleanup_repo: Arc<SqliteCleanupRepository>,
    _pool: DbPool,
    _dir: TempDir,
}

fn setup() -> TestContext {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(dir.path().join("audio")));
    let pres_repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));
    let item_repo = Arc::new(SqliteShowItemRepository::new(pool.clone()));
    let cleanup_repo = Arc::new(SqliteCleanupRepository::new(pool.clone()));

    TestContext {
        pres_repo,
        item_repo,
        storage,
        cleanup_repo,
        _pool: pool,
        _dir: dir,
    }
}

#[test]
fn test_add_lyrics_and_note_use_cases() {
    let ctx = setup();
    let pres = Presentation::new("Concert").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let add_lyrics = AddLyrics::new(ctx.item_repo.clone());
    let add_note = AddNote::new(ctx.item_repo.clone());

    // Valid lyrics
    let lyrics_item = add_lyrics.execute(&pres.id, "  Line 1\nLine 2  ").unwrap();
    assert_eq!(lyrics_item.kind(), ShowItemKind::Lyrics);
    assert_eq!(lyrics_item.position, 0);

    // Empty lyrics rejected
    assert!(add_lyrics.execute(&pres.id, "   ").is_err());

    // Valid note
    let note_item = add_note.execute(&pres.id, "  Intro solo  ").unwrap();
    assert_eq!(note_item.kind(), ShowItemKind::Note);
    assert_eq!(note_item.position, 1);

    // Empty note rejected
    assert!(add_note.execute(&pres.id, "").is_err());
}

#[test]
fn test_upload_track_success_and_compensation_on_failure() {
    let ctx = setup();
    let pres = Presentation::new("Concert").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let upload_track = UploadTrack::new(ctx.item_repo.clone(), ctx.storage.clone());

    // 1. Success case
    let (stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio binary content").unwrap();

    let track_item = upload_track
        .execute(&pres.id, "Guitar Solo", &stage_id, AudioFormat::Mp3, 20)
        .unwrap();

    assert_eq!(track_item.kind(), ShowItemKind::Track);
    assert_eq!(track_item.position, 0);

    // Verify audio file exists in files directory
    let audio_key = match &track_item.payload {
        shirley_show::domain::show_item::ShowItemPayload::Track(t) => &t.audio_key,
        _ => panic!("Expected track payload"),
    };
    assert!(ctx.storage.resolve_path_str(audio_key).unwrap().exists());

    // 2. Compensation case: DB append fails because presentation does not exist
    let (stage_id2, stage_path2) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path2, b"audio binary content 2").unwrap();

    let err = upload_track.execute(
        "00000000-0000-0000-0000-000000000000",
        "Failed Track",
        &stage_id2,
        AudioFormat::Mp3,
        22,
    );
    assert!(err.is_err());

    // Staged file must not exist and promoted file must have been purged by compensation!
    assert!(stage_path2.exists());
    let entries = std::fs::read_dir(ctx.storage.files_dir()).unwrap();
    // Only the first valid audio file should exist in files_dir
    let files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    assert_eq!(files.len(), 1);

    // 3. Compensation case: Empty title rejected
    let (stage_id3, stage_path3) = ctx.storage.create_staging_file().unwrap();
    let err3 = upload_track.execute(&pres.id, "   ", &stage_id3, AudioFormat::Mp3, 50);
    assert!(err3.is_err());
    assert!(stage_path3.exists());
    let entries_after = std::fs::read_dir(ctx.storage.files_dir()).unwrap();
    let files_after: Vec<_> = entries_after.filter_map(|e| e.ok()).collect();
    assert_eq!(
        files_after.len(),
        1,
        "Promoted file must be cleaned up on validation failure"
    );
}

#[test]
fn test_reorder_item_use_case() {
    let ctx = setup();
    let pres = Presentation::new("Concert").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let add_note = AddNote::new(ctx.item_repo.clone());
    let i0 = add_note.execute(&pres.id, "First").unwrap();
    let i1 = add_note.execute(&pres.id, "Second").unwrap();

    let reorder = ReorderItem::new(ctx.item_repo.clone());

    // Swap i1 Up -> i1 is pos 0, i0 is pos 1
    let items = reorder
        .execute(&pres.id, &i1.id, MoveDirection::Up)
        .unwrap();
    assert_eq!(items[0].id, i1.id);
    assert_eq!(items[0].position, 0);
    assert_eq!(items[1].id, i0.id);
    assert_eq!(items[1].position, 1);

    // Swap boundary -> moving i1 Up again fails
    assert!(
        reorder
            .execute(&pres.id, &i1.id, MoveDirection::Up)
            .is_err()
    );
}

#[test]
fn test_delete_item_use_case_cleans_file() {
    let ctx = setup();
    let pres = Presentation::new("Concert").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let upload_track = UploadTrack::new(ctx.item_repo.clone(), ctx.storage.clone());
    let (stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio to delete").unwrap();

    let track_item = upload_track
        .execute(&pres.id, "Track To Delete", &stage_id, AudioFormat::Mp3, 15)
        .unwrap();

    let audio_key = match &track_item.payload {
        shirley_show::domain::show_item::ShowItemPayload::Track(t) => t.audio_key.clone(),
        _ => panic!("Expected track payload"),
    };
    let resolved_path = ctx.storage.resolve_path_str(&audio_key).unwrap();
    assert!(resolved_path.exists());

    let delete_item = DeleteItem::new(
        ctx.item_repo.clone(),
        ctx.storage.clone(),
        ctx.cleanup_repo.clone(),
    );

    let del_output = delete_item.execute(&pres.id, &track_item.id).unwrap();
    assert_eq!(del_output.deleted_id, track_item.id);
    assert_eq!(del_output.items.len(), 0);
    assert_eq!(del_output.cleanup_pending, false);

    // File should be deleted from disk
    assert!(!resolved_path.exists());
    // Pending queue should be empty
    assert!(ctx.cleanup_repo.get_pending().unwrap().is_empty());
}

#[test]
fn test_open_track_audio_use_case() {
    let ctx = setup();
    let pres = Presentation::new("Concert").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let upload_track = UploadTrack::new(ctx.item_repo.clone(), ctx.storage.clone());
    let (stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio bytes 12345").unwrap();

    let track_item = upload_track
        .execute(&pres.id, "Stream Track", &stage_id, AudioFormat::Mp3, 17)
        .unwrap();

    let open_audio = OpenTrackAudio::new(ctx.item_repo.clone(), ctx.storage.clone());

    // 1. Success case
    let opened = open_audio.execute(&track_item.id).unwrap();
    assert_eq!(opened.media_type, "audio/mpeg");
    assert_eq!(opened.byte_size, 17);
    assert!(opened.path.exists());

    // 2. Track not found
    let not_found_err = open_audio.execute("non-existent-track-id");
    assert!(not_found_err.is_err());

    // 3. Opening non-track item (lyrics)
    let add_lyrics = AddLyrics::new(ctx.item_repo.clone());
    let lyrics_item = add_lyrics.execute(&pres.id, "Some text").unwrap();
    let err_not_track = open_audio.execute(&lyrics_item.id);
    assert!(err_not_track.is_err());

    // 4. Audio file missing on disk
    let audio_key = match &track_item.payload {
        shirley_show::domain::show_item::ShowItemPayload::Track(t) => t.audio_key.clone(),
        _ => panic!("Expected track payload"),
    };
    let file_path = ctx.storage.resolve_path_str(&audio_key).unwrap();
    std::fs::remove_file(&file_path).unwrap();

    let missing_file_err = open_audio.execute(&track_item.id);
    assert!(missing_file_err.is_err());
}

#[test]
fn test_update_show_item_use_case() {
    let ctx = setup();
    let pres = Presentation::new("Show to Edit").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let upload_track = UploadTrack::new(ctx.item_repo.clone(), ctx.storage.clone());
    let (stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio sample").unwrap();
    let track_item = upload_track
        .execute(&pres.id, "Original Title", &stage_id, AudioFormat::Mp3, 12)
        .unwrap();

    let add_lyrics = AddLyrics::new(ctx.item_repo.clone());
    let lyrics_item = add_lyrics.execute(&pres.id, "Original Lyrics").unwrap();

    let update_uc = UpdateShowItem::new(ctx.item_repo.clone(), ctx.item_repo.clone());

    // 1. Valid track update
    let updated_track = update_uc
        .execute(&pres.id, &track_item.id, Some("Renamed Song".into()), None)
        .unwrap();
    match &updated_track.payload {
        shirley_show::domain::show_item::ShowItemPayload::Track(t) => {
            assert_eq!(t.title, "Renamed Song");
        }
        _ => panic!("Expected track payload"),
    }

    // 2. Reject incompatible field on track (body instead of title)
    let err_incompatible = update_uc
        .execute(&pres.id, &track_item.id, None, Some("Some text".into()))
        .unwrap_err();
    assert!(matches!(
        err_incompatible,
        AppError::BadRequest {
            code: "incompatible_field",
            ..
        }
    ));

    // 3. Valid lyrics update
    let updated_lyrics = update_uc
        .execute(&pres.id, &lyrics_item.id, None, Some("New Lyrics".into()))
        .unwrap();
    match &updated_lyrics.payload {
        shirley_show::domain::show_item::ShowItemPayload::Lyrics(l) => {
            assert_eq!(l.body, "New Lyrics");
        }
        _ => panic!("Expected lyrics payload"),
    }

    // 4. Reject incompatible field on lyrics (title instead of body)
    let err_incompatible_lyrics = update_uc
        .execute(&pres.id, &lyrics_item.id, Some("New Title".into()), None)
        .unwrap_err();
    assert!(matches!(
        err_incompatible_lyrics,
        AppError::BadRequest {
            code: "incompatible_field",
            ..
        }
    ));

    // 5. Reject both fields provided
    let err_both = update_uc
        .execute(
            &pres.id,
            &track_item.id,
            Some("Title".into()),
            Some("Body".into()),
        )
        .unwrap_err();
    assert!(matches!(
        err_both,
        AppError::BadRequest {
            code: "invalid_update_fields",
            ..
        }
    ));

    // 6. Reject empty fields provided
    let err_none = update_uc
        .execute(&pres.id, &track_item.id, None, None)
        .unwrap_err();
    assert!(matches!(
        err_none,
        AppError::BadRequest {
            code: "invalid_update_fields",
            ..
        }
    ));

    // 7. Reject empty string after trim
    let err_empty = update_uc
        .execute(&pres.id, &track_item.id, Some("   ".into()), None)
        .unwrap_err();
    assert!(matches!(
        err_empty,
        AppError::BadRequest {
            code: "empty_update_value",
            ..
        }
    ));

    // 8. 404 on missing item
    let err_not_found = update_uc
        .execute(&pres.id, "missing-item-id", Some("Title".into()), None)
        .unwrap_err();
    assert!(matches!(err_not_found, AppError::NotFound { .. }));
}

#[test]
fn test_reorder_item_target_position_use_case() {
    let ctx = setup();
    let pres = Presentation::new("Reorder UC Test").unwrap();
    ctx.pres_repo.create(&pres).unwrap();

    let add_note = AddNote::new(ctx.item_repo.clone());
    let i0 = add_note.execute(&pres.id, "Zero").unwrap();
    let i1 = add_note.execute(&pres.id, "One").unwrap();
    let i2 = add_note.execute(&pres.id, "Two").unwrap();

    let reorder = ReorderItem::new(ctx.item_repo.clone());

    // Move i2 to position 0 -> [i2, i0, i1]
    let items = reorder
        .execute_target_position(&pres.id, &i2.id, 0)
        .unwrap();
    assert_eq!(items[0].id, i2.id);
    assert_eq!(items[0].position, 0);
    assert_eq!(items[1].id, i0.id);
    assert_eq!(items[1].position, 1);
    assert_eq!(items[2].id, i1.id);
    assert_eq!(items[2].position, 2);
}
