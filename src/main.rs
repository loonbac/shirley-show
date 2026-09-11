use std::fs;
use std::sync::Arc;

use shirley_show::config::Config;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::filesystem::reconcile_audio_files::reconcile_audio_files;
use shirley_show::infra::groq::client::GroqClient;
use shirley_show::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::infra::youtube::{ProcessYoutubeDownloader, ProcessYoutubeDownloaderConfig};
use shirley_show::ports::youtube_downloader::YoutubeDownloader;
use shirley_show::router::create_router;
use shirley_show::state::{AppState, StagedAudioRegistry};
use shirley_show::use_cases::agent_chat::AgentLimits;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = Config::from_env().map_err(|e| format!("Config error: {e}"))?;

    eprintln!("====================================================================");
    eprintln!("ADVERTENCIA: Shirley Show no incluye autenticación ni control de acceso.");
    eprintln!("No exponer públicamente sin una capa de seguridad o proxy inverso.");
    eprintln!("====================================================================");

    let audio_dir = config.data_dir.join("audio");
    fs::create_dir_all(audio_dir.join("files"))?;
    fs::create_dir_all(audio_dir.join("staging"))?;
    fs::create_dir_all(&config.frontend_dir)?;

    let db_path = config.data_dir.join("shirley-show.sqlite3");
    let pool = init_pool(db_path.to_str().expect("valid path"))?;
    migrate_pool(&pool)?;

    let storage = Arc::new(LocalAudioStorage::new(audio_dir));
    let cleanup_repo = SqliteCleanupRepository::new(pool.clone());

    match reconcile_audio_files(&storage, &cleanup_repo) {
        Ok(report) => {
            println!(
                "Reconciliación de audio: {} limpiezas completadas, {} fallos, {} temporales purgados",
                report.processed_deletions, report.failed_deletions, report.purged_staging
            );
        }
        Err(e) => {
            eprintln!("Aviso: Falló la reconciliación inicial de audio: {e}");
        }
    }

    let staged_audio_registry = Arc::new(StagedAudioRegistry::new());
    let youtube_downloader: Arc<dyn YoutubeDownloader> = Arc::new(ProcessYoutubeDownloader::new(
        ProcessYoutubeDownloaderConfig {
            yt_dlp_bin: config.yt_dlp_bin.clone(),
            timeout: std::time::Duration::from_secs(90),
        },
    ));

    let mut state =
        AppState::new_with_all_components(pool, storage, staged_audio_registry, youtube_downloader);
    state.frontend_dir = config.frontend_dir;

    let agent_limits = AgentLimits {
        max_iterations: 5,
        request_timeout: std::time::Duration::from_secs(config.agent_timeout_secs),
        separation_timeout: std::time::Duration::from_secs(config.separation_timeout_secs),
    };

    if config.groq.api_key.is_some() {
        match GroqClient::new(config.groq.clone()) {
            Ok(client) => {
                state = state.with_agent_provider(
                    Some(Arc::new(client)),
                    agent_limits,
                    config.groq.model.clone(),
                );
                println!(
                    "Asistente Ressy configurada con modelo {}",
                    config.groq.model
                );
            }
            Err(e) => {
                eprintln!("Aviso: No se pudo inicializar el cliente de Groq: {e}");
            }
        }
    } else {
        state = state.with_agent_provider(None, agent_limits, config.groq.model.clone());
        println!("Asistente Ressy en modo no configurado (GROQ_API_KEY no detectada).");
    }

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    println!("Shirley Show escuchando en http://{}", config.bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Servidor detenido limpiamente.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
