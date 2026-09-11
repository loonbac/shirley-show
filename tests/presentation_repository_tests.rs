use shirley_show::domain::Presentation;
use shirley_show::error::AppError;
use shirley_show::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::presentation_reader::PresentationReader;
use shirley_show::ports::presentation_writer::PresentationWriter;
use tempfile::NamedTempFile;

fn setup_repo() -> (SqlitePresentationRepository, tempfile::NamedTempFile) {
    let temp_file = NamedTempFile::new().expect("temp file");
    let path = temp_file.path().to_str().expect("valid path");
    let pool = init_pool(path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");
    (SqlitePresentationRepository::new(pool), temp_file)
}

#[test]
fn test_create_and_list_presentations() {
    let (repo, _f) = setup_repo();

    let list_empty = repo.list().expect("list empty");
    assert!(list_empty.is_empty());

    let p1 = Presentation::new("Viernes 24 Acústico").expect("p1");
    let p2 = Presentation::new("Ensayo General").expect("p2");

    repo.create(&p1).expect("create p1");
    repo.create(&p2).expect("create p2");

    let list = repo.list().expect("list presentations");
    assert_eq!(list.len(), 2);
    let ids: Vec<&str> = list.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&p1.id.as_str()));
    assert!(ids.contains(&p2.id.as_str()));
}

#[test]
fn test_duplicate_titles_yield_distinct_records() {
    let (repo, _f) = setup_repo();

    let p1 = Presentation::new("Mismo Nombre").expect("p1");
    let p2 = Presentation::new("Mismo Nombre").expect("p2");
    assert_ne!(p1.id, p2.id);

    repo.create(&p1).expect("create p1");
    repo.create(&p2).expect("create p2");

    let list = repo.list().expect("list");
    assert_eq!(list.len(), 2);
    assert_ne!(list[0].id, list[1].id);
}

#[test]
fn test_get_presentation() {
    let (repo, _f) = setup_repo();
    let p = Presentation::new("Show Acústico").expect("presentation");
    repo.create(&p).expect("create");

    let found = repo.get(&p.id).expect("get").expect("must exist");
    assert_eq!(found.id, p.id);
    assert_eq!(found.title, "Show Acústico");

    let missing = repo.get("non-existent-id").expect("get non-existent");
    assert!(missing.is_none());
}

#[test]
fn test_rename_presentation() {
    let (repo, _f) = setup_repo();
    let p = Presentation::new("Setlist Borrador").expect("presentation");
    repo.create(&p).expect("create");

    let updated = repo
        .rename(&p.id, "Setlist Final", "2026-03-22T00:00:00Z")
        .expect("rename");
    assert_eq!(updated.id, p.id);
    assert_eq!(updated.title, "Setlist Final");
    assert_eq!(updated.updated_at, "2026-03-22T00:00:00Z");

    let fetched = repo.get(&p.id).expect("get").expect("found");
    assert_eq!(fetched.title, "Setlist Final");
}

#[test]
fn test_rename_non_existent_returns_not_found() {
    let (repo, _f) = setup_repo();
    let err = repo.rename("missing-id", "New Title", "2026-03-22T00:00:00Z").unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[test]
fn test_delete_non_existent_returns_not_found() {
    let (repo, _f) = setup_repo();
    let err = repo.delete("missing-id").unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[test]
fn test_delete_presentation_and_cascade_with_audio_queue() {
    let (repo, f) = setup_repo();
    let p = Presentation::new("Para Borrar").expect("presentation");
    repo.create(&p).expect("create");

    // Insert show items directly via rusqlite to test cascade & pending deletions
    {
        let conn = rusqlite::Connection::open(f.path()).expect("open raw conn");
        conn.execute(
            "INSERT INTO show_items (id, presentation_id, position, kind, body, created_at)
             VALUES ('item-1', ?1, 0, 'lyrics', 'La la la', '2026-03-22T00:00:00Z');",
            [&p.id],
        ).expect("insert lyrics item");

        conn.execute(
            "INSERT INTO show_items (id, presentation_id, position, kind, title, audio_key, media_type, byte_size, created_at)
             VALUES ('item-2', ?1, 1, 'track', 'Intro', 'audio-key-123.mp3', 'audio/mpeg', 1024, '2026-03-22T00:00:00Z');",
            [&p.id],
        ).expect("insert track item");
    }

    let del_res = repo.delete(&p.id).expect("delete");
    assert_eq!(del_res.deleted_id, p.id);

    let fetched = repo.get(&p.id).expect("get");
    assert!(fetched.is_none());

    // Verify cascade deleted show_items
    let conn = rusqlite::Connection::open(f.path()).expect("open raw conn");
    let items_count: i32 = conn
        .query_row("SELECT count(*) FROM show_items WHERE presentation_id = ?1;", [&p.id], |r| r.get(0))
        .expect("count show_items");
    assert_eq!(items_count, 0, "Show items should be deleted via cascade");

    // Verify audio_key was inserted into pending_audio_deletions
    let pending_count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pending_audio_deletions WHERE audio_key = 'audio-key-123.mp3';",
            [],
            |r| r.get(0),
        )
        .expect("count pending deletions");
    assert_eq!(pending_count, 1, "Track audio_key should be queued in pending_audio_deletions");
}

#[test]
fn test_delete_presentation_physical_audio_cleanup() {
    let (repo, f) = setup_repo();
    let audio_dir = tempfile::tempdir().expect("audio dir");
    let storage = std::sync::Arc::new(shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage::new(
        audio_dir.path().join("audio"),
    ));
    let repo = repo.with_storage(storage.clone());

    let p = Presentation::new("Live Album").expect("presentation");
    repo.create(&p).expect("create");

    // Stage and promote a real audio file
    let (stage_id, stage_path) = storage.create_staging_file().expect("stage file");
    std::fs::write(&stage_path, &[0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB])
        .expect("write staged audio");
    let key = storage
        .promote_staged_file(&stage_id, shirley_show::domain::audio_format::AudioFormat::Mp3)
        .expect("promote audio");

    let file_path = storage.resolve_path(&key);
    assert!(file_path.exists(), "Audio file must exist on disk");

    // Insert track item into DB
    {
        let conn = rusqlite::Connection::open(f.path()).expect("open raw conn");
        conn.execute(
            "INSERT INTO show_items (id, presentation_id, position, kind, title, audio_key, media_type, byte_size, created_at)
             VALUES ('item-track-1', ?1, 0, 'track', 'Encore', ?2, 'audio/mpeg', 12, '2026-03-22T00:00:00Z');",
            [&p.id, key.as_str()],
        ).expect("insert track item");
    }

    let del_res = repo.delete(&p.id).expect("delete presentation");
    assert_eq!(del_res.deleted_id, p.id);
    assert_eq!(del_res.cleanup_pending, false);

    // Assert physical file removed
    assert!(!file_path.exists(), "Audio file must be physically deleted post-commit");

    // Assert DB cascade
    let conn = rusqlite::Connection::open(f.path()).expect("open raw conn");
    let items_count: i32 = conn
        .query_row("SELECT count(*) FROM show_items WHERE presentation_id = ?1;", [&p.id], |r| r.get(0))
        .expect("count show_items");
    assert_eq!(items_count, 0, "Show items must be cascade deleted");

    // Assert pending queue is completed (deleted)
    let pending_count: i32 = conn
        .query_row(
            "SELECT count(*) FROM pending_audio_deletions WHERE audio_key = ?1;",
            [key.as_str()],
            |r| r.get(0),
        )
        .expect("count pending deletions");
    assert_eq!(pending_count, 0, "Pending queue must have 0 entries after successful cleanup");
}
