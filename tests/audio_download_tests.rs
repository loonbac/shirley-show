use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

use shirley_show::domain::audio_format::AudioFormat;
use shirley_show::domain::presentation::Presentation;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use shirley_show::infra::sqlite::connect::init_pool;
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use shirley_show::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use shirley_show::ports::presentation_writer::PresentationWriter;
use shirley_show::router::create_router;
use shirley_show::state::AppState;
use shirley_show::use_cases::upload_track::UploadTrack;

struct TestContext {
    app: axum::Router,
    presentation_id: String,
    upload_track: UploadTrack<SqliteShowItemRepository>,
    storage: Arc<LocalAudioStorage>,
    _dir: TempDir,
}

fn setup_download_test() -> TestContext {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(dir.path().join("audio")));
    let pres_repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));
    let item_repo = Arc::new(SqliteShowItemRepository::new(pool.clone()));
    let _cleanup_repo = Arc::new(SqliteCleanupRepository::new(pool.clone()));

    let pres = Presentation::new("Download Show").unwrap();
    pres_repo.create(&pres).unwrap();

    let upload_track = UploadTrack::new(item_repo.clone(), storage.clone());
    let state = AppState::new_with_items(pool, storage.clone());
    let app = create_router(state);

    TestContext {
        app,
        presentation_id: pres.id,
        upload_track,
        storage,
        _dir: dir,
    }
}

fn create_track(ctx: &TestContext, title: &str, format: AudioFormat, content: &[u8]) -> String {
    let (stage_id, stage_path) = ctx.storage.create_staging_file().unwrap();
    std::fs::write(&stage_path, content).unwrap();

    let track = ctx
        .upload_track
        .execute(
            &ctx.presentation_id,
            title,
            &stage_id,
            format,
            content.len() as i64,
        )
        .unwrap();

    track.id
}

#[tokio::test]
async fn test_audio_without_download_query_param_has_no_content_disposition() {
    let ctx = setup_download_test();
    let track_id = create_track(&ctx, "Track 1", AudioFormat::Mp3, b"0123456789ABCDEFGHIJ");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CONTENT_TYPE).unwrap(),
        "audio/mpeg"
    );
    assert!(
        res.headers().get(header::CONTENT_DISPOSITION).is_none(),
        "Content-Disposition must NOT be present when download query is not specified"
    );
}

#[tokio::test]
async fn test_audio_download_1_with_ascii_title() {
    let ctx = setup_download_test();
    let track_id = create_track(&ctx, "Track 1", AudioFormat::Mp3, b"0123456789ABCDEFGHIJ");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio?download=1"))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .expect("Content-Disposition should be present with download=1")
        .to_str()
        .unwrap();

    assert_eq!(disposition, "attachment; filename=\"Track 1.mp3\"");
}

#[tokio::test]
async fn test_audio_download_1_with_utf8_accents_and_enye() {
    let ctx = setup_download_test();
    // Title with accents, enye, and characters needing sanitization
    let title = "Canción de Año Nuevo / Shirley \"En Vivo\"";
    let track_id = create_track(&ctx, title, AudioFormat::Mp3, b"0123456789ABCDEFGHIJ");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio?download=1"))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .expect("Content-Disposition should be present with download=1")
        .to_str()
        .unwrap();

    // Must start with attachment
    assert!(
        disposition.starts_with("attachment;"),
        "Disposition must start with attachment: {disposition}"
    );

    // Must include sanitized ASCII fallback filename without quotes or slashes
    assert!(
        disposition.contains("filename=\"Cancion de Ano Nuevo Shirley En Vivo.mp3\""),
        "Disposition must contain correct sanitized ASCII filename: {disposition}"
    );

    // Must include RFC 5987 filename* with UTF-8 percent-encoding for accents and enye
    assert!(
        disposition.contains(
            "filename*=UTF-8''Canci%C3%B3n%20de%20A%C3%B1o%20Nuevo%20Shirley%20En%20Vivo.mp3"
        ),
        "Disposition must contain correct UTF-8 filename*: {disposition}"
    );
}

#[tokio::test]
async fn test_audio_download_1_with_range_request_206() {
    let ctx = setup_download_test();
    let track_id = create_track(
        &ctx,
        "Intro Show",
        AudioFormat::Mp3,
        b"0123456789ABCDEFGHIJ",
    );

    // Request bytes=0-15 with download=1
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio?download=1"))
        .header(header::RANGE, "bytes=0-15")
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(res.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
    assert_eq!(
        res.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 0-15/20"
    );
    assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "16");

    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .expect("Content-Disposition should be present in 206 response with download=1")
        .to_str()
        .unwrap();
    assert_eq!(disposition, "attachment; filename=\"Intro Show.mp3\"");

    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"0123456789ABCDEF");
}

#[tokio::test]
async fn test_audio_download_1_head_request_without_body() {
    let ctx = setup_download_test();
    let track_id = create_track(
        &ctx,
        "Pista HEAD",
        AudioFormat::Mp3,
        b"0123456789ABCDEFGHIJ",
    );

    let req = Request::builder()
        .method("HEAD")
        .uri(format!("/api/tracks/{track_id}/audio?download=1"))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
    assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "20");

    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .expect("Content-Disposition should be present in HEAD response with download=1")
        .to_str()
        .unwrap();
    assert_eq!(disposition, "attachment; filename=\"Pista HEAD.mp3\"");

    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body_bytes.len(), 0);
}

#[tokio::test]
async fn test_audio_download_formats_wav_and_m4a() {
    let ctx = setup_download_test();

    // WAV format
    let wav_id = create_track(&ctx, "Batería", AudioFormat::Wav, b"0123456789ABCDEFGHIJ");
    let req_wav = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{wav_id}/audio?download=1"))
        .body(Body::empty())
        .unwrap();

    let res_wav = ctx.app.clone().oneshot(req_wav).await.unwrap();
    assert_eq!(res_wav.status(), StatusCode::OK);
    let disp_wav = res_wav
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(disp_wav.contains("filename=\"Bateria.wav\""));
    assert!(disp_wav.contains("filename*=UTF-8''Bater%C3%ADa.wav"));

    // M4A format
    let m4a_id = create_track(&ctx, "Acústico", AudioFormat::M4a, b"0123456789ABCDEFGHIJ");
    let req_m4a = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{m4a_id}/audio?download=1"))
        .body(Body::empty())
        .unwrap();

    let res_m4a = ctx.app.oneshot(req_m4a).await.unwrap();
    assert_eq!(res_m4a.status(), StatusCode::OK);
    let disp_m4a = res_m4a
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(disp_m4a.contains("filename=\"Acustico.m4a\""));
    assert!(disp_m4a.contains("filename*=UTF-8''Ac%C3%BAstico.m4a"));
}

#[tokio::test]
async fn test_audio_download_sanitizes_empty_and_special_chars_fallback() {
    let ctx = setup_download_test();
    // Track title consisting only of slashes and quotes
    let title = "  \"///\\\\  ";
    let track_id = create_track(&ctx, title, AudioFormat::Mp3, b"0123456789ABCDEFGHIJ");

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio?download=1"))
        .body(Body::empty())
        .unwrap();

    let res = ctx.app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(disposition, "attachment; filename=\"pista.mp3\"");
}
