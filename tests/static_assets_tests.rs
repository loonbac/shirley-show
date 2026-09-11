use std::fs;
use std::path::PathBuf;

use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::router::create_router;
use shirley_show::state::AppState;
use tempfile::TempDir;
use tower::ServiceExt;

fn setup_test_env() -> (axum::Router, TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let frontend_dir = temp_dir.path().join("frontend");
    fs::create_dir_all(&frontend_dir).expect("create frontend dir");

    // Create a mock index.html
    fs::write(
        frontend_dir.join("index.html"),
        "<!DOCTYPE html><html><head><title>Test Shirley Show</title></head><body><div id=\"app\"></div></body></html>",
    )
    .expect("write index.html");

    // Create a mock asset in styles/
    let styles_dir = frontend_dir.join("styles");
    fs::create_dir_all(&styles_dir).expect("create styles dir");
    fs::write(styles_dir.join("index.css"), "body { margin: 0; }").expect("write index.css");

    // Create a mock asset in js/
    let js_dir = frontend_dir.join("js");
    fs::create_dir_all(&js_dir).expect("create js dir");
    fs::write(js_dir.join("main.js"), "console.log('shirley-show');").expect("write main.js");

    let mut state = AppState::new(pool);
    state.frontend_dir = frontend_dir.clone();

    let router = create_router(state);
    (router, temp_dir, frontend_dir)
}

#[tokio::test]
async fn test_serve_index_html_with_no_cache() {
    let (app, _dir, _frontend_dir) = setup_test_env();

    let req = Request::builder()
        .uri("/")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let cache_control = response
        .headers()
        .get(header::CACHE_CONTROL)
        .expect("cache-control header")
        .to_str()
        .unwrap();
    assert_eq!(cache_control, "no-cache");

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header")
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(body_str.contains("<title>Test Shirley Show</title>"));
    assert!(body_str.contains("<div id=\"app\"></div>"));
}

#[tokio::test]
async fn test_serve_static_assets_css_and_js() {
    let (app, _dir, _frontend_dir) = setup_test_env();

    // Test CSS asset
    let req_css = Request::builder()
        .uri("/assets/styles/index.css")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res_css = app.clone().oneshot(req_css).await.unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);

    let css_content_type = res_css
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .unwrap();
    assert!(css_content_type.contains("text/css"));

    let css_body = res_css.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        std::str::from_utf8(&css_body).unwrap(),
        "body { margin: 0; }"
    );

    // Test JS asset
    let req_js = Request::builder()
        .uri("/assets/js/main.js")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res_js = app.oneshot(req_js).await.unwrap();
    assert_eq!(res_js.status(), StatusCode::OK);

    let js_content_type = res_js
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .unwrap();
    assert!(js_content_type.contains("javascript"));

    let js_body = res_js.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        std::str::from_utf8(&js_body).unwrap(),
        "console.log('shirley-show');"
    );
}

#[tokio::test]
async fn test_missing_asset_returns_404() {
    let (app, _dir, _frontend_dir) = setup_test_env();

    let req = Request::builder()
        .uri("/assets/non_existent.png")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_missing_index_html_graceful_404() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).expect("init pool");
    migrate_pool(&pool).expect("migrate pool");

    let empty_frontend_dir = temp_dir.path().join("empty_frontend");
    fs::create_dir_all(&empty_frontend_dir).expect("create empty frontend dir");

    let mut state = AppState::new(pool);
    state.frontend_dir = empty_frontend_dir;

    let app = create_router(state);

    let req = Request::builder()
        .uri("/")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_routes_unaffected_by_asset_routing() {
    let (app, _dir, _frontend_dir) = setup_test_env();

    let req = Request::builder()
        .uri("/api/presentations")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
