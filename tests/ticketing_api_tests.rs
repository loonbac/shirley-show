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
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

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
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

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
    assert_eq!(
        json["resumen"]["desglose_gastos"]["sonido_luces"],
        500.0
    );

    // 7. DELETE /api/gastos/{id} -> 200
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/gastos/{gasto_id}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
