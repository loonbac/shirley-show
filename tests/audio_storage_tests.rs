use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::infra::filesystem::detect_audio_format::detect_audio_format;
use shirley_show::infra::filesystem::generated_audio_key::GeneratedAudioKey;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::filesystem::reconcile_audio_files::reconcile_audio_files;
use shirley_show::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use shirley_show::infra::sqlite::connect::init_pool;
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::ports::cleanup_queue::CleanupQueue;
use std::io::Write;

#[test]
fn test_detect_audio_format_magic_bytes() {
    // MP3 with ID3 header
    let mp3_id3 = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(
        detect_audio_format(&mp3_id3, "mp3").unwrap(),
        AudioFormat::Mp3
    );
    assert_eq!(
        detect_audio_format(&mp3_id3, ".MP3").unwrap(),
        AudioFormat::Mp3
    );

    // MP3 with raw frame sync (0xFF, 0xFB)
    let mp3_frame = [0xFF, 0xFB, 0x90, 0x64, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(
        detect_audio_format(&mp3_frame, "mp3").unwrap(),
        AudioFormat::Mp3
    );

    // WAV RIFF/WAVE header
    let mut wav = vec![0u8; 12];
    wav[0..4].copy_from_slice(b"RIFF");
    wav[8..12].copy_from_slice(b"WAVE");
    assert_eq!(detect_audio_format(&wav, "wav").unwrap(), AudioFormat::Wav);

    // M4A ISO-BMFF header with ftyp
    let mut m4a = vec![0u8; 12];
    m4a[4..8].copy_from_slice(b"ftyp");
    assert_eq!(detect_audio_format(&m4a, "m4a").unwrap(), AudioFormat::M4a);

    // Mismatch: WAV header with .mp3 extension
    assert!(detect_audio_format(&wav, "mp3").is_err());

    // Mismatch: Random text file with .mp3 extension
    let text = b"Hello world this is not audio";
    assert!(detect_audio_format(text, "mp3").is_err());

    // Unsupported extension
    assert!(detect_audio_format(&mp3_id3, "flac").is_err());
    assert!(detect_audio_format(&mp3_id3, "ogg").is_err());

    // Corrupted WAV: RIFF but missing WAVE
    let mut bad_wav = vec![0u8; 12];
    bad_wav[0..4].copy_from_slice(b"RIFF");
    bad_wav[8..12].copy_from_slice(b"AVI ");
    assert!(detect_audio_format(&bad_wav, "wav").is_err());

    // Corrupted M4A: missing ftyp
    let mut bad_m4a = vec![0u8; 12];
    bad_m4a[4..8].copy_from_slice(b"moov");
    assert!(detect_audio_format(&bad_m4a, "m4a").is_err());
}

#[test]
fn test_generated_audio_key_validation_and_traversal_resistance() {
    let key = GeneratedAudioKey::generate(AudioFormat::Mp3);
    assert!(key.as_str().ends_with(".mp3"));
    assert_eq!(key.format(), AudioFormat::Mp3);

    // Valid parse
    let parsed = GeneratedAudioKey::parse(key.as_str()).unwrap();
    assert_eq!(parsed.as_str(), key.as_str());

    // Directory traversal attacks
    assert!(GeneratedAudioKey::parse("../secret.mp3").is_err());
    assert!(GeneratedAudioKey::parse("..\\secret.mp3").is_err());
    assert!(GeneratedAudioKey::parse("audio/nested.mp3").is_err());
    assert!(GeneratedAudioKey::parse("/etc/passwd.mp3").is_err());
    assert!(GeneratedAudioKey::parse("something..mp3").is_err());

    // Invalid uuid or extension
    assert!(GeneratedAudioKey::parse("not-a-uuid.mp3").is_err());
    assert!(GeneratedAudioKey::parse("00000000-0000-0000-0000-000000000000.flac").is_err());
    assert!(GeneratedAudioKey::parse("").is_err());
}

#[test]
fn test_local_audio_storage_stage_promote_delete() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = LocalAudioStorage::new(temp_dir.path().join("audio"));

    // Stage a file
    let (stage_id, stage_path) = storage.create_staging_file().unwrap();
    assert!(stage_path.exists());
    std::fs::write(&stage_path, b"dummy audio content").unwrap();

    // Promote
    let key = storage
        .promote_staged_file(&stage_id, AudioFormat::Mp3)
        .unwrap();
    assert!(!stage_path.exists(), "Staged file should be moved/renamed");

    let resolved_path = storage.resolve_path(&key);
    assert!(resolved_path.exists());
    let read_back = std::fs::read(&resolved_path).unwrap();
    assert_eq!(read_back, b"dummy audio content");

    // Delete
    let deleted = storage.delete_file(key.as_str()).unwrap();
    assert!(deleted);
    assert!(!resolved_path.exists());

    // Delete again returns false
    let deleted_again = storage.delete_file(key.as_str()).unwrap();
    assert!(!deleted_again);
}

#[test]
fn test_cleanup_queue_and_reconciliation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = LocalAudioStorage::new(temp_dir.path().join("audio"));
    let cleanup_repo = SqliteCleanupRepository::new(pool.clone());

    // Create a real file on disk and queue it in pending_audio_deletions
    let (stage_id, stage_path) = storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio to clean up").unwrap();
    let key = storage
        .promote_staged_file(&stage_id, AudioFormat::Mp3)
        .unwrap();

    // Add to pending cleanup queue
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO pending_audio_deletions (audio_key, created_at, attempts) VALUES (?1, '2026-03-30T00:00:00Z', 0);",
        [key.as_str()],
    ).unwrap();

    let pending_keys = cleanup_repo.get_pending().unwrap();
    assert_eq!(pending_keys.len(), 1);
    assert_eq!(pending_keys[0], key.as_str());

    // Also create a dangling staging file
    let (_dangling_id, dangling_path) = storage.create_staging_file().unwrap();
    let mut f = std::fs::File::create(&dangling_path).unwrap();
    f.write_all(b"dangling").unwrap();

    // Reconcile
    let report = reconcile_audio_files(&storage, &cleanup_repo).unwrap();
    assert_eq!(report.processed_deletions, 1);
    assert_eq!(report.failed_deletions, 0);
    assert!(report.purged_staging >= 1);

    // Verify key was removed from pending queue
    assert_eq!(cleanup_repo.get_pending().unwrap().len(), 0);

    // Verify file was deleted from disk
    assert!(!storage.resolve_path(&key).exists());
}

#[test]
fn test_cleanup_failure_retains_record_with_error_and_attempts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let cleanup_repo = SqliteCleanupRepository::new(pool.clone());

    let invalid_key = "00000000-0000-0000-0000-000000000099.mp3";
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO pending_audio_deletions (audio_key, created_at, attempts) VALUES (?1, '2026-03-30T00:00:00Z', 0);",
        [invalid_key],
    ).unwrap();

    // Record failure explicitly
    cleanup_repo
        .record_failure(invalid_key, "Permission denied on host")
        .unwrap();

    // Verify attempts incremented and last_error recorded
    let (attempts, last_error): (i32, Option<String>) = conn
        .query_row(
            "SELECT attempts, last_error FROM pending_audio_deletions WHERE audio_key = ?1;",
            [invalid_key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();

    assert_eq!(attempts, 1);
    assert_eq!(last_error, Some("Permission denied on host".to_string()));

    // Key is still pending
    assert_eq!(cleanup_repo.get_pending().unwrap().len(), 1);

    // Once marked completed, it is removed
    cleanup_repo.mark_completed(invalid_key).unwrap();
    assert_eq!(cleanup_repo.get_pending().unwrap().len(), 0);
}
