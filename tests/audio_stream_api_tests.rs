use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
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

fn setup_stream_test() -> (axum::Router, String, Arc<LocalAudioStorage>, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(dir.path().join("audio")));
    let pres_repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));
    let item_repo = Arc::new(SqliteShowItemRepository::new(pool.clone()));
    let _cleanup_repo = Arc::new(SqliteCleanupRepository::new(pool.clone()));

    let pres = Presentation::new("Stream Show").unwrap();
    pres_repo.create(&pres).unwrap();

    // Create a 20-byte dummy audio track
    let (stage_id, stage_path) = storage.create_staging_file().unwrap();
    let audio_bytes = b"0123456789ABCDEFGHIJ"; // 20 bytes
    std::fs::write(&stage_path, audio_bytes).unwrap();

    let upload_track = UploadTrack::new(item_repo.clone(), storage.clone());
    let track = upload_track
        .execute(&pres.id, "Track 1", &stage_id, AudioFormat::Mp3, 20)
        .unwrap();

    let state = AppState::new_with_items(pool, storage.clone());
    let app = create_router(state);

    (app, track.id, storage, dir)
}

#[tokio::test]
async fn test_audio_stream_full_request_200() {
    let (app, track_id, _storage, _dir) = setup_stream_test();

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
    assert_eq!(
        res.headers().get(header::CONTENT_TYPE).unwrap(),
        "audio/mpeg"
    );
    assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "20");

    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"0123456789ABCDEFGHIJ");
}

#[tokio::test]
async fn test_audio_stream_range_request_206() {
    let (app, track_id, _storage, _dir) = setup_stream_test();

    // Request bytes=0-4 (first 5 bytes)
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .header(header::RANGE, "bytes=0-4")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(res.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
    assert_eq!(
        res.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 0-4/20"
    );
    assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "5");

    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_bytes[..], b"01234");
}

#[tokio::test]
async fn test_audio_stream_out_of_bounds_range_416() {
    let (app, track_id, _storage, _dir) = setup_stream_test();

    // Request out of bounds
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .header(header::RANGE, "bytes=100-200")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        res.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes */20"
    );
}

#[tokio::test]
async fn test_audio_stream_head_request_without_body() {
    let (app, track_id, _storage, _dir) = setup_stream_test();

    let req = Request::builder()
        .method("HEAD")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
    assert_eq!(res.headers().get(header::CONTENT_LENGTH).unwrap(), "20");

    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body_bytes.len(), 0);
}

#[tokio::test]
async fn test_audio_stream_malformed_and_multi_range_fallback_to_200() {
    let (app, track_id, _storage, _dir) = setup_stream_test();

    // Malformed range: fallback to 200 full response
    let req_malformed = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .header(header::RANGE, "bytes=invalid")
        .body(Body::empty())
        .unwrap();

    let res_malformed = app.clone().oneshot(req_malformed).await.unwrap();
    assert_eq!(res_malformed.status(), StatusCode::OK);
    let bytes = res_malformed
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(bytes.len(), 20);

    // Multi-range: fallback to 200 full response
    let req_multi = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .header(header::RANGE, "bytes=0-2, 5-8")
        .body(Body::empty())
        .unwrap();

    let res_multi = app.oneshot(req_multi).await.unwrap();
    assert_eq!(res_multi.status(), StatusCode::OK);
    let bytes_multi = res_multi.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes_multi.len(), 20);
}

#[tokio::test]
async fn test_audio_stream_missing_track_and_missing_file_404() {
    let (app, track_id, storage, _dir) = setup_stream_test();

    // 1. Missing track ID in database
    let req_not_found = Request::builder()
        .method("GET")
        .uri("/api/tracks/00000000-0000-0000-0000-000000000000/audio")
        .body(Body::empty())
        .unwrap();

    let res_not_found = app.clone().oneshot(req_not_found).await.unwrap();
    assert_eq!(res_not_found.status(), StatusCode::NOT_FOUND);

    // 2. Track exists in DB, but file missing on disk -> 404 audio_unavailable
    // Find and delete the file from storage
    let files: Vec<_> = std::fs::read_dir(storage.files_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    for f in files {
        std::fs::remove_file(f.path()).unwrap();
    }

    let req_missing_file = Request::builder()
        .method("GET")
        .uri(format!("/api/tracks/{track_id}/audio"))
        .body(Body::empty())
        .unwrap();

    let res_missing_file = app.oneshot(req_missing_file).await.unwrap();
    assert_eq!(res_missing_file.status(), StatusCode::NOT_FOUND);

    let bytes = res_missing_file
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "audio_unavailable");
}
