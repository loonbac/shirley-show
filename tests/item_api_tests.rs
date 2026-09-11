use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::sqlite::connect::init_pool;
use shirley_show::infra::sqlite::migrate::migrate_pool;
use shirley_show::router::create_router;
use shirley_show::state::AppState;

async fn setup_test_app() -> (axum::Router, String, Arc<LocalAudioStorage>, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(dir.path().join("audio")));
    let state = AppState::new_with_items(pool, storage.clone());
    let app = create_router(state);

    // Create a presentation
    let req = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Main Concert" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let pres_id = json["presentation"]["id"].as_str().unwrap().to_string();

    (app, pres_id, storage, dir)
}

async fn body_to_json(body: Body) -> Value {
    let bytes = body.collect().await.expect("collect body").to_bytes();
    serde_json::from_slice(&bytes).expect("parse json")
}

fn build_multipart_body(boundary: &str, title: &str, filename: &str, file_bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();

    // title field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"title\"\r\n\r\n");
    body.extend_from_slice(title.as_bytes());
    body.extend_from_slice(b"\r\n");

    // audio field
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"audio\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(b"\r\n");

    // final boundary
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    body
}

#[tokio::test]
async fn test_add_text_items_and_get_presentation_detail() {
    let (app, pres_id, _storage, _dir) = setup_test_app().await;

    // 1. Add Lyrics
    let req_lyrics = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "type": "lyrics",
                "body": "Verse 1\nChorus"
            })
            .to_string(),
        ))
        .unwrap();

    let res_lyrics = app.clone().oneshot(req_lyrics).await.unwrap();
    assert_eq!(res_lyrics.status(), StatusCode::CREATED);
    let json_lyrics = body_to_json(res_lyrics.into_body()).await;
    assert_eq!(json_lyrics["type"], "lyrics");
    assert_eq!(json_lyrics["position"], 0);
    assert_eq!(json_lyrics["body"], "Verse 1\nChorus");
    let lyrics_id = json_lyrics["id"].as_str().unwrap().to_string();

    // 2. Add Note
    let req_note = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "type": "note",
                "body": "Switch guitar to drop D"
            })
            .to_string(),
        ))
        .unwrap();

    let res_note = app.clone().oneshot(req_note).await.unwrap();
    assert_eq!(res_note.status(), StatusCode::CREATED);
    let json_note = body_to_json(res_note.into_body()).await;
    assert_eq!(json_note["type"], "note");
    assert_eq!(json_note["position"], 1);

    // 3. GET /api/presentations/:id should now include both items in order
    let req_detail = Request::builder()
        .method("GET")
        .uri(format!("/api/presentations/{pres_id}"))
        .body(Body::empty())
        .unwrap();

    let res_detail = app.clone().oneshot(req_detail).await.unwrap();
    assert_eq!(res_detail.status(), StatusCode::OK);
    let json_detail = body_to_json(res_detail.into_body()).await;
    let items = json_detail["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], lyrics_id);
    assert_eq!(items[0]["position"], 0);
    assert_eq!(items[1]["position"], 1);
}

#[tokio::test]
async fn test_add_text_items_validation_and_not_found() {
    let (app, pres_id, _storage, _dir) = setup_test_app().await;

    // Empty body -> 422
    let req_empty = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "type": "lyrics", "body": "   " }).to_string(),
        ))
        .unwrap();
    let res_empty = app.clone().oneshot(req_empty).await.unwrap();
    assert_eq!(res_empty.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Invalid item type -> 422
    let req_bad_type = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "type": "unknown", "body": "Text" }).to_string(),
        ))
        .unwrap();
    let res_bad_type = app.clone().oneshot(req_bad_type).await.unwrap();
    assert_eq!(res_bad_type.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Non-existent presentation -> 404
    let req_not_found = Request::builder()
        .method("POST")
        .uri("/api/presentations/00000000-0000-0000-0000-000000000000/items")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "type": "lyrics", "body": "Text" }).to_string(),
        ))
        .unwrap();
    let res_not_found = app.oneshot(req_not_found).await.unwrap();
    assert_eq!(res_not_found.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_multipart_track_upload_success_and_validations() {
    let (app, pres_id, _storage, _dir) = setup_test_app().await;

    let boundary = "boundaryXYZ12345";
    // Valid MP3 magic bytes: ID3 header
    let mp3_bytes = [
        0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
    ];

    // 1. Success upload
    let body = build_multipart_body(boundary, "Intro Acoustic", "intro.mp3", &mp3_bytes);
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["type"], "track");
    assert_eq!(json["title"], "Intro Acoustic");
    assert_eq!(json["media_type"], "audio/mpeg");
    assert_eq!(json["position"], 0);
    let track_id = json["id"].as_str().unwrap();
    assert_eq!(json["audio_url"], format!("/api/tracks/{track_id}/audio"));

    // 2. Empty title -> 422
    let body_empty_title = build_multipart_body(boundary, "   ", "intro.mp3", &mp3_bytes);
    let req_empty_title = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_empty_title))
        .unwrap();
    let res_empty_title = app.clone().oneshot(req_empty_title).await.unwrap();
    assert_eq!(res_empty_title.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 3. Unsupported format (bad extension or bad magic bytes) -> 415
    let text_bytes = b"This is plain text not audio";
    let body_bad_format = build_multipart_body(boundary, "Bad Format", "intro.mp3", text_bytes);
    let req_bad_format = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_bad_format))
        .unwrap();
    let res_bad_format = app.clone().oneshot(req_bad_format).await.unwrap();
    assert_eq!(res_bad_format.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    // 4. Missing audio field -> 400
    let mut missing_audio_body = Vec::new();
    missing_audio_body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    missing_audio_body
        .extend_from_slice(b"Content-Disposition: form-data; name=\"title\"\r\n\r\nSolo\r\n");
    missing_audio_body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    let req_missing_audio = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(missing_audio_body))
        .unwrap();
    let res_missing = app.oneshot(req_missing_audio).await.unwrap();
    assert_eq!(res_missing.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_multipart_track_upload_oversized_returns_413() {
    let (app, pres_id, storage, _dir) = setup_test_app().await;

    let boundary = "boundaryLargeTest123";

    // Build a stream that sends header, followed by chunks exceeding 100 MiB (104,857,601 bytes)
    let header_part = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nBig Track\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"big.mp3\"\r\n\
         Content-Type: audio/mpeg\r\n\r\n"
    );

    use tokio::io::AsyncReadExt;
    use tokio_util::io::ReaderStream;

    let mut first_chunk = header_part.into_bytes();
    first_chunk.extend_from_slice(&[0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let prefix = std::io::Cursor::new(first_chunk);
    let zeroes = tokio::io::repeat(0).take(105 * 1024 * 1024);
    let chained = prefix.chain(zeroes);
    let stream = ReaderStream::new(chained);
    let body = Body::from_stream(stream);

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["error"]["code"], "file_too_large");

    // Ensure staging directory has no leftover parts
    assert_eq!(std::fs::read_dir(storage.staging_dir()).unwrap().count(), 0);
}

#[tokio::test]
async fn test_reorder_item_endpoint_and_boundary_conflict() {
    let (app, pres_id, _storage, _dir) = setup_test_app().await;

    // Create 2 items
    let req1 = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "type": "note", "body": "Note 0" }).to_string(),
        ))
        .unwrap();
    let res1 = app.clone().oneshot(req1).await.unwrap();
    let id0 = body_to_json(res1.into_body()).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let req2 = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "type": "note", "body": "Note 1" }).to_string(),
        ))
        .unwrap();
    let res2 = app.clone().oneshot(req2).await.unwrap();
    let id1 = body_to_json(res2.into_body()).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Move id0 Up -> boundary error 409
    let req_up_boundary = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items/{id0}/move"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "direction": "up" }).to_string()))
        .unwrap();
    let res_up_boundary = app.clone().oneshot(req_up_boundary).await.unwrap();
    assert_eq!(res_up_boundary.status(), StatusCode::CONFLICT);
    let json_boundary = body_to_json(res_up_boundary.into_body()).await;
    assert_eq!(json_boundary["error"]["code"], "reorder_boundary");

    // Move id1 Up -> 200 OK with items swapped
    let req_swap = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items/{id1}/move"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "direction": "up" }).to_string()))
        .unwrap();
    let res_swap = app.oneshot(req_swap).await.unwrap();
    assert_eq!(res_swap.status(), StatusCode::OK);
    let json_swap = body_to_json(res_swap.into_body()).await;
    let items = json_swap["items"].as_array().unwrap();
    assert_eq!(items[0]["id"], id1);
    assert_eq!(items[0]["position"], 0);
    assert_eq!(items[1]["id"], id0);
    assert_eq!(items[1]["position"], 1);
}

#[tokio::test]
async fn test_delete_item_endpoint() {
    let (app, pres_id, storage, _dir) = setup_test_app().await;

    // Upload track
    let boundary = "boundaryDelTest123";
    let mp3_bytes = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let body = build_multipart_body(boundary, "Track Del", "del.mp3", &mp3_bytes);
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    let track_id = json["id"].as_str().unwrap();

    // Verify audio file exists
    assert_eq!(std::fs::read_dir(storage.files_dir()).unwrap().count(), 1);

    // DELETE item
    let req_del = Request::builder()
        .method("DELETE")
        .uri(format!("/api/presentations/{pres_id}/items/{track_id}"))
        .body(Body::empty())
        .unwrap();

    let res_del = app.oneshot(req_del).await.unwrap();
    assert_eq!(res_del.status(), StatusCode::OK);
    let json_del = body_to_json(res_del.into_body()).await;
    assert_eq!(json_del["deleted_id"], track_id);
    assert_eq!(json_del["cleanup_pending"], false);
    assert_eq!(json_del["items"].as_array().unwrap().len(), 0);

    // Verify audio file deleted from disk
    assert_eq!(std::fs::read_dir(storage.files_dir()).unwrap().count(), 0);
}

struct ChannelReader {
    rx: tokio::sync::mpsc::Receiver<Result<Vec<u8>, std::io::ErrorKind>>,
    current: Option<Vec<u8>>,
    pos: usize,
}

impl tokio::io::AsyncRead for ChannelReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        loop {
            if let Some(ref bytes) = self.current {
                if self.pos < bytes.len() {
                    let to_copy = (bytes.len() - self.pos).min(buf.remaining());
                    buf.put_slice(&bytes[self.pos..self.pos + to_copy]);
                    self.pos += to_copy;
                    return std::task::Poll::Ready(Ok(()));
                }
                self.current = None;
                self.pos = 0;
            }
            match self.rx.poll_recv(cx) {
                std::task::Poll::Ready(Some(Ok(bytes))) => {
                    self.current = Some(bytes);
                    self.pos = 0;
                }
                std::task::Poll::Ready(Some(Err(kind))) => {
                    return std::task::Poll::Ready(Err(std::io::Error::new(kind, "drop")));
                }
                std::task::Poll::Ready(None) => return std::task::Poll::Ready(Ok(())),
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
    }
}

#[tokio::test]
async fn test_multipart_track_upload_interrupted_cleans_staging() {
    let (app, pres_id, storage, _dir) = setup_test_app().await;
    let boundary = "boundaryInterruptedTest123";
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    let mut part1 = Vec::new();
    part1.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nInterrupted\r\n"
        )
        .as_bytes(),
    );
    part1.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"test.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n").as_bytes());
    tx.send(Ok(part1)).await.unwrap();

    let stream = tokio_util::io::ReaderStream::new(ChannelReader {
        rx,
        current: None,
        pos: 0,
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from_stream(stream))
        .unwrap();

    let handle = tokio::spawn(async move { app.oneshot(req).await });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let _ = tx
        .send(Ok(vec![
            0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x34,
        ]))
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let _ = tx.send(Err(std::io::ErrorKind::ConnectionReset)).await;

    let res = handle.await.unwrap().unwrap();
    assert!(res.status().is_client_error() || res.status().is_server_error());
    assert_eq!(
        std::fs::read_dir(storage.staging_dir()).unwrap().count(),
        0,
        "No leftover staging files after interrupt"
    );
}

#[tokio::test]
async fn test_multipart_track_upload_duplicate_audio_rejected_and_cleaned() {
    let (app, pres_id, storage, _dir) = setup_test_app().await;
    let boundary = "boundaryDupTest123";
    let mp3 = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let mut body = Vec::new();
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nT1\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"1.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n").as_bytes());
    body.extend_from_slice(&mp3);
    body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"2.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n").as_bytes());
    body.extend_from_slice(&mp3);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        std::fs::read_dir(storage.staging_dir()).unwrap().count(),
        0,
        "No leftover staging files after duplicate audio"
    );
}

#[tokio::test]
async fn test_patch_item_endpoint_and_validations() {
    let (app, pres_id, _storage, _dir) = setup_test_app().await;

    // 1. Add a lyrics item
    let req_lyrics = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "type": "lyrics", "body": "Original Lyrics" }).to_string(),
        ))
        .unwrap();
    let res_lyrics = app.clone().oneshot(req_lyrics).await.unwrap();
    assert_eq!(res_lyrics.status(), StatusCode::CREATED);
    let lyrics_json = body_to_json(res_lyrics.into_body()).await;
    let lyrics_id = lyrics_json["id"].as_str().unwrap().to_string();

    // 2. Add a track item via raw DB for quick setup
    // Or upload track via endpoint
    let boundary = "boundaryPatchTest123";
    let mp3 = [0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let body_bytes = build_multipart_body(boundary, "Original Song", "song.mp3", &mp3);
    let req_track = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body_bytes))
        .unwrap();
    let res_track = app.clone().oneshot(req_track).await.unwrap();
    assert_eq!(res_track.status(), StatusCode::CREATED);
    let track_json = body_to_json(res_track.into_body()).await;
    let track_id = track_json["id"].as_str().unwrap().to_string();

    // 3. Success PATCH lyrics body -> 200
    let req_patch_lyrics = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{pres_id}/items/{lyrics_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "body": "Updated Lyrics Text" }).to_string(),
        ))
        .unwrap();
    let res_patch_lyrics = app.clone().oneshot(req_patch_lyrics).await.unwrap();
    assert_eq!(res_patch_lyrics.status(), StatusCode::OK);
    let patched_lyrics_json = body_to_json(res_patch_lyrics.into_body()).await;
    assert_eq!(patched_lyrics_json["type"], "lyrics");
    assert_eq!(patched_lyrics_json["body"], "Updated Lyrics Text");
    assert_eq!(patched_lyrics_json["id"], lyrics_id);

    // 4. Success PATCH track title -> 200
    let req_patch_track = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{pres_id}/items/{track_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "title": "Updated Song Title" }).to_string(),
        ))
        .unwrap();
    let res_patch_track = app.clone().oneshot(req_patch_track).await.unwrap();
    assert_eq!(res_patch_track.status(), StatusCode::OK);
    let patched_track_json = body_to_json(res_patch_track.into_body()).await;
    assert_eq!(patched_track_json["type"], "track");
    assert_eq!(patched_track_json["title"], "Updated Song Title");
    assert_eq!(patched_track_json["id"], track_id);

    // 5. 400 on empty payload
    let req_empty_payload = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{pres_id}/items/{lyrics_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let res_empty_payload = app.clone().oneshot(req_empty_payload).await.unwrap();
    assert_eq!(res_empty_payload.status(), StatusCode::BAD_REQUEST);

    // 6. 400 on whitespace value
    let req_ws = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{pres_id}/items/{lyrics_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "body": "   \n\t " }).to_string()))
        .unwrap();
    let res_ws = app.clone().oneshot(req_ws).await.unwrap();
    assert_eq!(res_ws.status(), StatusCode::BAD_REQUEST);

    // 7. 400 on incompatible field (body on track)
    let req_incompat_track = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{pres_id}/items/{track_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "body": "Trying to set body on track" }).to_string(),
        ))
        .unwrap();
    let res_incompat_track = app.clone().oneshot(req_incompat_track).await.unwrap();
    assert_eq!(res_incompat_track.status(), StatusCode::BAD_REQUEST);

    // 8. 400 on incompatible field (title on lyrics)
    let req_incompat_lyrics = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{pres_id}/items/{lyrics_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "title": "Trying to set title on lyrics" }).to_string(),
        ))
        .unwrap();
    let res_incompat_lyrics = app.clone().oneshot(req_incompat_lyrics).await.unwrap();
    assert_eq!(res_incompat_lyrics.status(), StatusCode::BAD_REQUEST);

    // 9. 404 on missing item or missing presentation
    let req_404_item = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/presentations/{pres_id}/items/missing-item-id"
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "body": "valid text" }).to_string()))
        .unwrap();
    let res_404_item = app.clone().oneshot(req_404_item).await.unwrap();
    assert_eq!(res_404_item.status(), StatusCode::NOT_FOUND);

    let req_404_pres = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/presentations/missing-pres-id/items/{lyrics_id}"
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "body": "valid text" }).to_string()))
        .unwrap();
    let res_404_pres = app.clone().oneshot(req_404_pres).await.unwrap();
    assert_eq!(res_404_pres.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_move_item_target_position_and_validations() {
    let (app, pres_id, _storage, _dir) = setup_test_app().await;

    // Add 3 notes: pos 0, 1, 2
    let mut item_ids = Vec::new();
    for title in ["Note 0", "Note 1", "Note 2"] {
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/presentations/{pres_id}/items"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "type": "note", "body": title }).to_string(),
            ))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let j = body_to_json(res.into_body()).await;
        item_ids.push(j["id"].as_str().unwrap().to_string());
    }

    // 1. Success move Note 2 to target_position 0 -> [Note 2, Note 0, Note 1]
    let req_move = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/presentations/{pres_id}/items/{}/move",
            item_ids[2]
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "target_position": 0 }).to_string()))
        .unwrap();
    let res_move = app.clone().oneshot(req_move).await.unwrap();
    assert_eq!(res_move.status(), StatusCode::OK);
    let moved_json = body_to_json(res_move.into_body()).await;
    let items = moved_json["items"].as_array().unwrap();
    assert_eq!(items[0]["id"], item_ids[2]);
    assert_eq!(items[0]["position"], 0);
    assert_eq!(items[1]["id"], item_ids[0]);
    assert_eq!(items[1]["position"], 1);
    assert_eq!(items[2]["id"], item_ids[1]);
    assert_eq!(items[2]["position"], 2);

    // 2. Target position out of bounds (< 0) -> 409
    let req_oob_neg = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/presentations/{pres_id}/items/{}/move",
            item_ids[0]
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "target_position": -1 }).to_string()))
        .unwrap();
    let res_oob_neg = app.clone().oneshot(req_oob_neg).await.unwrap();
    assert_eq!(res_oob_neg.status(), StatusCode::CONFLICT);

    // Target position out of bounds (>= N, here N=3, target=3) -> 409
    let req_oob_high = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/presentations/{pres_id}/items/{}/move",
            item_ids[0]
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "target_position": 3 }).to_string()))
        .unwrap();
    let res_oob_high = app.clone().oneshot(req_oob_high).await.unwrap();
    assert_eq!(res_oob_high.status(), StatusCode::CONFLICT);

    // 3. Ambiguity: both direction and target_position provided -> 400
    let req_both = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/presentations/{pres_id}/items/{}/move",
            item_ids[0]
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "direction": "up", "target_position": 0 }).to_string(),
        ))
        .unwrap();
    let res_both = app.clone().oneshot(req_both).await.unwrap();
    assert_eq!(res_both.status(), StatusCode::BAD_REQUEST);

    // 4. Neither direction nor target_position provided -> 400
    let req_neither = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/presentations/{pres_id}/items/{}/move",
            item_ids[0]
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let res_neither = app.clone().oneshot(req_neither).await.unwrap();
    assert_eq!(res_neither.status(), StatusCode::BAD_REQUEST);

    // 5. Retrocompatibility: direction "down" works
    // Currently order is [Note 2, Note 0, Note 1]. Move Note 2 down -> [Note 0, Note 2, Note 1]
    let req_retro = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/presentations/{pres_id}/items/{}/move",
            item_ids[2]
        ))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "direction": "down" }).to_string()))
        .unwrap();
    let res_retro = app.clone().oneshot(req_retro).await.unwrap();
    assert_eq!(res_retro.status(), StatusCode::OK);
    let retro_json = body_to_json(res_retro.into_body()).await;
    let retro_items = retro_json["items"].as_array().unwrap();
    assert_eq!(retro_items[0]["id"], item_ids[0]);
    assert_eq!(retro_items[1]["id"], item_ids[2]);
    assert_eq!(retro_items[2]["id"], item_ids[1]);
}
