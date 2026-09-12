use std::sync::Arc;
use tempfile::tempdir;

use shirley_show::domain::ticketing::{Money, TicketType};
use shirley_show::error::AppError;
use shirley_show::infra::filesystem::local_poster_storage::{ImageFormat, LocalPosterStorage};
use shirley_show::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::ticketing::{EventRepository, PosterStorage};
use shirley_show::use_cases::ticketing::events::{CreateEvent, CreateEventInput, DeleteEvent};
use shirley_show::use_cases::ticketing::posters::{GetEventPoster, ReplaceEventPoster};

const SAMPLE_JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
const SAMPLE_PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
const SAMPLE_WEBP: &[u8] = &[
    b'R', b'I', b'F', b'F', 0x20, 0x00, 0x00, 0x00, b'W', b'E', b'B', b'P', b'V', b'P', b'8',
];

#[test]
fn test_image_format_magic_bytes_detection_and_rejection() {
    assert_eq!(ImageFormat::detect(SAMPLE_JPEG).unwrap(), ImageFormat::Jpeg);
    assert_eq!(ImageFormat::detect(SAMPLE_PNG).unwrap(), ImageFormat::Png);
    assert_eq!(ImageFormat::detect(SAMPLE_WEBP).unwrap(), ImageFormat::Webp);

    // Reject ELF executable
    let elf_header = &[0x7F, 0x45, 0x4C, 0x46, 0x02, 0x01, 0x01, 0x00];
    let err = ImageFormat::detect(elf_header);
    assert!(matches!(err, Err(AppError::UnsupportedMediaType { .. })));

    // Reject PE executable (MZ)
    let pe_header = &[0x4D, 0x5A, 0x90, 0x00, 0x03, 0x00, 0x00, 0x00];
    let err = ImageFormat::detect(pe_header);
    assert!(matches!(err, Err(AppError::UnsupportedMediaType { .. })));

    // Reject random bytes / text
    let text = b"#!/bin/bash\necho hello";
    let err = ImageFormat::detect(text);
    assert!(matches!(err, Err(AppError::UnsupportedMediaType { .. })));

    // Reject empty bytes
    let err = ImageFormat::detect(&[]);
    assert!(matches!(err, Err(AppError::UnsupportedMediaType { .. })));
}

#[test]
fn test_poster_storage_10mib_limit_and_replacement_cleanup() {
    let temp_dir = tempdir().unwrap();
    let posters_path = temp_dir.path().join("posters");
    let storage = Arc::new(LocalPosterStorage::new(posters_path.clone()));

    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();
    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let create_event = CreateEvent::new(repo.clone());
    let replace_poster = ReplaceEventPoster::new(repo.clone(), storage.clone());
    let get_poster = GetEventPoster::new(repo.clone(), storage.clone());

    let event = create_event
        .execute(CreateEventInput {
            nombre: "Concierto Primavera".to_string(),
            fecha_hora: "2026-10-15T21:00:00Z".to_string(),
            lugar: "Teatro Municipal".to_string(),
            tipos_de_entrada: vec![
                TicketType::new("General", Money::from_cents(2500), 50).unwrap(),
            ],
        })
        .unwrap();

    // 1. Upload > 10 MiB must be rejected with PayloadTooLarge (413)
    let oversized = vec![0xFF; 10 * 1024 * 1024 + 1];
    let err = replace_poster.execute(&event.id, &oversized);
    assert!(matches!(err, Err(AppError::PayloadTooLarge { .. })));

    // 2. Upload valid JPEG poster
    let filename1 = replace_poster.execute(&event.id, SAMPLE_JPEG).unwrap();
    assert!(filename1.starts_with(&format!("{}_", event.id)));
    assert!(filename1.ends_with(".jpg"));

    let disk_file1 = posters_path.join(&filename1);
    assert!(disk_file1.exists());

    // Verify get_poster returns content and Jpeg format
    let (data, format) = get_poster.execute(&event.id).unwrap();
    assert_eq!(format, ImageFormat::Jpeg);
    assert_eq!(data, SAMPLE_JPEG);

    // 3. Replace with PNG poster -> old file on disk must be removed
    let filename2 = replace_poster.execute(&event.id, SAMPLE_PNG).unwrap();
    assert!(filename2.starts_with(&format!("{}_", event.id)));
    assert!(filename2.ends_with(".png"));

    let disk_file2 = posters_path.join(&filename2);
    assert!(disk_file2.exists());
    assert!(
        !disk_file1.exists(),
        "Previous poster file should have been deleted from disk"
    );

    let (data2, format2) = get_poster.execute(&event.id).unwrap();
    assert_eq!(format2, ImageFormat::Png);
    assert_eq!(data2, SAMPLE_PNG);
}

#[test]
fn test_event_deletion_cleans_up_poster_file() {
    let temp_dir = tempdir().unwrap();
    let posters_path = temp_dir.path().join("posters");
    let storage = Arc::new(LocalPosterStorage::new(posters_path.clone()));

    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();
    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let create_event = CreateEvent::new(repo.clone());
    let replace_poster = ReplaceEventPoster::new(repo.clone(), storage.clone());
    let delete_event = DeleteEvent::new(repo.clone(), storage.clone());

    let event = create_event
        .execute(CreateEventInput {
            nombre: "Evento a Borrar".to_string(),
            fecha_hora: "2026-12-01T20:00:00Z".to_string(),
            lugar: "Miraflores".to_string(),
            tipos_de_entrada: vec![
                TicketType::new("General", Money::from_cents(2000), 10).unwrap(),
            ],
        })
        .unwrap();

    let poster_file = replace_poster.execute(&event.id, SAMPLE_WEBP).unwrap();
    let disk_poster = posters_path.join(&poster_file);
    assert!(disk_poster.exists());

    // Delete event -> database record and poster on disk must be deleted
    delete_event.execute(&event.id).unwrap();

    assert!(
        !disk_poster.exists(),
        "Poster file must be deleted when event is deleted"
    );
    assert!(repo.get_event(&event.id).unwrap().is_none());
}

#[test]
fn test_poster_storage_path_traversal_resistance() {
    let temp_dir = tempdir().unwrap();
    let posters_path = temp_dir.path().join("posters");
    let storage = LocalPosterStorage::new(posters_path.clone());

    // Create a secret file outside posters dir
    let secret_file = temp_dir.path().join("secret.txt");
    std::fs::write(&secret_file, b"top-secret-data").unwrap();

    // Traversal read attempts must return None
    assert!(storage.get_poster("../secret.txt").unwrap().is_none());
    assert!(storage.get_poster("/etc/passwd").unwrap().is_none());
    assert!(storage.get_poster("..\\secret.txt").unwrap().is_none());
    assert!(storage.get_poster("foo/bar.jpg").unwrap().is_none());

    // Traversal delete attempts must return false and not delete the file
    assert!(!storage.delete_poster("../secret.txt").unwrap());
    assert!(
        secret_file.exists(),
        "Traversal delete must not touch files outside posters dir"
    );
}

#[test]
fn test_poster_storage_reconciliation_part_trash_and_orphans() {
    let temp_dir = tempdir().unwrap();
    let posters_path = temp_dir.path().join("posters");
    std::fs::create_dir_all(posters_path.join(".trash")).unwrap();

    // Create a .part file in posters_dir
    let part_file = posters_path.join("dangling_upload.jpg.part");
    std::fs::write(&part_file, b"incomplete-data").unwrap();

    // Create a file in .trash
    let trash_file = posters_path.join(".trash").join("old_deleted.jpg");
    std::fs::write(&trash_file, b"deleted-data").unwrap();

    // Create an orphan image and an active image
    let orphan_file = posters_path.join("evt_orphan_123.jpg");
    std::fs::write(&orphan_file, b"orphan-data").unwrap();
    let active_file = posters_path.join("evt_active_456.jpg");
    std::fs::write(&active_file, b"active-data").unwrap();

    let storage = LocalPosterStorage::new(posters_path.clone());
    assert!(
        !part_file.exists(),
        ".part files must be cleaned at startup"
    );
    assert!(
        !trash_file.exists(),
        ".trash files must be cleaned at startup"
    );
    assert!(
        orphan_file.exists(),
        "orphan file exists before orphan reconciliation"
    );
    assert!(
        active_file.exists(),
        "active file exists before orphan reconciliation"
    );

    // Reconcile orphans
    let active_list = vec!["evt_active_456.jpg".to_string()];
    let cleaned = storage.reconcile_orphans(&active_list);
    assert_eq!(cleaned, 1);
    assert!(!orphan_file.exists(), "Orphan poster must be removed");
    assert!(active_file.exists(), "Active poster must be preserved");
}

#[test]
fn test_delete_event_returns_cleanup_pending_outcome() {
    let temp_dir = tempdir().unwrap();
    let posters_path = temp_dir.path().join("posters");
    let storage = Arc::new(LocalPosterStorage::new(posters_path.clone()));

    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();
    let repo = Arc::new(SqliteTicketingRepository::new(pool));

    let create_event = CreateEvent::new(repo.clone());
    let replace_poster = ReplaceEventPoster::new(repo.clone(), storage.clone());
    let delete_event = DeleteEvent::new(repo.clone(), storage.clone());

    let event = create_event
        .execute(CreateEventInput {
            nombre: "Evento Outcome".to_string(),
            fecha_hora: "2026-12-01T20:00:00Z".to_string(),
            lugar: "Miraflores".to_string(),
            tipos_de_entrada: vec![
                TicketType::new("General", Money::from_cents(2000), 10).unwrap(),
            ],
        })
        .unwrap();

    let poster_file = replace_poster.execute(&event.id, SAMPLE_WEBP).unwrap();
    assert!(posters_path.join(&poster_file).exists());

    let result = delete_event.execute(&event.id).unwrap();
    assert!(result.deleted);
    assert!(!result.cleanup_pending);
    assert!(!posters_path.join(&poster_file).exists());
}
