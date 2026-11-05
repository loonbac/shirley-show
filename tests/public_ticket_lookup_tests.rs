//! The buyer arrives with whatever their ticket carries, and must see it.
//!
//! The buyer is never sent a link: they are sent the photo of their ticket, and
//! the QR inside it encodes `<base>/#/ticket/<codigo_qr>`. So the 32-hex door
//! secret is the only token that reaches `/api/public/ticket/` from a phone
//! camera. An earlier version indexed that route by `scan_token` alone, which
//! answered 404 for exactly the credential the product hands out -- the buyer
//! scanned their own ticket and saw the landing page with no ticket on it.

use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
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

async fn post_json(app: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    (status, body_to_json(res.into_body()).await)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    (status, body_to_json(res.into_body()).await)
}

/// Creates one paid ticket and returns `(codigo_qr, scan_token)`.
async fn sell_paid_ticket(app: &axum::Router) -> (String, String) {
    let (status, event) = post_json(
        app,
        "/api/eventos",
        json!({
            "nombre": "Shirley Show",
            "fecha_hora": "2027-01-10T23:00:00Z",
            "lugar": "Lima",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 20.0, "capacidad": 100 }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "event created: {event}");
    let evento_id = event["evento"]["id"].as_str().expect("event id").to_string();

    let (status, sale) = post_json(
        app,
        &format!("/api/eventos/{evento_id}/entradas"),
        json!({
            "nombre_comprador": "Mayela Salinas",
            "contacto": "999999999",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Efectivo",
            "estado_pago": "Pagado"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "ticket sold: {sale}");

    let entrada = &sale["entradas"][0];
    (
        entrada["codigo_qr"].as_str().expect("codigo_qr").to_string(),
        entrada["scan_token"].as_str().expect("scan_token").to_string(),
    )
}

#[tokio::test]
async fn test_public_ticket_resolves_the_door_secret_printed_in_the_qr() {
    let (app, _dir) = setup_app();
    let (codigo_qr, _scan_token) = sell_paid_ticket(&app).await;

    // This is the request a phone makes after scanning the printed QR.
    let (status, body) = get(&app, &format!("/api/public/ticket/{codigo_qr}")).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "the QR the buyer owns must open their own ticket, got: {body}"
    );
    assert_eq!(body["nombre_comprador"], "Mayela Salinas");
    assert_eq!(body["estado_pago"], "Pagado");
    assert!(
        body["qr_svg"].as_str().is_some_and(|s| s.contains("<svg")),
        "a paid ticket shows the QR the door will scan"
    );
    assert!(
        body.get("codigo_qr").is_none(),
        "the raw hex must still not be echoed back: the SVG is the credential"
    );
}

#[tokio::test]
async fn test_public_ticket_still_resolves_the_buyer_link_token() {
    // Existing behaviour, pinned so the fallback above cannot be implemented by
    // dropping the link token instead of adding the door secret.
    let (app, _dir) = setup_app();
    let (_codigo_qr, scan_token) = sell_paid_ticket(&app).await;

    let (status, body) = get(&app, &format!("/api/public/ticket/{scan_token}")).await;

    assert_eq!(status, StatusCode::OK, "buyer link keeps working: {body}");
    assert_eq!(body["nombre_comprador"], "Mayela Salinas");
    assert!(body["qr_svg"].as_str().is_some_and(|s| s.contains("<svg")));
}

#[tokio::test]
async fn test_public_ticket_rejects_a_token_that_belongs_to_nobody() {
    // Negative control: without it, the two tests above would also pass if the
    // route answered every string with the first ticket in the database.
    let (app, _dir) = setup_app();
    let _ = sell_paid_ticket(&app).await;

    let (status, body) = get(&app, "/api/public/ticket/00000000000000000000000000000000").await;

    assert_eq!(status, StatusCode::NOT_FOUND, "unknown secret: {body}");
    assert_eq!(body["error"]["code"], "ticket_not_found");
}

#[tokio::test]
async fn test_public_ticket_by_door_secret_does_not_leak_another_tickets_data() {
    // Two tickets, two secrets: each one must resolve to its own buyer.
    let (app, _dir) = setup_app();
    let (first_qr, _) = sell_paid_ticket(&app).await;
    let (second_qr, _) = sell_paid_ticket(&app).await;

    assert_ne!(first_qr, second_qr, "each ticket gets its own door secret");

    let (status, body) = get(&app, &format!("/api/public/ticket/{second_qr}")).await;
    assert_eq!(status, StatusCode::OK, "second ticket resolves: {body}");
    assert_eq!(body["nombre_comprador"], "Mayela Salinas");
}
