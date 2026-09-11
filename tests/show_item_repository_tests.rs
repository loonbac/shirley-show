use shirley_show::domain::move_direction::MoveDirection;
use shirley_show::domain::presentation::Presentation;
use shirley_show::domain::show_item::{LyricsPayload, NotePayload, ShowItemPayload, TrackPayload};
use shirley_show::domain::show_item_kind::ShowItemKind;
use shirley_show::infra::sqlite::connect::{init_pool, DbPool};
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use shirley_show::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use shirley_show::ports::presentation_writer::PresentationWriter;
use shirley_show::ports::show_item_reader::ShowItemReader;
use shirley_show::ports::show_item_writer::ShowItemWriter;
use tempfile::TempDir;

fn setup_repo() -> (SqliteShowItemRepository, SqlitePresentationRepository, DbPool, TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("test.sqlite3");
    let path = db_path.to_str().expect("valid path");
    let pool = init_pool(path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let item_repo = SqliteShowItemRepository::new(pool.clone());
    let pres_repo = SqlitePresentationRepository::new(pool.clone());
    (item_repo, pres_repo, pool, dir)
}

#[test]
fn test_append_items_and_list_ordered() {
    let (item_repo, pres_repo, _pool, _dir) = setup_repo();

    let pres = Presentation::new("Test Show").unwrap();
    pres_repo.create(&pres).unwrap();

    // Append 1: Lyrics
    let p1 = ShowItemPayload::Lyrics(LyricsPayload::new("Verse 1").unwrap());
    let item1 = item_repo.append(&pres.id, p1).unwrap();
    assert_eq!(item1.position, 0);
    assert_eq!(item1.kind(), ShowItemKind::Lyrics);

    // Append 2: Note
    let p2 = ShowItemPayload::Note(NotePayload::new("Check tuning").unwrap());
    let item2 = item_repo.append(&pres.id, p2).unwrap();
    assert_eq!(item2.position, 1);
    assert_eq!(item2.kind(), ShowItemKind::Note);

    // Append 3: Track
    let p3 = ShowItemPayload::Track(TrackPayload::new("Backing track", "audio-abc.mp3", "audio/mpeg", 2048).unwrap());
    let item3 = item_repo.append(&pres.id, p3).unwrap();
    assert_eq!(item3.position, 2);
    assert_eq!(item3.kind(), ShowItemKind::Track);

    // List items
    let list = item_repo.list_by_presentation(&pres.id).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, item1.id);
    assert_eq!(list[0].position, 0);
    assert_eq!(list[1].id, item2.id);
    assert_eq!(list[1].position, 1);
    assert_eq!(list[2].id, item3.id);
    assert_eq!(list[2].position, 2);
}

#[test]
fn test_append_non_existent_presentation_fails() {
    let (item_repo, _pres_repo, _pool, _dir) = setup_repo();
    let p = ShowItemPayload::Lyrics(LyricsPayload::new("Verse").unwrap());
    let result = item_repo.append("00000000-0000-0000-0000-000000000000", p);
    assert!(result.is_err());
}

#[test]
fn test_get_item_by_presentation_and_by_id() {
    let (item_repo, pres_repo, _pool, _dir) = setup_repo();

    let pres = Presentation::new("Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let p = ShowItemPayload::Note(NotePayload::new("Note 1").unwrap());
    let created = item_repo.append(&pres.id, p).unwrap();

    // Get with matching presentation
    let found = item_repo.get(&pres.id, &created.id).unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, created.id);

    // Get with mismatched presentation
    let not_found_pres = item_repo.get("other-id", &created.id).unwrap();
    assert!(not_found_pres.is_none());

    // Get by id directly
    let found_by_id = item_repo.get_by_id(&created.id).unwrap();
    assert!(found_by_id.is_some());
    assert_eq!(found_by_id.unwrap().id, created.id);
}

#[test]
fn test_swap_atomic_and_boundary_guards() {
    let (item_repo, pres_repo, _pool, _dir) = setup_repo();

    let pres = Presentation::new("Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let item0 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("Item 0").unwrap())).unwrap();
    let item1 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("Item 1").unwrap())).unwrap();
    let item2 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("Item 2").unwrap())).unwrap();

    // Boundary: moving item0 Up must fail with conflict
    let err_up = item_repo.swap(&pres.id, &item0.id, MoveDirection::Up);
    assert!(err_up.is_err());

    // Boundary: moving item2 Down must fail with conflict
    let err_down = item_repo.swap(&pres.id, &item2.id, MoveDirection::Down);
    assert!(err_down.is_err());

    // Swap item1 Up: item1 becomes pos 0, item0 becomes pos 1
    let swapped_items = item_repo.swap(&pres.id, &item1.id, MoveDirection::Up).unwrap();
    assert_eq!(swapped_items.len(), 3);
    assert_eq!(swapped_items[0].id, item1.id);
    assert_eq!(swapped_items[0].position, 0);
    assert_eq!(swapped_items[1].id, item0.id);
    assert_eq!(swapped_items[1].position, 1);
    assert_eq!(swapped_items[2].id, item2.id);
    assert_eq!(swapped_items[2].position, 2);

    // Swap item1 Down: reverts back to original
    let swapped_back = item_repo.swap(&pres.id, &item1.id, MoveDirection::Down).unwrap();
    assert_eq!(swapped_back[0].id, item0.id);
    assert_eq!(swapped_back[0].position, 0);
    assert_eq!(swapped_back[1].id, item1.id);
    assert_eq!(swapped_back[1].position, 1);
    assert_eq!(swapped_back[2].id, item2.id);
    assert_eq!(swapped_back[2].position, 2);
}

#[test]
fn test_single_item_presentation_swap_boundary_checks() {
    let (item_repo, pres_repo, _pool, _dir) = setup_repo();

    let pres = Presentation::new("Single Item Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let only_item = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("Only").unwrap())).unwrap();

    // Both up and down must fail on a 1-item list
    let err_up = item_repo.swap(&pres.id, &only_item.id, MoveDirection::Up);
    assert!(err_up.is_err());
    let err_down = item_repo.swap(&pres.id, &only_item.id, MoveDirection::Down);
    assert!(err_down.is_err());
}

#[test]
fn test_rapid_successive_swaps_maintain_contiguous_indices() {
    let (item_repo, pres_repo, _pool, _dir) = setup_repo();

    let pres = Presentation::new("Rapid Swap Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let _it0 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("0").unwrap())).unwrap();
    let it1 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("1").unwrap())).unwrap();
    let it2 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("2").unwrap())).unwrap();
    let it3 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("3").unwrap())).unwrap();

    // Perform rapid successive swaps
    let swaps = [
        (&it1.id, MoveDirection::Up),   // it1 from 1 to 0
        (&it2.id, MoveDirection::Up),   // it2 from 2 to 1
        (&it3.id, MoveDirection::Up),   // it3 from 3 to 2
        (&it1.id, MoveDirection::Down), // it1 from 0 to 1
        (&it2.id, MoveDirection::Down), // it2 from 1 to 2
    ];

    for (id, dir) in swaps {
        let items = item_repo.swap(&pres.id, id, dir).unwrap();
        assert_eq!(items.len(), 4);
        for (idx, item) in items.iter().enumerate() {
            assert_eq!(item.position, idx as i64, "Positions must be strictly contiguous 0..N-1");
        }
    }
}

#[test]
fn test_delete_item_recompacts_and_enqueues_audio() {
    let (item_repo, pres_repo, pool, _dir) = setup_repo();

    let pres = Presentation::new("Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let item0 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("Item 0").unwrap())).unwrap();
    let item1 = item_repo.append(
        &pres.id,
        ShowItemPayload::Track(TrackPayload::new("Track 1", "audio-key-999.mp3", "audio/mpeg", 5000).unwrap()),
    ).unwrap();
    let item2 = item_repo.append(&pres.id, ShowItemPayload::Lyrics(LyricsPayload::new("Lyrics 2").unwrap())).unwrap();

    // Delete item1 (middle track)
    let del_result = item_repo.delete(&pres.id, &item1.id).unwrap();
    assert_eq!(del_result.deleted_id, item1.id);
    assert_eq!(del_result.audio_key, Some("audio-key-999.mp3".to_string()));

    // Remaining items recompacted to 0 and 1
    assert_eq!(del_result.items.len(), 2);
    assert_eq!(del_result.items[0].id, item0.id);
    assert_eq!(del_result.items[0].position, 0);
    assert_eq!(del_result.items[1].id, item2.id);
    assert_eq!(del_result.items[1].position, 1);

    // Verify audio_key is in pending_audio_deletions table
    let conn = pool.get().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pending_audio_deletions WHERE audio_key = 'audio-key-999.mp3';",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(count, 1);

    // Verify deleting non-existent item returns error
    assert!(item_repo.delete(&pres.id, "non-existent").is_err());
}

#[test]
fn test_delete_first_and_last_items_recompaction() {
    let (item_repo, pres_repo, _pool, _dir) = setup_repo();

    let pres = Presentation::new("Edge Delete Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let i0 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("0").unwrap())).unwrap();
    let i1 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("1").unwrap())).unwrap();
    let i2 = item_repo.append(&pres.id, ShowItemPayload::Note(NotePayload::new("2").unwrap())).unwrap();

    // Delete first item (i0)
    let res0 = item_repo.delete(&pres.id, &i0.id).unwrap();
    assert_eq!(res0.items.len(), 2);
    assert_eq!(res0.items[0].id, i1.id);
    assert_eq!(res0.items[0].position, 0);
    assert_eq!(res0.items[1].id, i2.id);
    assert_eq!(res0.items[1].position, 1);

    // Delete last item (now i2 at pos 1)
    let res1 = item_repo.delete(&pres.id, &i2.id).unwrap();
    assert_eq!(res1.items.len(), 1);
    assert_eq!(res1.items[0].id, i1.id);
    assert_eq!(res1.items[0].position, 0);

    // Delete remaining single item (i1 at pos 0)
    let res2 = item_repo.delete(&pres.id, &i1.id).unwrap();
    assert_eq!(res2.items.len(), 0);
}
