use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tempfile::TempDir;

use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::domain::youtube_url::YouTubeUrl;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::filesystem::staged_promotion_guard::StagedPromotionGuard;
use shirley_show::infra::sqlite::connect::{DbPool, init_pool};
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use shirley_show::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use shirley_show::ports::youtube_downloader::{
    DownloadedYoutubeAudio, YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};
use shirley_show::state::staged_audio_registry::StagedAudioRegistry;
use shirley_show::use_cases::download_youtube_audio::DownloadYouTubeAudio;
use shirley_show::use_cases::upload_track::UploadTrack;

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
    ) -> Pin<Box<dyn Future<Output = Result<DownloadedYoutubeAudio, YoutubeDownloadError>> + Send>> {
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

struct TestContext {
    storage: Arc<LocalAudioStorage>,
    registry: Arc<StagedAudioRegistry>,
    item_repo: Arc<SqliteShowItemRepository>,
    _pres_repo: Arc<SqlitePresentationRepository>,
    _pool: DbPool,
    _dir: TempDir,
}

impl TestContext {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.sqlite3");
        let pool = init_pool(db_path.to_str().expect("path")).expect("pool");
        migrate_pool(&pool).expect("migrate");

        let storage = Arc::new(LocalAudioStorage::new(dir.path().join("audio")));
        let registry = Arc::new(StagedAudioRegistry::new());
        let item_repo = Arc::new(SqliteShowItemRepository::new(pool.clone()));
        let pres_repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));

        Self {
            storage,
            registry,
            item_repo,
            _pres_repo: pres_repo,
            _pool: pool,
            _dir: dir,
        }
    }
}

#[tokio::test]
async fn test_title_fallback_hierarchy() {
    let ctx = TestContext::new();

    // 1. Requested title takes precedence when non-empty
    let downloader1 = Arc::new(FakeDownloader::new("Metadata Title", 180, 1024));
    let use_case1 = DownloadYouTubeAudio::new(downloader1, ctx.storage.clone(), ctx.registry.clone());
    let out1 = use_case1
        .execute(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            Some("  Solicited Title  "),
        )
        .await
        .unwrap();
    assert_eq!(out1.title, "Solicited Title");

    // 2. Requested title is whitespace -> fallback to metadata title
    let downloader2 = Arc::new(FakeDownloader::new("  Metadata Title From YT  ", 180, 1024));
    let use_case2 = DownloadYouTubeAudio::new(downloader2, ctx.storage.clone(), ctx.registry.clone());
    let out2 = use_case2
        .execute(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            Some("   \t  "),
        )
        .await
        .unwrap();
    assert_eq!(out2.title, "Metadata Title From YT");

    // 3. Requested title is None and metadata title is whitespace -> fallback to 11-char video ID
    let downloader3 = Arc::new(FakeDownloader::new("    ", 180, 1024));
    let use_case3 = DownloadYouTubeAudio::new(downloader3, ctx.storage.clone(), ctx.registry.clone());
    let out3 = use_case3
        .execute("https://www.youtube.com/watch?v=dQw4w9WgXcQ", None)
        .await
        .unwrap();
    assert_eq!(out3.title, "dQw4w9WgXcQ");

    // 4. Sanitization: control characters stripped and collapsed spaces
    let downloader4 = Arc::new(FakeDownloader::new("Raw\x00 Title\r\n\tWith\x1bControls", 180, 1024));
    let use_case4 = DownloadYouTubeAudio::new(downloader4, ctx.storage.clone(), ctx.registry.clone());
    let out4 = use_case4
        .execute("https://www.youtube.com/watch?v=dQw4w9WgXcQ", None)
        .await
        .unwrap();
    assert_eq!(out4.title, "Raw Title With Controls");
}

#[tokio::test]
async fn test_download_success_registers_in_staging_and_registry() {
    let ctx = TestContext::new();
    let downloader = Arc::new(FakeDownloader::new("Great Song", 210, 3_000_000));
    let use_case = DownloadYouTubeAudio::new(downloader, ctx.storage.clone(), ctx.registry.clone());

    let out = use_case
        .execute("https://youtu.be/dQw4w9WgXcQ", Some("My Song"))
        .await
        .unwrap();

    assert!(out.staged_audio_id.starts_with("staged-"));
    assert_eq!(out.title, "My Song");
    assert_eq!(out.duration_secs, 210);
    assert_eq!(out.size_bytes, 3_000_000);

    // Assert that the audio can be reserved from the registry
    let lease = ctx.registry.reserve(&out.staged_audio_id).unwrap();
    assert_eq!(lease.title, "My Song");
    assert!(lease.file_path.exists(), "Staged .part file must exist on disk");
}

#[tokio::test]
async fn test_download_failure_purges_staging_file() {
    let ctx = TestContext::new();
    let downloader = Arc::new(FakeDownloader::failing(YoutubeDownloadError::DownloadTimeout));
    let use_case = DownloadYouTubeAudio::new(downloader, ctx.storage.clone(), ctx.registry.clone());

    let err = use_case
        .execute("https://youtu.be/dQw4w9WgXcQ", None)
        .await
        .unwrap_err();

    assert_eq!(err.code(), "download_timeout");

    // Verify staging directory is clean
    let entries: Vec<_> = std::fs::read_dir(ctx.storage.staging_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 0, "No leftover staging files on failure");
}

#[test]
fn test_staged_promotion_guard_reversion_on_drop() {
    let ctx = TestContext::new();
    let (_stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio payload to revert").unwrap();

    let dest_path = ctx.storage.files_dir().join("managed-revert-key.mp3");

    // Simulate promotion rename
    std::fs::rename(&stage_path, &dest_path).unwrap();
    assert!(!stage_path.exists());
    assert!(dest_path.exists());

    // Create guard and drop without defusing
    {
        let _guard = StagedPromotionGuard::new(stage_path.clone(), dest_path.clone());
        // Guard drops here
    }

    // Reversion must move dest_path BACK to stage_path
    assert!(stage_path.exists(), "Staged file must be safely restored to staging on guard drop");
    assert!(!dest_path.exists(), "Promoted file must not remain in files directory");
}

#[test]
fn test_staged_promotion_guard_persists_on_defuse() {
    let ctx = TestContext::new();
    let (_stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"audio payload to persist").unwrap();

    let dest_path = ctx.storage.files_dir().join("managed-persist-key.mp3");

    std::fs::rename(&stage_path, &dest_path).unwrap();

    {
        let mut guard = StagedPromotionGuard::new(stage_path.clone(), dest_path.clone());
        guard.defuse();
    }

    assert!(!stage_path.exists());
    assert!(dest_path.exists(), "Promoted file must remain in files dir when defused");
}

#[test]
fn test_upload_track_reverts_to_staging_on_db_insert_failure() {
    let ctx = TestContext::new();
    let upload_track = UploadTrack::new(ctx.item_repo.clone(), ctx.storage.clone());

    let (stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, b"important staged audio bytes").unwrap();

    // Presentation does not exist -> DB append will fail
    let err = upload_track.execute(
        "00000000-0000-0000-0000-000000000000",
        "Failed Track Title",
        &stage_id,
        AudioFormat::Mp3,
        28,
    );
    assert!(err.is_err());

    // Staging file must be safely reverted to staging for retry!
    assert!(
        stage_path.exists(),
        "Staged file must be restored to audio/staging/{{stage_id}}.part on DB failure"
    );

    // No files in managed storage
    let files: Vec<_> = std::fs::read_dir(ctx.storage.files_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 0, "No orphaned files in files dir");
}
