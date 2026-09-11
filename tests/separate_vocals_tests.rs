use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::domain::presentation::Presentation;
use shirley_show::domain::show_item::NotePayload;
use shirley_show::use_cases::upload_track::UploadTrack;
use shirley_show::domain::show_item::{ShowItem, ShowItemPayload, TrackPayload};
use shirley_show::ports::presentation_writer::PresentationWriter;
use shirley_show::ports::show_item_writer::ShowItemWriter;
use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::domain::youtube_url::YouTubeUrl;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::sqlite::connect::{DbPool, init_pool};
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use shirley_show::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use shirley_show::ports::vocal_separator::{
    SeparatedAudio, VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};
use shirley_show::ports::youtube_downloader::{
    DownloadedYoutubeAudio, YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};
use shirley_show::state::staged_audio_registry::{StagedAudioRegistry, StagedAudioState};
use shirley_show::use_cases::download_youtube_audio::DownloadYouTubeAudio;
use shirley_show::use_cases::separate_vocals::{
    SeparateVocals, SeparateVocalsInput, VocalAudioSource,
};

#[derive(Clone)]
struct FakeVocalSeparator {
    duration_secs: u32,
    size_bytes: u64,
    fail_with_error: Arc<Mutex<Option<VocalSeparationError>>>,
    requests: Arc<Mutex<Vec<VocalSeparationRequest>>>,
}

impl FakeVocalSeparator {
    fn new(duration_secs: u32, size_bytes: u64) -> Self {
        Self {
            duration_secs,
            size_bytes,
            fail_with_error: Arc::new(Mutex::new(None)),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failing(err: VocalSeparationError) -> Self {
        Self {
            duration_secs: 0,
            size_bytes: 0,
            fail_with_error: Arc::new(Mutex::new(Some(err))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn recorded_requests(&self) -> Vec<VocalSeparationRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl VocalSeparator for FakeVocalSeparator {
    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>> {
        self.requests.lock().unwrap().push(request.clone());

        if let Some(ref err) = *self.fail_with_error.lock().unwrap() {
            let err = err.clone();
            return Box::pin(async move { Err(err) });
        }

        let duration_secs = self.duration_secs;
        let size_bytes = self.size_bytes;

        Box::pin(async move {
            // Simulate writing a valid MP3 file to output_path
            std::fs::write(&request.output_path, b"ID3\x03\0\0\0\0\0\0dummy-320k-mp3-bytes")
                .expect("write dummy mp3");
            Ok(SeparatedAudio {
                duration_secs,
                size_bytes,
            })
        })
    }
}

struct FakeDownloader {
    title: String,
    duration_secs: u32,
    size_bytes: u64,
    fail_with_error: Option<YoutubeDownloadError>,
}

impl FakeDownloader {
    fn new(title: &str, duration_secs: u32, size_bytes: u64) -> Self {
        Self {
            title: title.to_string(),
            duration_secs,
            size_bytes,
            fail_with_error: None,
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

        let title = self.title.clone();
        let duration_secs = self.duration_secs;
        let size_bytes = self.size_bytes;

        Box::pin(async move {
            let part_path = target.staging_dir.join(format!("{}.part", target.stage_id));
            std::fs::write(&part_path, b"dummy downloaded audio").expect("write downloaded part");

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
    pres_repo: Arc<SqlitePresentationRepository>,
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
            pres_repo,
            _pool: pool,
            _dir: dir,
        }
    }

    fn create_staged_audio(&self, title: &str, content: &[u8]) -> (String, PathBuf) {
        let (stage_id, stage_path) = self.storage.create_staging_file().expect("create staging file");
        std::fs::write(&stage_path, content).expect("write staging content");
        let descriptor = self.registry.register(
            stage_id,
            stage_path.clone(),
            title.to_string(),
            180,
            content.len() as u64,
        );
        (descriptor.staged_audio_id, stage_path)
    }

    fn create_managed_track(&self, presentation_id: &str, title: &str, content: &[u8]) -> (ShowItem, PathBuf) {
        let audio_key = format!("{}.mp3", uuid::Uuid::new_v4());
        let track_path = self.storage.files_dir().join(&audio_key);
        std::fs::create_dir_all(self.storage.files_dir()).expect("create files dir");
        std::fs::write(&track_path, content).expect("write track content");

        let payload = ShowItemPayload::Track(TrackPayload {
            title: title.to_string(),
            audio_key,
            media_type: "audio/mpeg".to_string(),
            byte_size: content.len() as i64,
        });

        let item = self.item_repo.append(presentation_id, payload).expect("append show item");
        (item, track_path)
    }

    fn build_separate_vocals(
        &self,
        separator: Arc<dyn VocalSeparator>,
        downloader: Arc<dyn YoutubeDownloader>,
    ) -> SeparateVocals<SqliteShowItemRepository> {
        let download_youtube_audio = Arc::new(DownloadYouTubeAudio::new(
            downloader,
            self.storage.clone(),
            self.registry.clone(),
        ));
        SeparateVocals::new(
            separator,
            self.storage.clone(),
            self.registry.clone(),
            self.item_repo.clone(),
            download_youtube_audio,
        )
    }
}

#[tokio::test]
async fn test_separate_vocals_from_staged_audio_id_success() {
    let ctx = TestContext::new();
    let (staged_audio_id, source_path) = ctx.create_staged_audio("Bohemian Rhapsody", b"source audio content");
    assert!(source_path.exists());

    let separator = Arc::new(FakeVocalSeparator::new(355, 14_000_000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator.clone(), downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId(staged_audio_id.clone()),
        mode: VocalSeparationMode::Coros,
    };

    let output = use_case.execute(input).await.expect("separation succeeds");

    // 1. Title must have suffix " (Coros)"
    assert_eq!(output.title, "Bohemian Rhapsody (Coros)");
    assert_eq!(output.mode, VocalSeparationMode::Coros);
    assert_eq!(output.duration_secs, 355);
    assert_eq!(output.size_bytes, 14_000_000);

    // 2. Resulting staged_audio_id must be registered in StagedAudioRegistry
    assert!(output.staged_audio_id.starts_with("staged-"));
    assert_ne!(output.staged_audio_id, staged_audio_id);
    assert_eq!(
        ctx.registry.get_state(&output.staged_audio_id).unwrap(),
        StagedAudioState::Available
    );
    let desc = ctx.registry.get_descriptor(&output.staged_audio_id).unwrap();
    assert_eq!(desc.title, "Bohemian Rhapsody (Coros)");

    // 3. Source staged file must be removed from disk via commit_processed
    assert!(!source_path.exists(), "Source staged file must be deleted after commit_processed");

    // 4. Source staged_audio_id must be marked as Consumed (tombstone retained)
    assert_eq!(
        ctx.registry.get_state(&staged_audio_id).unwrap(),
        StagedAudioState::Consumed
    );

    // 5. Separator must have received the source path
    let calls = separator.recorded_requests();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].source_path, source_path);
    assert_eq!(calls[0].mode, VocalSeparationMode::Coros);
}

#[tokio::test]
async fn test_separate_vocals_from_track_id_preserves_original_file() {
    let ctx = TestContext::new();
    let pres = Presentation::new("Show Viernes").expect("pres new");
    ctx.pres_repo.create(&pres).expect("create pres");
    let (track_item, original_track_path) =
        ctx.create_managed_track(&pres.id, "Hotel California", b"managed track audio 44100");
    assert!(original_track_path.exists());

    let separator = Arc::new(FakeVocalSeparator::new(390, 15_600_000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator.clone(), downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::TrackId(track_item.id.clone()),
        mode: VocalSeparationMode::Instrumental,
    };

    let output = use_case.execute(input).await.expect("separation succeeds");

    // 1. Output title has " (Instrumental)"
    assert_eq!(output.title, "Hotel California (Instrumental)");
    assert_eq!(output.mode, VocalSeparationMode::Instrumental);

    // 2. Output is staged
    assert_eq!(
        ctx.registry.get_state(&output.staged_audio_id).unwrap(),
        StagedAudioState::Available
    );

    // 3. Original track audio file MUST REMAIN INTACT on disk (read-only)
    assert!(
        original_track_path.exists(),
        "Managed track audio file must NEVER be deleted or modified"
    );
    let original_bytes = std::fs::read(&original_track_path).expect("read original track");
    assert_eq!(original_bytes, b"managed track audio 44100");

    // 4. Separator received the managed track path
    let calls = separator.recorded_requests();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].source_path, original_track_path);
}

#[tokio::test]
async fn test_separate_vocals_from_youtube_url_success() {
    let ctx = TestContext::new();
    let separator = Arc::new(FakeVocalSeparator::new(210, 8_400_000));
    let downloader = Arc::new(FakeDownloader::new("Never Gonna Give You Up", 210, 8_400_000));
    let use_case = ctx.build_separate_vocals(separator.clone(), downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::YoutubeUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
        mode: VocalSeparationMode::Coros,
    };

    let output = use_case.execute(input).await.expect("youtube separation succeeds");

    assert_eq!(output.title, "Never Gonna Give You Up (Coros)");
    assert_eq!(output.mode, VocalSeparationMode::Coros);
    assert_eq!(
        ctx.registry.get_state(&output.staged_audio_id).unwrap(),
        StagedAudioState::Available
    );
}

#[test]
fn test_source_resolution_priority_order() {
    // staged_audio_id > track_id > youtube_url
    let res1 = VocalAudioSource::resolve(
        Some("staged-123".to_string()),
        Some("track-456".to_string()),
        Some("https://youtu.be/abc".to_string()),
    ).expect("resolve ok");
    assert_eq!(res1, VocalAudioSource::StagedAudioId("staged-123".to_string()));

    // track_id > youtube_url when staged is None or empty
    let res2 = VocalAudioSource::resolve(
        Some("   ".to_string()),
        Some("track-456".to_string()),
        Some("https://youtu.be/abc".to_string()),
    ).expect("resolve ok");
    assert_eq!(res2, VocalAudioSource::TrackId("track-456".to_string()));

    // youtube_url when others are None
    let res3 = VocalAudioSource::resolve(
        None,
        None,
        Some("https://youtu.be/abc".to_string()),
    ).expect("resolve ok");
    assert_eq!(res3, VocalAudioSource::YoutubeUrl("https://youtu.be/abc".to_string()));

    // None of all -> SourceRequired error
    let err = VocalAudioSource::resolve(None, None, None).unwrap_err();
    assert_eq!(err.code(), "source_required");
}

#[tokio::test]
async fn test_separator_failure_preserves_source_staged_audio() {
    let ctx = TestContext::new();
    let (staged_audio_id, source_path) = ctx.create_staged_audio("Retry Track", b"source retry content");
    assert!(source_path.exists());

    let separator = Arc::new(FakeVocalSeparator::failing(VocalSeparationError::TranscodeFailed));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId(staged_audio_id.clone()),
        mode: VocalSeparationMode::Coros,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "transcode_failed");

    // Source staged file must STILL EXIST on disk
    assert!(
        source_path.exists(),
        "Source staged file must remain intact on disk when separator fails"
    );

    // Source staged_audio_id must STILL BE AVAILABLE in registry for retry
    assert_eq!(
        ctx.registry.get_state(&staged_audio_id).unwrap(),
        StagedAudioState::Available
    );
}

#[tokio::test]
async fn test_instrumental_unavailable_degrades_gracefully() {
    let ctx = TestContext::new();
    let (staged_audio_id, source_path) = ctx.create_staged_audio("Instrumental Source", b"inst audio content");

    let separator = Arc::new(FakeVocalSeparator::failing(VocalSeparationError::InstrumentalUnavailable));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId(staged_audio_id.clone()),
        mode: VocalSeparationMode::Instrumental,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "instrumental_unavailable");
    assert_eq!(err.message(), "El modo instrumental no está disponible en el servidor");

    // Source remains available
    assert!(source_path.exists());
    assert_eq!(
        ctx.registry.get_state(&staged_audio_id).unwrap(),
        StagedAudioState::Available
    );
}

#[tokio::test]
async fn test_separate_vocals_nonexistent_staged_audio_id_returns_not_found() {
    let ctx = TestContext::new();
    let separator = Arc::new(FakeVocalSeparator::new(180, 1000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId("staged-nonexistent-uuid".to_string()),
        mode: VocalSeparationMode::Coros,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "staged_audio_not_found");
    assert_eq!(err.message(), "Archivo de audio temporal no encontrado");
}

#[tokio::test]
async fn test_separate_vocals_already_consumed_staged_audio_rejected() {
    let ctx = TestContext::new();
    let (staged_audio_id, _path) = ctx.create_staged_audio("Consumed Track", b"consumed content");

    // Manually reserve and commit as consumed
    let lease = ctx.registry.reserve(&staged_audio_id).unwrap();
    ctx.registry.commit_consumed(&staged_audio_id, &lease.lease_id).unwrap();

    let separator = Arc::new(FakeVocalSeparator::new(180, 1000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId(staged_audio_id),
        mode: VocalSeparationMode::Coros,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "audio_already_consumed");
    assert_eq!(err.message(), "El archivo de audio ya fue consumido");
}

#[tokio::test]
async fn test_separate_vocals_nonexistent_track_id_returns_track_not_found() {
    let ctx = TestContext::new();
    let separator = Arc::new(FakeVocalSeparator::new(180, 1000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::TrackId("track-nonexistent-uuid".to_string()),
        mode: VocalSeparationMode::Instrumental,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "track_not_found");
    assert_eq!(err.message(), "Pista no encontrada");
}

#[tokio::test]
async fn test_separate_vocals_track_id_not_a_track_returns_source_not_track() {
    let ctx = TestContext::new();
    let pres = Presentation::new("Show Notas").expect("pres new");
    ctx.pres_repo.create(&pres).expect("create pres");

    // Create a note item instead of a track
    let note_item = ctx.item_repo.append(
        &pres.id,
        ShowItemPayload::Note(NotePayload::new("Una nota").unwrap()),
    ).expect("append note");

    let separator = Arc::new(FakeVocalSeparator::new(180, 1000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::TrackId(note_item.id),
        mode: VocalSeparationMode::Coros,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "source_not_track");
    assert_eq!(err.message(), "El elemento seleccionado no es una pista de audio");
}

#[tokio::test]
async fn test_separate_vocals_youtube_download_failure_aborts_before_separator() {
    let ctx = TestContext::new();
    let separator = Arc::new(FakeVocalSeparator::new(180, 1000));
    let mut downloader = FakeDownloader::new("unused", 0, 0);
    downloader.fail_with_error = Some(YoutubeDownloadError::DownloadFailed);

    let use_case = ctx.build_separate_vocals(separator.clone(), Arc::new(downloader));

    let input = SeparateVocalsInput {
        source: VocalAudioSource::YoutubeUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string()),
        mode: VocalSeparationMode::Coros,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "download_failed");

    // Separator must NEVER have been invoked
    assert_eq!(separator.recorded_requests().len(), 0);
}

#[tokio::test]
async fn test_separate_vocals_duration_exceeded_propagates_and_preserves_source() {
    let ctx = TestContext::new();
    let (staged_audio_id, source_path) = ctx.create_staged_audio("Long Track", b"long audio content");

    let separator = Arc::new(FakeVocalSeparator::failing(VocalSeparationError::DurationExceeded));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId(staged_audio_id.clone()),
        mode: VocalSeparationMode::Coros,
    };

    let err = use_case.execute(input).await.unwrap_err();
    assert_eq!(err.code(), "duration_exceeded");
    assert_eq!(
        err.message(),
        "El audio supera el límite máximo de 10 minutos para separación vocal"
    );

    // Staged source audio is preserved for retry
    assert!(source_path.exists());
    assert_eq!(
        ctx.registry.get_state(&staged_audio_id).unwrap(),
        StagedAudioState::Available
    );
}

#[tokio::test]
async fn test_separated_output_consumable_by_upload_track_to_canonical() {
    let ctx = TestContext::new();
    let pres = Presentation::new("Repertorio Oficial").expect("pres new");
    ctx.pres_repo.create(&pres).expect("create pres");

    let (staged_audio_id, _source_path) = ctx.create_staged_audio("Yesterday", b"source wav audio");
    let separator = Arc::new(FakeVocalSeparator::new(125, 5_000_000));
    let downloader = Arc::new(FakeDownloader::new("unused", 0, 0));
    let use_case = ctx.build_separate_vocals(separator, downloader);

    let input = SeparateVocalsInput {
        source: VocalAudioSource::StagedAudioId(staged_audio_id),
        mode: VocalSeparationMode::Coros,
    };

    let output = use_case.execute(input).await.expect("separate ok");
    assert_eq!(output.title, "Yesterday (Coros)");

    // Now consume the separated audio into the canonical show presentation via UploadTrack
    let upload_track = UploadTrack::new(ctx.item_repo.clone(), ctx.storage.clone());

    let lease = ctx.registry.reserve(&output.staged_audio_id).expect("reserve separated audio");
    let created_item = upload_track
        .execute(
            &pres.id,
            &output.title,
            &lease.stage_id,
            AudioFormat::Mp3,
            lease.size_bytes as i64,
        )
        .expect("upload_track succeeds");

    ctx.registry
        .commit_consumed(&lease.staged_audio_id, &lease.lease_id)
        .expect("commit consumed separated audio");

    // Verify canonical track item in presentation
    assert_eq!(created_item.presentation_id, pres.id);
    match created_item.payload {
        ShowItemPayload::Track(t) => {
            assert_eq!(t.title, "Yesterday (Coros)");
            assert_eq!(t.media_type, "audio/mpeg");
            let canonical_path = ctx.storage.resolve_path_str(&t.audio_key).expect("resolve path");
            assert!(canonical_path.exists(), "Canonical audio file must exist in managed files dir");
        }
        _ => panic!("Expected Track payload"),
    }

    // Separated staged audio cannot be consumed a second time
    let err = ctx.registry.reserve(&output.staged_audio_id).unwrap_err();
    assert_eq!(err, shirley_show::state::staged_audio_registry::StagedAudioRegistryError::AlreadyConsumed);
}
