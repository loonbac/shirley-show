//! The door host must reach the door, and nothing else.
//!
//! The door runs on its own subdomain behind its own Access application, and it
//! is the only surface besides the panel that can **write**: it sells tickets at
//! the entrance and marks check-ins. Sharing a host with the panel would give
//! whoever holds the door session the whole management application, and a door
//! session is long by design because it must not expire mid-event.
//!
//! So the door gets a positive allowlist, written from what `door-controller.js`
//! actually calls, and everything else on `/api` is refused there.

use shirley_show::config::{PublicSurface, SurfaceConfig};

fn three_hosts() -> SurfaceConfig {
    SurfaceConfig::new_three(
        Some("https://shirleyshow.site"),
        Some("https://panel.shirleyshow.site"),
        Some("https://puerta.shirleyshow.site"),
    )
}

#[test]
fn test_door_host_is_its_own_surface() {
    let cfg = three_hosts();
    assert_eq!(
        cfg.classify(Some("puerta.shirleyshow.site")),
        PublicSurface::Door,
        "the door host must be its own surface, not the panel"
    );
}

#[test]
fn test_door_reaches_exactly_what_the_door_controller_calls() {
    let cfg = three_hosts();

    // Read from `frontend/js/modules/events/door-controller.js`: four calls,
    // two of which write. Missing one of these breaks the door during an event.
    for (method, path) in [
        ("GET", "/api/eventos/6c6d16b3-6672-4111-a00d-28c5ce0565b8"),
        (
            "GET",
            "/api/eventos/6c6d16b3-6672-4111-a00d-28c5ce0565b8/resumen",
        ),
        (
            "GET",
            "/api/eventos/6c6d16b3-6672-4111-a00d-28c5ce0565b8/imagen",
        ),
        ("POST", "/api/entradas/checkin"),
        (
            "POST",
            "/api/eventos/6c6d16b3-6672-4111-a00d-28c5ce0565b8/entradas",
        ),
        ("GET", "/api/public/ticket/st_abc123"),
    ] {
        assert!(
            cfg.door_allows(method, path),
            "the door must be able to {method} {path}: the door controller calls it"
        );
    }
}

#[test]
fn test_door_cannot_reach_the_rest_of_the_application() {
    let cfg = three_hosts();

    // Everything the door has no business with. The agent chat is the sharpest
    // case: it spends paid tokens and has no auth layer of its own.
    for (method, path) in [
        ("GET", "/api/presentations"),
        ("POST", "/api/agent/chat"),
        ("POST", "/api/agent/gpu-status"),
        ("GET", "/api/eventos"),
        ("POST", "/api/eventos"),
        ("GET", "/api/eventos/abc/gastos"),
        ("PATCH", "/api/eventos/abc"),
        ("DELETE", "/api/eventos/abc"),
        ("POST", "/api/eventos/abc/finalizar"),
        ("GET", "/api/eventos/abc/interesados"),
        ("POST", "/api/entradas/abc/confirmar-pago"),
        ("GET", "/api/tracks/abc/audio"),
    ] {
        assert!(
            !cfg.door_allows(method, path),
            "the door must not reach {method} {path}"
        );
    }
}

#[test]
fn test_door_refuses_reads_on_write_routes_and_the_reverse() {
    let cfg = three_hosts();

    // A method mismatch must not slip through: allowing `GET /api/entradas/checkin`
    // would be harmless today and wrong the moment a route answers both.
    assert!(!cfg.door_allows("GET", "/api/entradas/checkin"));
    assert!(!cfg.door_allows("POST", "/api/eventos/abc", ));
    assert!(cfg.door_allows("POST", "/api/eventos/abc/entradas"));
    assert!(!cfg.door_allows("GET", "/api/eventos/abc/entradas"));
}

#[test]
fn test_panel_and_landing_are_unaffected_by_the_door_rules() {
    let cfg = three_hosts();

    assert_eq!(
        cfg.classify(Some("panel.shirleyshow.site")),
        PublicSurface::Application
    );
    assert_eq!(
        cfg.classify(Some("shirleyshow.site")),
        PublicSurface::Landing
    );

    // The panel keeps the whole application; the landing keeps its own routes.
    assert!(PublicSurface::Application.serves_api());
    assert!(!PublicSurface::Landing.serves_api());
    assert!(!PublicSurface::Door.serves_api());
}

// The tests above prove the allowlist. This proves the wiring, which is where
// the previous guard bug lived: the rule was right and the wiring was wrong.

mod wiring {
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use shirley_show::config::{Config, SurfaceConfig};
    use shirley_show::router::create_router;
    use shirley_show::state::AppState;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn app() -> (axum::Router, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir");
        let audio_dir = temp_dir.path().join("audio");
        std::fs::create_dir_all(audio_dir.join("files")).unwrap();
        std::fs::create_dir_all(audio_dir.join("staging")).unwrap();
        let pool = shirley_show::infra::sqlite::init_pool(
            temp_dir.path().join("t.sqlite3").to_str().unwrap(),
        )
        .unwrap();
        shirley_show::infra::sqlite::migrate_pool(&pool).unwrap();

        let frontend = temp_dir.path().join("frontend");
        std::fs::create_dir_all(frontend.join("landing")).unwrap();
        std::fs::write(frontend.join("index.html"), "<html>panel</html>").unwrap();
        std::fs::write(frontend.join("landing/index.html"), "<html>landing</html>").unwrap();

        let config = Config {
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            data_dir: temp_dir.path().join("data"),
            frontend_dir: frontend,
            groq: Default::default(),
            yt_dlp_bin: None,
            agent_timeout_secs: 120,
            separation_timeout_secs: 600,
            remote_separation_timeout_secs: 1500,
            joshua_pc: None,
            surface: SurfaceConfig::new_three(
                Some("https://shirleyshow.site"),
                Some("https://panel.shirleyshow.site"),
                Some("https://puerta.shirleyshow.site"),
            ),
        };
        std::fs::create_dir_all(&config.data_dir).unwrap();

        let state = AppState::new_with_all_components_and_config(
            pool,
            std::sync::Arc::new(shirley_show::infra::filesystem::LocalAudioStorage::new(
                audio_dir,
            )),
            std::sync::Arc::new(
                shirley_show::state::staged_audio_registry::StagedAudioRegistry::new(),
            ),
            std::sync::Arc::new(
                shirley_show::infra::youtube::ProcessYoutubeDownloader::new(Default::default()),
            ),
            &config,
        );
        (create_router(state), temp_dir)
    }

    /// Returns the status and whether the surface guard, not the handler,
    /// produced it. Both answer 404 by design, so the status alone cannot tell
    /// "the door cannot do this" from "this event does not exist".
    async fn probe(
        app: axum::Router,
        host: &str,
        method: &str,
        path: &str,
    ) -> (StatusCode, bool) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, host)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let refused = response.headers().contains_key("x-surface-refused");
        (response.status(), refused)
    }

    async fn status(app: axum::Router, host: &str, method: &str, path: &str) -> StatusCode {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, host)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::empty())
            .unwrap();
        app.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn test_door_host_refuses_the_agent_chat() {
        let (app, _tmp) = app();
        // The sharpest case: `/api/agent/chat` spends paid tokens.
        assert_eq!(
            status(app, "puerta.shirleyshow.site", "POST", "/api/agent/chat").await,
            StatusCode::NOT_FOUND,
            "the door must never reach the paid agent endpoint"
        );
    }

    #[tokio::test]
    async fn test_door_host_refuses_the_management_routes() {
        let (app, _tmp) = app();
        for (method, path) in [
            ("GET", "/api/presentations"),
            ("GET", "/api/eventos"),
            ("GET", "/api/tracks/abc/audio"),
        ] {
            assert_eq!(
                status(app.clone(), "puerta.shirleyshow.site", method, path).await,
                StatusCode::NOT_FOUND,
                "the door must not reach {method} {path}"
            );
        }
    }

    #[tokio::test]
    async fn test_door_host_reaches_its_own_routes() {
        let (app, _tmp) = app();
        // 200 or a domain error, but never the guard's 404: the request reached
        // the handler. A 404 here would mean the door cannot work at all.
        for (method, path) in [
            ("GET", "/api/eventos/abc"),
            ("GET", "/api/eventos/abc/resumen"),
            ("POST", "/api/entradas/checkin"),
            ("POST", "/api/eventos/abc/entradas"),
        ] {
            let (got, refused_by_guard) = probe(app.clone(), "puerta.shirleyshow.site", method, path).await;
            assert!(
                !refused_by_guard,
                "the door must be able to {method} {path}, but the surface guard \
                 refused it (status {got}): a handler 404 is fine, a guard 404 is not"
            );
        }
    }

    #[tokio::test]
    async fn test_panel_host_still_reaches_everything() {
        let (app, _tmp) = app();
        assert_eq!(
            status(app.clone(), "panel.shirleyshow.site", "GET", "/api/presentations").await,
            StatusCode::OK
        );
        // And the panel may also use the door routes, since it hosts the mode.
        let (_, refused) =
            probe(app, "panel.shirleyshow.site", "GET", "/api/eventos/abc").await;
        assert!(!refused, "the panel must reach the door routes too");
    }

#[tokio::test]
async fn test_the_landing_may_read_a_public_ticket_and_nothing_else() {
    // The buyer receives a WhatsApp link like `https://shirleyshow.site/#/ticket/st_...`.
    // The hash never reaches the server, so the landing must fetch the ticket
    // itself. That means the public host needs exactly one API route: the public
    // ticket read. Opening `/api/` on the landing would also open
    // `/api/agent/chat`, which spends paid tokens on anonymous calls.
    let (app, _tmp) = app();

    let (status, refused) =
        probe(app.clone(), "shirleyshow.site", "GET", "/api/public/ticket/st_abc").await;
    assert!(
        !refused,
        "the landing must be able to read a public ticket (status {status}): the \
         buyer's proof of purchase is shown by the landing itself"
    );

    // And still nothing else, in particular not the paid endpoint.
    for (method, path) in [
        ("POST", "/api/agent/chat"),
        ("GET", "/api/presentations"),
        ("GET", "/api/eventos"),
        ("POST", "/api/entradas/checkin"),
    ] {
        let (_, refused) = probe(app.clone(), "shirleyshow.site", method, path).await;
        assert!(
            refused,
            "the landing must still be refused {method} {path}: only the public \
             ticket read is open on the anonymous host"
        );
    }

    // Path traversal needs no case here: `curl` and the browser percent-encode
    // or the router normalises `..` before any layer sees it, and the prefix
    // match cannot be fooled by a path that still starts with the literal
    // prefix. Measured rather than assumed: a raw `..` reaches axum's own
    // rejection with an empty body and no `x-surface-refused` header, so it never
    // becomes `/api/agent/chat`.
}

    #[tokio::test]
    async fn test_the_panel_host_serves_the_panel_at_its_root() {
        // `panel.shirleyshow.site` is the address the owner is told to use. If it
        // answers with the public landing, the one host that is supposed to be
        // the way in looks like the site they already had, and they have to know
        // the `/panel` path to get anywhere.
        let (app, _tmp) = app();

        let request = Request::builder()
            .uri("/")
            .header(header::HOST, "panel.shirleyshow.site")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);

        assert!(
            html.contains("/assets/js/main.js"),
            "the panel host must serve the panel at `/`, not the public landing"
        );

        // And `/panel` keeps working, because it is where every existing bookmark
        // and every link inside the app points.
        let request = Request::builder()
            .uri("/panel")
            .header(header::HOST, "panel.shirleyshow.site")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::OK,
            "`/panel` must keep answering: nothing else changed for it"
        );
    }

    #[tokio::test]
    async fn test_the_door_host_serves_the_panel_bundle_at_its_root() {
        // The door is a mode of the panel application, not a page of the landing.
        // Serving the landing at `puerta.shirleyshow.site/` looks harmless — it
        // answers 200 — but the landing never loads the panel bundle, so the doorman
        // never reaches the door mode. A 200 that makes the feature unreachable is
        // worse than a 404: it looks like it worked.
        let (app, _tmp) = app();

        let request = Request::builder()
            .uri("/")
            .header(header::HOST, "puerta.shirleyshow.site")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);

        assert!(
            html.contains("/assets/js/main.js"),
            "the door host must serve the panel document at `/`: the doorman needs the \
             bundle to enter door mode, and the landing never loads it"
        );
        assert!(
            !html.contains("pres__hero"),
            "the door host must not serve the public landing at `/`"
        );
    }

    #[tokio::test]
    async fn test_the_landing_host_is_unaffected_by_the_door_root() {
        // Whatever the door serves at `/`, the public site keeps serving the landing
        // there. Pinning both sides stops a fix for one from silently breaking the
        // other: they share the same route.
        let (app, _tmp) = app();

        let request = Request::builder()
            .uri("/")
            .header(header::HOST, "shirleyshow.site")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);

        assert!(html.contains("pres__hero"), "the public site keeps the landing at /");
        assert!(
            !html.contains("/assets/js/main.js"),
            "the public landing must stay independent of the panel bundle"
        );
    }
}

#[test]
fn test_the_door_does_not_close_the_event() {
    // `finalizeEvent` lives in `controllers.js`, which the door does not
    // import. The doc-comment used to claim the door closed the event; pinning
    // the refusal here stops anyone from widening the allowlist to match it.
    let cfg = three_hosts();
    for path in [
        "/api/eventos/abc/finalizar",
        "/api/eventos/abc/reabrir",
    ] {
        assert!(
            !cfg.door_allows("POST", path),
            "the door must not reach POST {path}: only the panel closes an event"
        );
    }
}

