use axum::body::Body;
use http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::router::create_router;
use shirley_show::state::AppState;
use tempfile::TempDir;
use tower::ServiceExt;

fn setup_app() -> (axum::Router, TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("test.sqlite3");
    let path = db_path.to_str().expect("valid path");
    let pool = init_pool(path).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");
    let state = AppState::new(pool);
    (create_router(state), dir)
}

async fn body_to_json(body: Body) -> Value {
    let bytes = body.collect().await.expect("collect body").to_bytes();
    serde_json::from_slice(&bytes).expect("parse json")
}

#[tokio::test]
async fn test_get_presentations_empty() {
    let (app, _dir) = setup_app();

    let req = Request::builder()
        .method("GET")
        .uri("/api/presentations")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["presentations"], json!([]));
}

#[tokio::test]
async fn test_create_and_get_presentation() {
    let (app, _dir) = setup_app();

    // POST /api/presentations
    let req = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "  Viernes Acústico  " }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let json = body_to_json(res.into_body()).await;
    let presentation = &json["presentation"];
    assert_eq!(presentation["title"], "Viernes Acústico");
    let id = presentation["id"].as_str().unwrap().to_string();

    // GET /api/presentations/:id
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/presentations/{id}"))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["presentation"]["id"], id);
    assert_eq!(json["presentation"]["title"], "Viernes Acústico");
    assert_eq!(json["items"], json!([]));
}

#[tokio::test]
async fn test_patch_presentation_rename() {
    let (app, _dir) = setup_app();

    // Create presentation
    let req = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Initial" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    let id = json["presentation"]["id"].as_str().unwrap();

    // PATCH /api/presentations/:id
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Renamed Title" }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["presentation"]["title"], "Renamed Title");
    assert_eq!(json["presentation"]["id"], id);
}

#[tokio::test]
async fn test_delete_presentation() {
    let (app, _dir) = setup_app();

    // Create presentation
    let req = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "To Delete" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    let id = json["presentation"]["id"].as_str().unwrap();

    // DELETE /api/presentations/:id
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/presentations/{id}"))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["deleted_id"], id);
    assert_eq!(json["cleanup_pending"], false);

    // GET /api/presentations/:id should now return 404
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/presentations/{id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_error_response_format_and_status_codes() {
    let (app, _dir) = setup_app();

    // 400 Bad Request on malformed JSON
    let req_bad_json = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{ invalid_json: "))
        .unwrap();
    let res_bad_json = app.clone().oneshot(req_bad_json).await.unwrap();
    assert_eq!(res_bad_json.status(), StatusCode::BAD_REQUEST);
    let json_bad = body_to_json(res_bad_json.into_body()).await;
    assert!(json_bad.get("error").is_some());
    assert!(json_bad["error"].get("code").is_some());
    assert!(json_bad["error"].get("message").is_some());
    assert!(json_bad["error"].as_object().unwrap().contains_key("field"));

    // 404 Not Found on missing presentation
    let req_not_found = Request::builder()
        .method("GET")
        .uri("/api/presentations/non-existent-uuid")
        .body(Body::empty())
        .unwrap();
    let res_not_found = app.clone().oneshot(req_not_found).await.unwrap();
    assert_eq!(res_not_found.status(), StatusCode::NOT_FOUND);
    let json_404 = body_to_json(res_not_found.into_body()).await;
    assert_eq!(json_404["error"]["code"], "presentation_not_found");
    assert!(json_404["error"].as_object().unwrap().contains_key("field"));

    // 422 Unprocessable Entity on empty title in POST
    let req_empty_post = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "   " }).to_string()))
        .unwrap();
    let res_empty_post = app.clone().oneshot(req_empty_post).await.unwrap();
    assert_eq!(res_empty_post.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json_422_post = body_to_json(res_empty_post.into_body()).await;
    assert_eq!(json_422_post["error"]["code"], "title_required");
    assert_eq!(json_422_post["error"]["field"], "title");

    // 422 Unprocessable Entity on empty title in PATCH
    let req_create = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Valid" }).to_string()))
        .unwrap();
    let res_create = app.clone().oneshot(req_create).await.unwrap();
    let id = body_to_json(res_create.into_body()).await["presentation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let req_empty_patch = Request::builder()
        .method("PATCH")
        .uri(format!("/api/presentations/{id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "" }).to_string()))
        .unwrap();
    let res_empty_patch = app.oneshot(req_empty_patch).await.unwrap();
    assert_eq!(res_empty_patch.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json_422_patch = body_to_json(res_empty_patch.into_body()).await;
    assert_eq!(json_422_patch["error"]["code"], "title_required");
    assert_eq!(json_422_patch["error"]["field"], "title");
}


fn setup_app_with_storage() -> (axum::Router, std::sync::Arc<shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage>, shirley_show::infra::sqlite::connect::DbPool, TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let pool = init_pool(dir.path().join("test.sqlite3").to_str().unwrap()).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");
    let storage = std::sync::Arc::new(shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage::new(dir.path().join("audio")));
    let state = AppState::new_with_items(pool.clone(), storage.clone());
    (create_router(state), storage, pool, dir)
}

#[tokio::test]
async fn test_delete_presentation_with_cascade_audio_file_cleanup() {
    let (app, storage, pool, _dir) = setup_app_with_storage();

    // 1. Create presentation
    let req = Request::builder().method("POST").uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Concert Show" }).to_string())).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let pres_id = body_to_json(res.into_body()).await["presentation"]["id"].as_str().unwrap().to_string();

    // 2. Upload track with audio file
    let boundary = "testDelCascadeBoundary123";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nSolo\r\n").as_bytes());
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"solo.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n").as_bytes());
    body.extend_from_slice(&[0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB]);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let req_upload = Request::builder().method("POST").uri(format!("/api/presentations/{pres_id}/tracks"))
        .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body)).unwrap();
    let res_upload = app.clone().oneshot(req_upload).await.unwrap();
    assert_eq!(res_upload.status(), StatusCode::CREATED);
    let item_id = body_to_json(res_upload.into_body()).await["id"].as_str().unwrap().to_string();

    let conn = pool.get().unwrap();
    let audio_key: String = conn.query_row("SELECT audio_key FROM show_items WHERE id = ?1;", [&item_id], |r| r.get(0)).unwrap();
    let audio_file_path = storage.resolve_path_str(&audio_key).expect("resolve path");
    assert!(audio_file_path.exists(), "Audio file must exist before presentation deletion");

    // 3. Delete presentation
    let req_del = Request::builder().method("DELETE").uri(format!("/api/presentations/{pres_id}")).body(Body::empty()).unwrap();
    let res_del = app.clone().oneshot(req_del).await.unwrap();
    assert_eq!(res_del.status(), StatusCode::OK);
    let del_json = body_to_json(res_del.into_body()).await;
    assert_eq!(del_json["deleted_id"], pres_id);
    assert_eq!(del_json["cleanup_pending"], false);

    // 4. Assert physical audio file is deleted post-commit & DB cascade clean
    assert!(!audio_file_path.exists(), "Audio file must be physically removed from storage");
    let items_count: i64 = conn.query_row("SELECT count(*) FROM show_items WHERE presentation_id = ?1;", [&pres_id], |r| r.get(0)).unwrap();
    assert_eq!(items_count, 0, "Show items must be cascade deleted in DB");
    let pending_count: i64 = conn.query_row("SELECT count(*) FROM pending_audio_deletions WHERE audio_key = ?1;", [&audio_key], |r| r.get(0)).unwrap();
    assert_eq!(pending_count, 0, "Pending queue must be 0 after successful physical cleanup");
}

#[tokio::test]
async fn test_get_presentations_enriched_summary() {
    let (app, _dir) = setup_app();

    // 1. Create a presentation with 0 items
    let req1 = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Empty Presentation" }).to_string()))
        .unwrap();
    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::CREATED);
    let pres1_id = body_to_json(res1.into_body()).await["presentation"]["id"].as_str().unwrap().to_string();

    // 2. Create second presentation and add items
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/presentations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "title": "Rich Presentation" }).to_string()))
        .unwrap();
    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::CREATED);
    let pres2_id = body_to_json(res2.into_body()).await["presentation"]["id"].as_str().unwrap().to_string();

    // Add note and lyrics to pres2
    let req_item1 = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres2_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "type": "note", "body": "Opening Cue" }).to_string()))
        .unwrap();
    app.clone().oneshot(req_item1).await.unwrap();

    let req_item2 = Request::builder()
        .method("POST")
        .uri(format!("/api/presentations/{pres2_id}/items"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "type": "lyrics", "body": "First verse lyrics" }).to_string()))
        .unwrap();
    app.clone().oneshot(req_item2).await.unwrap();

    // 3. GET /api/presentations
    let req_list = Request::builder()
        .method("GET")
        .uri("/api/presentations")
        .body(Body::empty())
        .unwrap();
    let res_list = app.oneshot(req_list).await.unwrap();
    assert_eq!(res_list.status(), StatusCode::OK);
    let list_json = body_to_json(res_list.into_body()).await;

    let presentations = list_json["presentations"].as_array().unwrap();
    assert_eq!(presentations.len(), 2);

    let p1 = presentations.iter().find(|p| p["id"] == pres1_id).unwrap();
    assert_eq!(p1["title"], "Empty Presentation");
    assert_eq!(p1["tracks_count"], 0);
    assert_eq!(p1["lyrics_count"], 0);
    assert_eq!(p1["notes_count"], 0);
    assert_eq!(p1["total_items"], 0);
    assert_eq!(p1["preview_items"].as_array().unwrap().len(), 0);

    let p2 = presentations.iter().find(|p| p["id"] == pres2_id).unwrap();
    assert_eq!(p2["title"], "Rich Presentation");
    assert_eq!(p2["tracks_count"], 0);
    assert_eq!(p2["lyrics_count"], 1);
    assert_eq!(p2["notes_count"], 1);
    assert_eq!(p2["total_items"], 2);
    let previews = p2["preview_items"].as_array().unwrap();
    assert_eq!(previews.len(), 2);
    assert_eq!(previews[0]["type"], "note");
    assert_eq!(previews[0]["snippet"], "Opening Cue");
    assert_eq!(previews[1]["type"], "lyrics");
    assert_eq!(previews[1]["snippet"], "First verse lyrics");
}
