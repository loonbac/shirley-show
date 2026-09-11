use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::filesystem::reconcile_audio_files::reconcile_audio_files;
use shirley_show::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use shirley_show::infra::sqlite::connect::init_pool;
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::state::staged_audio_registry::{
    StagedAudioRegistry, StagedAudioRegistryError, StagedAudioState,
};

struct TestHarness {
    _dir: TempDir,
    staging_dir: PathBuf,
}

impl TestHarness {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let staging_dir = dir.path().join("staging");
        std::fs::create_dir_all(&staging_dir).expect("create staging dir");
        Self {
            _dir: dir,
            staging_dir,
        }
    }

    fn create_dummy_part(&self, stage_id: &str) -> PathBuf {
        let path = self.staging_dir.join(format!("{stage_id}.part"));
        std::fs::write(&path, b"dummy audio content").expect("write dummy part");
        path
    }
}

#[test]
fn test_opaque_staged_audio_id_assignment() {
    let harness = TestHarness::new();
    let registry = StagedAudioRegistry::new();

    let stage_id = "test-internal-stage-uuid-001";
    let file_path = harness.create_dummy_part(stage_id);

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path.clone(),
        "Mi Canción".to_string(),
        180,
        3_500_000,
    );

    // Assert opaque public ID format and strict separation from stage_id
    assert!(
        descriptor.staged_audio_id.starts_with("staged-"),
        "staged_audio_id must start with 'staged-'"
    );
    assert_ne!(
        descriptor.staged_audio_id, stage_id,
        "Public staged_audio_id must strictly differ from internal filesystem stage_id"
    );
    assert_eq!(descriptor.stage_id, stage_id);
    assert_eq!(descriptor.title, "Mi Canción");
    assert_eq!(descriptor.duration_secs, 180);
    assert_eq!(descriptor.size_bytes, 3_500_000);
}

#[test]
fn test_lifecycle_available_to_reserved_to_consumed() {
    let harness = TestHarness::new();
    let registry = StagedAudioRegistry::new();

    let stage_id = "test-stage-lifecycle-002";
    let file_path = harness.create_dummy_part(stage_id);

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path,
        "Lifecycle Track".to_string(),
        210,
        4_200_000,
    );

    // 1. Initial state: Available
    assert_eq!(
        registry.get_state(&descriptor.staged_audio_id).unwrap(),
        StagedAudioState::Available
    );

    // 2. Reserve: transitions to Reserved(lease_id)
    let lease = registry.reserve(&descriptor.staged_audio_id).unwrap();
    assert_eq!(lease.staged_audio_id, descriptor.staged_audio_id);
    assert_eq!(lease.stage_id, stage_id);
    assert!(!lease.lease_id.is_empty());

    let state = registry.get_state(&descriptor.staged_audio_id).unwrap();
    assert_eq!(
        state,
        StagedAudioState::Reserved {
            lease_id: lease.lease_id.clone()
        }
    );

    // 3. Commit consumed: transitions to Consumed
    registry
        .commit_consumed(&descriptor.staged_audio_id, &lease.lease_id)
        .unwrap();

    assert_eq!(
        registry.get_state(&descriptor.staged_audio_id).unwrap(),
        StagedAudioState::Consumed
    );
}

#[test]
fn test_reject_second_consumption_with_audio_already_consumed() {
    let harness = TestHarness::new();
    let registry = StagedAudioRegistry::new();

    let stage_id = "test-stage-double-consume-003";
    let file_path = harness.create_dummy_part(stage_id);

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path,
        "Single Use Track".to_string(),
        120,
        2_500_000,
    );

    let lease = registry.reserve(&descriptor.staged_audio_id).unwrap();
    registry
        .commit_consumed(&descriptor.staged_audio_id, &lease.lease_id)
        .unwrap();

    // Second reservation must be rejected with AlreadyConsumed error code "audio_already_consumed"
    let err = registry.reserve(&descriptor.staged_audio_id).unwrap_err();
    assert_eq!(err, StagedAudioRegistryError::AlreadyConsumed);
    assert_eq!(err.code(), "audio_already_consumed");
    assert_eq!(err.message(), "El archivo de audio ya fue consumido");
}

#[test]
fn test_release_reservation_on_failure_allows_retry() {
    let harness = TestHarness::new();
    let registry = StagedAudioRegistry::new();

    let stage_id = "test-stage-release-retry-004";
    let file_path = harness.create_dummy_part(stage_id);

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path,
        "Retry Track".to_string(),
        150,
        3_000_000,
    );

    let lease = registry.reserve(&descriptor.staged_audio_id).unwrap();

    // While reserved, concurrent reserve attempt must be rejected as InUse
    let in_use_err = registry.reserve(&descriptor.staged_audio_id).unwrap_err();
    assert_eq!(in_use_err, StagedAudioRegistryError::InUse);

    // Release reservation (simulating show item DB insert failure)
    registry
        .release(&descriptor.staged_audio_id, &lease.lease_id)
        .unwrap();

    // Entry returns to Available
    assert_eq!(
        registry.get_state(&descriptor.staged_audio_id).unwrap(),
        StagedAudioState::Available
    );

    // Now a retry reservation succeeds with a new lease
    let second_lease = registry.reserve(&descriptor.staged_audio_id).unwrap();
    assert_ne!(second_lease.lease_id, lease.lease_id);
}

#[tokio::test]
async fn test_automatic_disk_purge_after_ttl_expiration() {
    let harness = TestHarness::new();
    // Use short TTL of 50 ms for testing automatic expiration
    let registry = StagedAudioRegistry::with_ttl(Duration::from_millis(50));

    let stage_id = "test-stage-ttl-purge-005";
    let file_path = harness.create_dummy_part(stage_id);
    assert!(file_path.exists(), "Staging file must exist initially");

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path.clone(),
        "Expiring Track".to_string(),
        90,
        1_800_000,
    );

    // Wait for the TTL task to wake up and purge the file
    tokio::time::sleep(Duration::from_millis(120)).await;

    // The physical staging file on disk must be automatically purged
    assert!(
        !file_path.exists(),
        "Staging file must be purged from disk after TTL expiration"
    );

    // Subsequent reserve must fail
    let err = registry.reserve(&descriptor.staged_audio_id).unwrap_err();
    assert!(
        err == StagedAudioRegistryError::NotFound || err == StagedAudioRegistryError::AlreadyConsumed,
        "Expired entry must no longer be reservable"
    );
}

#[test]
fn test_concurrent_reservation_stress_single_winner() {
    let harness = TestHarness::new();
    let registry = Arc::new(StagedAudioRegistry::new());

    let stage_id = "test-stage-concurrency-stress-006";
    let file_path = harness.create_dummy_part(stage_id);

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path,
        "Stress Track".to_string(),
        240,
        5_000_000,
    );

    let num_threads = 50;
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let reg = registry.clone();
        let target_id = descriptor.staged_audio_id.clone();
        handles.push(thread::spawn(move || reg.reserve(&target_id)));
    }

    let mut successes = 0;
    let mut in_use_failures = 0;

    for h in handles {
        match h.join().expect("thread join") {
            Ok(_) => successes += 1,
            Err(StagedAudioRegistryError::InUse) => in_use_failures += 1,
            Err(e) => panic!("Unexpected error under concurrency: {e:?}"),
        }
    }

    assert_eq!(
        successes, 1,
        "Exactly ONE concurrent caller must successfully acquire the lease"
    );
    assert_eq!(
        in_use_failures,
        num_threads - 1,
        "All other concurrent attempts must be rejected with InUse"
    );
}

#[test]
fn test_filesystem_compensation_failure_invalidates_entry_gracefully() {
    let harness = TestHarness::new();
    let registry = StagedAudioRegistry::new();

    let stage_id = "test-stage-fs-fail-007";
    let file_path = harness.create_dummy_part(stage_id);

    let descriptor = registry.register(
        stage_id.to_string(),
        file_path.clone(),
        "Fragile Track".to_string(),
        100,
        2_000_000,
    );

    let lease = registry.reserve(&descriptor.staged_audio_id).unwrap();

    // 1. Invalid lease rejected
    let bad_lease_err = registry
        .release(&descriptor.staged_audio_id, "wrong-lease-id")
        .unwrap_err();
    assert_eq!(bad_lease_err, StagedAudioRegistryError::InvalidLease);

    // 2. External removal of staging file during reservation
    std::fs::remove_file(&file_path).unwrap();
    assert!(!file_path.exists());

    // Release detects missing physical file, invalidates entry gracefully without panic
    let res = registry.release(&descriptor.staged_audio_id, &lease.lease_id);
    assert!(res.is_ok());

    // Entry is no longer available since file was missing
    let next_reserve = registry.reserve(&descriptor.staged_audio_id).unwrap_err();
    assert_eq!(next_reserve, StagedAudioRegistryError::NotFound);
}

#[test]
fn test_startup_reconciliation_purges_staging_but_never_managed_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().expect("path")).expect("pool");
    migrate_pool(&pool).expect("migrate");

    let audio_dir = dir.path().join("audio");
    let storage = LocalAudioStorage::new(&audio_dir);
    let cleanup_repo = SqliteCleanupRepository::new(pool);

    // 1. Create managed files in files/
    let managed1 = storage.files_dir().join("managed-track-001.mp3");
    let managed2 = storage.files_dir().join("managed-track-002.mp3");
    let managed3 = storage.files_dir().join("managed-track-003.mp3");
    std::fs::write(&managed1, b"permanent track 1").unwrap();
    std::fs::write(&managed2, b"permanent track 2").unwrap();
    std::fs::write(&managed3, b"permanent track 3").unwrap();

    // 2. Create orphan staging files in staging/
    let orphan1 = storage.staging_dir().join("orphan-yt-1.part");
    let orphan2 = storage.staging_dir().join("orphan-yt-2.mp3");
    let orphan3 = storage.staging_dir().join("temp-partial.tmp");
    std::fs::write(&orphan1, b"orphan 1").unwrap();
    std::fs::write(&orphan2, b"orphan 2").unwrap();
    std::fs::write(&orphan3, b"orphan 3").unwrap();

    assert_eq!(std::fs::read_dir(storage.staging_dir()).unwrap().count(), 3);
    assert_eq!(std::fs::read_dir(storage.files_dir()).unwrap().count(), 3);

    // 3. Run startup reconciliation
    let report = reconcile_audio_files(&storage, &cleanup_repo).expect("reconcile ok");
    assert_eq!(report.purged_staging, 3);

    // 4. Staging directory must be completely clean (0 files)
    assert_eq!(
        std::fs::read_dir(storage.staging_dir()).unwrap().count(),
        0,
        "Startup reconciliation must purge all files under audio/staging"
    );

    // 5. Managed files directory MUST NEVER BE TOUCHED (all 3 permanent tracks preserved!)
    assert_eq!(
        std::fs::read_dir(storage.files_dir()).unwrap().count(),
        3,
        "Startup reconciliation must NEVER delete files under audio/files"
    );
    assert!(managed1.exists());
    assert!(managed2.exists());
    assert!(managed3.exists());
}
