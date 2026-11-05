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

#[tokio::test]
async fn test_events_api_crud_lifecycle() {
    let (app, _dir) = setup_app();

    // 1. GET /api/eventos -> empty
    let req = Request::builder()
        .method("GET")
        .uri("/api/eventos")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["eventos"], json!([]));

    // 2. POST /api/eventos with invalid payload -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/eventos")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "   ",
                "fecha_hora": "2026-10-15T21:00:00Z",
                "lugar": "La Noche",
                "tipos_de_entrada": []
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. POST /api/eventos with valid payload -> 201 Created
    let req = Request::builder()
        .method("POST")
        .uri("/api/eventos")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "  Concierto Acústico Barranco  ",
                "fecha_hora": "2026-10-15T21:00:00Z",
                "lugar": "  La Noche de Barranco  ",
                "tipos_de_entrada": [
                    { "nombre": "General", "precio": 30.0, "capacidad": 80 },
                    { "nombre": "VIP", "precio": 50.0, "capacidad": 20 }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let json = body_to_json(res.into_body()).await;
    let event_id = json["evento"]["id"].as_str().unwrap().to_string();
    assert_eq!(json["evento"]["nombre"], "Concierto Acústico Barranco");
    assert_eq!(json["evento"]["lugar"], "La Noche de Barranco");
    assert_eq!(json["evento"]["finalizado"], false);

    // 4. GET /api/eventos/{id} -> 200 OK
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["evento"]["id"], event_id);

    // 5. PATCH /api/eventos/{id} -> 200 OK
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/eventos/{event_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Concierto Acústico Barranco - 2da Función",
                "fecha_hora": "2026-10-16T21:00:00Z",
                "lugar": "La Noche de Barranco",
                "tipos_de_entrada": [
                    { "nombre": "General", "precio": 35.0, "capacidad": 100 }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(
        json["evento"]["nombre"],
        "Concierto Acústico Barranco - 2da Función"
    );

    // 6. POST /api/eventos/{id}/finalizar -> 200 OK
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/finalizar"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify finalized is true
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["evento"]["finalizado"], true);

    // 7. POST /api/eventos/{id}/reabrir -> 200 OK
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/reabrir"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify finalized is false
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["evento"]["finalizado"], false);

    // 8. DELETE /api/eventos/{id} -> 200 OK
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify 404
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_expenses_and_resumen_api() {
    let (app, _dir) = setup_app();

    // Create event
    let req = Request::builder()
        .method("POST")
        .uri("/api/eventos")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Show Barranco",
                "fecha_hora": "2026-10-15T21:00:00Z",
                "lugar": "La Noche",
                "tipos_de_entrada": [
                    { "nombre": "General", "precio": 30.0, "capacidad": 80 }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    let event_id = json["evento"]["id"].as_str().unwrap().to_string();

    // 1. GET /api/eventos/{id}/resumen (empty)
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/resumen"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_to_json(res.into_body()).await;
    let resumen = &json["resumen"];
    assert_eq!(resumen["cobrado"], 0.0);
    assert_eq!(resumen["pendiente"], 0.0);
    assert_eq!(resumen["total_proyectado"], 0.0);
    assert_eq!(resumen["gastos"], 0.0);
    assert_eq!(resumen["ganancia_neta"], 0.0);
    assert_eq!(resumen["capacidad_total"], 80);
    assert_eq!(resumen["entradas_emitidas"], 0);
    assert_eq!(resumen["aforo_proyectado"], 0);

    // 2. POST /api/eventos/{id}/gastos with invalid category -> 400
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/gastos"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "categoria": "Inexistente",
                "monto": 100.0,
                "descripcion": "Gasto",
                "fecha": "2026-10-10"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. POST /api/eventos/{id}/gastos valid -> 201
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/gastos"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "categoria": "Sonido/Luces",
                "monto": 450.0,
                "descripcion": "Consola y micrófonos",
                "fecha": "2026-10-14"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let json = body_to_json(res.into_body()).await;
    let gasto_id = json["gasto"]["id"].as_str().unwrap().to_string();
    assert_eq!(json["gasto"]["categoria"], "Sonido/Luces");
    assert_eq!(json["gasto"]["monto"], 450.0);

    // 4. GET /api/eventos/{id}/gastos -> list
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/gastos"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["gastos"].as_array().unwrap().len(), 1);

    // 5. PATCH /api/gastos/{id} -> 200
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/gastos/{gasto_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "categoria": "Sonido/Luces",
                "monto": 500.0,
                "descripcion": "Consola, microfonos y luces",
                "fecha": "2026-10-14"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["gasto"]["monto"], 500.0);

    // 6. GET /api/eventos/{id}/resumen reflects new expense
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/resumen"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let json = body_to_json(res.into_body()).await;
    assert_eq!(json["resumen"]["gastos"], 500.0);
    assert_eq!(json["resumen"]["ganancia_neta"], -500.0);
    assert_eq!(json["resumen"]["desglose_gastos"]["sonido_luces"], 500.0);

    // 7. DELETE /api/gastos/{id} -> 200
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/gastos/{gasto_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_tickets_and_checkin_api_endpoints() {
    let (app, _dir) = setup_app();

    // 1. Create an event
    let req = Request::builder()
        .method("POST")
        .uri("/api/eventos")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Noche Criolla",
                "fecha_hora": "2026-10-20T21:00:00Z",
                "lugar": "Barranco",
                "tipos_de_entrada": [
                    { "nombre": "General", "precio": 40.0, "capacidad": 20 },
                    { "nombre": "VIP", "precio": 70.0, "capacidad": 5 }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let event_json = body_to_json(res.into_body()).await;
    let event_id = event_json["evento"]["id"].as_str().unwrap();

    // 2a. Reject sale with quantity 0 -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/entradas"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Juan Perez",
                "contacto": "987000111",
                "tipo_entrada": "General",
                "cantidad": 0,
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2b. Reject sale with capacity exceeded -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/entradas"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Juan Perez",
                "contacto": "987000111",
                "tipo_entrada": "General",
                "cantidad": 100,
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 1b. POST /api/eventos/{id}/entradas with negative quantity (-2) -> MUST return 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/entradas"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Juan Perez",
                "contacto": "987000111",
                "tipo_entrada": "General",
                "cantidad": -2,
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err_json = body_to_json(res.into_body()).await;
    assert_eq!(err_json["error"]["code"], "invalid_request");

    // 1c. POST /api/eventos/{id}/entradas with wrong types -> MUST return 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/entradas"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Juan Perez",
                "contacto": "987000111",
                "tipo_entrada": "General",
                "cantidad": "diez",
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. POST /api/eventos/{id}/entradas -> sell 2 General paid tickets
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/entradas"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Juan Perez",
                "contacto": "987000111",
                "tipo_entrada": "General",
                "cantidad": 2,
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let sale_json = body_to_json(res.into_body()).await;
    let entradas = sale_json["entradas"].as_array().unwrap();
    assert_eq!(entradas.len(), 2);
    let ticket1_id = entradas[0]["id"].as_str().unwrap().to_string();
    let ticket2_id = entradas[1]["id"].as_str().unwrap().to_string();
    assert_eq!(entradas[0]["precio"], 40.0);
    assert_eq!(entradas[0]["estado_pago"], "Pagado");
    assert!(
        entradas[0]["ticket_path"]
            .as_str()
            .unwrap()
            .starts_with("#/ticket/")
    );
    assert!(entradas[0]["qr_svg"].is_string());

    // 3. GET /api/eventos/{id}/entradas
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/entradas?q=juan"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list_json = body_to_json(res.into_body()).await;
    assert_eq!(list_json["entradas"].as_array().unwrap().len(), 2);

    // 4. GET /api/entradas/{id}
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/entradas/{ticket1_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ticket_json = body_to_json(res.into_body()).await;
    assert_eq!(ticket_json["entrada"]["id"], ticket1_id);

    // 4b. PATCH /api/entradas/{id} with malformed JSON body -> MUST return 400 Bad Request
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/entradas/{ticket1_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": [1, 2, 3]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err_json = body_to_json(res.into_body()).await;
    assert_eq!(err_json["error"]["code"], "invalid_request");

    // 5. PATCH /api/entradas/{id} -> update contact
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/entradas/{ticket1_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Juan Carlos Perez",
                "contacto": "987999888"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let updated_json = body_to_json(res.into_body()).await;
    assert_eq!(
        updated_json["entrada"]["nombre_comprador"],
        "Juan Carlos Perez"
    );

    // 6. POST /api/eventos/{id}/entradas -> sell 1 Pending ticket
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/entradas"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre_comprador": "Maria Garcia",
                "contacto": "912345678",
                "tipo_entrada": "VIP",
                "cantidad": 1,
                "medio_pago": "Efectivo",
                "estado_pago": "Pendiente"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let pending_sale = body_to_json(res.into_body()).await;
    let pending_ticket_id = pending_sale["entradas"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let pending_scan_token = pending_sale["entradas"][0]["scan_token"]
        .as_str()
        .unwrap()
        .to_string();

    // 7. GET /api/public/ticket/{scan_token} for pending ticket -> no-store, qr_svg null
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/public/ticket/{pending_scan_token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("cache-control").unwrap(), "no-store");
    let pub_json = body_to_json(res.into_body()).await;
    assert_eq!(pub_json["estado_pago"], "Pendiente");
    assert!(pub_json["qr_svg"].is_null());

    // 8. POST /api/entradas/{id}/confirmar-pago
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/entradas/{pending_ticket_id}/confirmar-pago"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let confirmed_json = body_to_json(res.into_body()).await;
    assert_eq!(confirmed_json["entrada"]["estado_pago"], "Pagado");

    // 9. GET /api/public/ticket/{scan_token} serves the buyer's ticket with the
    // QR image they need to enter, but not the raw `codigo_qr` hex: the SVG is
    // the credential, the string is only a copyable leak.
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/public/ticket/{pending_scan_token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pub_paid_json = body_to_json(res.into_body()).await;
    assert_eq!(pub_paid_json["estado_pago"], "Pagado");
    assert!(
        pub_paid_json["qr_svg"].as_str().unwrap().contains("<svg"),
        "a paid ticket shows its buyer the QR: it is the entry credential"
    );
    assert!(
        pub_paid_json.get("codigo_qr").is_none(),
        "but the raw hex secret must not appear: the SVG is enough to scan, \
         the string is only a copyable leak"
    );

    // 9b. POST /api/entradas/checkin with invalid/malformed JSON body -> MUST return 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/entradas/checkin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "evento_id": 12345,
                "codigo": false
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err_json = body_to_json(res.into_body()).await;
    assert_eq!(err_json["error"]["code"], "invalid_request");

    // 10. POST /api/entradas/checkin with unknown code -> Invalida
    let req = Request::builder()
        .method("POST")
        .uri("/api/entradas/checkin")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "evento_id": event_id,
                "codigo": "ffff0000ffff0000ffff0000ffff0000"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let checkin_json = body_to_json(res.into_body()).await;
    assert_eq!(checkin_json["resultado"], "Invalida");

    // 11. DELETE unconsumed ticket2 -> 204 No Content
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/entradas/{ticket2_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

fn build_poster_multipart_body(
    boundary: &str,
    filename: &str,
    file_bytes: &[u8],
    content_type: &str,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"imagen\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

#[tokio::test]
async fn test_interests_and_posters_admin_endpoints() {
    let (app, _dir) = setup_app();

    // 1. Create event
    let req = Request::builder()
        .method("POST")
        .uri("/api/eventos")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Festival Andino",
                "fecha_hora": "2026-11-10T19:00:00Z",
                "lugar": "Cusco",
                "tipos_de_entrada": [
                    { "nombre": "General", "precio": 40.0, "capacidad": 100 }
                ]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let event_json = body_to_json(res.into_body()).await;
    let event_id = event_json["evento"]["id"].as_str().unwrap().to_string();

    // 2. POST /api/eventos/{id}/interesados -> 201 Created
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/interesados"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Elena Gomez",
                "contacto": "+51999888777",
                "canal": "WhatsApp",
                "promotora": "Shirley",
                "personas": 4
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let lead_json = body_to_json(res.into_body()).await;
    let lead_id = lead_json["interesado"]["id"].as_str().unwrap().to_string();
    assert_eq!(lead_json["interesado"]["nombre"], "Elena Gomez");
    assert_eq!(lead_json["interesado"]["estado"], "Pendiente");

    // 3. GET /api/eventos/{id}/interesados -> 200 OK with 1 lead
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/interesados"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list_json = body_to_json(res.into_body()).await;
    assert_eq!(list_json["interesados"].as_array().unwrap().len(), 1);

    // 4. GET /api/interesados/{id} -> 200 OK
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/interesados/{lead_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let get_json = body_to_json(res.into_body()).await;
    assert_eq!(get_json["interesado"]["id"], lead_id);

    // 5. PATCH /api/interesados/{id} -> 200 OK (update to Confirmado)
    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/interesados/{lead_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Elena Gomez Sanchez",
                "contacto": "+51999888777",
                "canal": "WhatsApp",
                "promotora": "Shirley",
                "personas": 5,
                "estado": "Confirmado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let updated_json = body_to_json(res.into_body()).await;
    assert_eq!(updated_json["interesado"]["estado"], "Confirmado");
    assert_eq!(updated_json["interesado"]["personas"], 5);

    // 6. POST /api/interesados/{id}/vender -> 201 Created (convert lead to sale)
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/interesados/{lead_id}/vender"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "tipo_entrada": "General",
                "cantidad": 5,
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let sale_json = body_to_json(res.into_body()).await;
    let tickets = sale_json["entradas"].as_array().unwrap();
    assert_eq!(tickets.len(), 5);

    // 7. POST /api/interesados/{id}/vender again -> 409 Conflict
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/interesados/{lead_id}/vender"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "tipo_entrada": "General",
                "cantidad": 5,
                "medio_pago": "Yape",
                "estado_pago": "Pagado"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // 7b. Upload poster with forbidden image/jpg MIME -> MUST return 415 Unsupported Media Type
    let boundary_jpg = "----WebKitFormBoundaryJpg123";
    let sample_jpeg: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    let body_jpg =
        build_poster_multipart_body(boundary_jpg, "poster.jpg", sample_jpeg, "image/jpg");
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/imagen"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary_jpg}"),
        )
        .body(Body::from(body_jpg))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "image/jpg must be rejected (only image/jpeg is allowed)"
    );

    // 7c. Upload poster with discordant MIME (PNG bytes declared as image/jpeg) -> MUST return 415
    let boundary_mismatch = "----WebKitFormBoundaryMis123";
    let sample_png: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let body_mismatch =
        build_poster_multipart_body(boundary_mismatch, "poster.png", sample_png, "image/jpeg");
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/imagen"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary_mismatch}"),
        )
        .body(Body::from(body_mismatch))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Discordant MIME must return 415"
    );

    // 8. Upload poster via multipart POST /api/eventos/{id}/imagen -> 200 OK
    let boundary = "----WebKitFormBoundaryXyZ123";
    let sample_jpeg: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    let body = build_poster_multipart_body(boundary, "poster.jpg", sample_jpeg, "image/jpeg");
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/eventos/{event_id}/imagen"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let poster_json = body_to_json(res.into_body()).await;
    let imagen_filename = poster_json["imagen"].as_str().unwrap().to_string();
    assert!(imagen_filename.ends_with(".jpg"));

    // 9. GET /api/eventos/{id}/imagen -> 200 OK bytes with headers
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/imagen"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/jpeg"
    );
    assert!(
        res.headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("public")
    );
    assert!(res.headers().contains_key(header::ETAG));
    let img_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&img_bytes[..], sample_jpeg);

    // 10. DELETE /api/eventos/{id} -> 200 OK (deletes event, cascades child records, and deletes poster)
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Confirm GET /api/eventos/{id} is now 404
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Confirm GET /api/eventos/{id}/imagen is now 404
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/eventos/{event_id}/imagen"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
