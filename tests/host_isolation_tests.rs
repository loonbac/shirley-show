//! The landing host must not expose the application.
//!
//! The public landing is served from the same container as the panel and the
//! API. `/api/agent/chat` has no authentication layer, so a border rule alone
//! must never be the thing that keeps it closed: if Cloudflare Access were
//! misconfigured, or the container were reached directly, the landing host
//! would publish an unauthenticated API. The cut therefore lives in the
//! application, keyed on the `Host` header, and these tests pin it.

use shirley_show::config::{PublicSurface, SurfaceConfig};

#[test]
fn test_landing_host_serves_only_the_landing() {
    let cfg = SurfaceConfig::new(
        Some("https://shirleyshow.site"),
        Some("https://panel.shirleyshow.site"),
    );
    let landing = cfg.classify(Some("shirleyshow.site"));

    assert!(landing.serves_landing(), "the root host serves the landing");
    assert!(
        !landing.serves_panel(),
        "the public host must not serve the panel"
    );
    assert!(
        !landing.serves_api(),
        "/api/agent/chat has no auth layer: the public host must not expose it"
    );
}

#[test]
fn test_panel_host_serves_the_application() {
    let cfg = SurfaceConfig::new(
        Some("https://shirleyshow.site"),
        Some("https://panel.shirleyshow.site"),
    );
    let panel = cfg.classify(Some("panel.shirleyshow.site"));

    assert!(panel.serves_panel(), "the panel host serves the panel");
    assert!(panel.serves_api(), "the panel host serves the API");
    // The panel host keeps serving the landing document too: existing
    // single-host deployments rely on `/` being the landing, and dropping it
    // there would be a regression nobody asked for.
    assert!(
        panel.serves_landing(),
        "the application host keeps the landing available"
    );
}

#[test]
fn test_unknown_and_absent_hosts_stay_closed() {
    // No Host header (or an unexpected one) must never be treated as public:
    // a request that reaches the container directly, or through an IP, is not
    // a visitor of the landing site.
    let cfg = SurfaceConfig::new(
        Some("https://shirleyshow.site"),
        Some("https://panel.shirleyshow.site"),
    );
    for host in [None, Some("100.82.97.118"), Some("evil.example.com")] {
        let surface = cfg.classify(host);
        assert!(
            !surface.serves_api(),
            "host {host:?} must not reach the API"
        );
        assert!(
            !surface.serves_panel(),
            "host {host:?} must not reach the panel"
        );
    }
}

// The tests above prove the rule. This one proves the wiring, which is where a
// real bug lived: the rule was correct but the guard only closed the landing
// surface, so an unknown host still reached the API. A unit test of the rule
// alone could not have caught it.

mod wiring {
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use shirley_show::config::{Config, SurfaceConfig};
    use shirley_show::router::create_router;
    use shirley_show::state::AppState;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn app_with_split() -> (axum::Router, TempDir) {
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
        std::fs::write(
            frontend.join("landing/index.html"),
            "<html>landing</html>",
        )
        .unwrap();

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
            surface: SurfaceConfig::new(
                Some("https://shirleyshow.site"),
                Some("https://panel.shirleyshow.site"),
            ),
        };
        std::fs::create_dir_all(&config.data_dir).unwrap();

        let storage = std::sync::Arc::new(
            shirley_show::infra::filesystem::LocalAudioStorage::new(audio_dir),
        );
        let state = AppState::new_with_all_components_and_config(
            pool,
            storage,
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

    async fn status_for(app: axum::Router, host: &str, path: &str) -> StatusCode {
        let request = Request::builder()
            .uri(path)
            .header(header::HOST, host)
            .body(Body::empty())
            .unwrap();
        app.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn test_public_host_cannot_reach_the_api() {
        let (app, _tmp) = app_with_split();
        assert_eq!(
            status_for(app.clone(), "shirleyshow.site", "/api/presentations").await,
            StatusCode::NOT_FOUND,
            "the landing host must not expose /api: /api/agent/chat has no auth layer"
        );
        assert_eq!(
            status_for(app, "shirleyshow.site", "/panel").await,
            StatusCode::NOT_FOUND,
            "the landing host must not serve the panel"
        );
    }

    #[tokio::test]
    async fn test_public_host_still_serves_the_landing() {
        let (app, _tmp) = app_with_split();
        assert_eq!(
            status_for(app, "shirleyshow.site", "/").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn test_unknown_host_cannot_reach_the_api() {
        // This is the case the rule alone got right and the wiring got wrong:
        // a request with an IP or a foreign Host still reached the API.
        let (app, _tmp) = app_with_split();
        for host in ["100.82.97.118", "evil.example.com"] {
            assert_eq!(
                status_for(app.clone(), host, "/api/presentations").await,
                StatusCode::NOT_FOUND,
                "host {host} must not reach the API"
            );
        }
    }

    #[tokio::test]
    async fn test_panel_host_reaches_the_api() {
        let (app, _tmp) = app_with_split();
        assert_eq!(
            status_for(app, "panel.shirleyshow.site", "/api/presentations").await,
            StatusCode::OK,
            "the panel host must keep working: production depends on it"
        );
    }
}

#[test]
fn test_single_host_deployment_is_untouched() {
    // A deployment that never opted into the split serves everything, even
    // without a Host header. Checking the split first would close it: this is
    // the order bug that broke 28 existing API tests.
    let single = SurfaceConfig::new(Some("https://shirleyshow.site"), None);
    assert_eq!(
        single.classify(None),
        PublicSurface::Application,
        "with no panel host configured, a request without Host must still work"
    );
    assert_eq!(
        single.classify(Some("127.0.0.1:3000")),
        PublicSurface::Application
    );
}
