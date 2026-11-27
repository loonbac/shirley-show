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

    // Barrido de runtimes huérfanos al arrancar.
    //
    // Un SIGKILL, un OOM o un reinicio duro no ejecutan ningún destructor, así
    // que la tarea de release que vive dentro del proceso nunca corre y la GPU
    // sigue facturando hasta que Colab expira la sesión. El proceso nuevo es el
    // primero que puede limpiarlo, y aquí no hay ningún turno en curso, así que
    // cualquier runtime vivo lo dejó un proceso anterior.
    {
        let colab_bin = std::env::var("SHIRLEY_COLAB_BIN")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("colab"));
        let token_path =
            shirley_show::infra::vocal::colab_vocal_separator_config::token_path_from_env();
        // The lease tells us whether a runtime belongs to a *living* sibling.
        // Sweeping while another instance is alive would destroy its work: a
        // blind sweep killed a separation mid-flight and the user saw
        // `separation_failed` with no explanation. When it is safe, the sweep
        // enumerates the official CLI's sessions and stops each by name.
        let foreign_owner =
            shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&config.data_dir);
        let decision =
            shirley_show::infra::vocal::colab_orphan_reaper::boot_reap_decision(foreign_owner);
        if decision == shirley_show::infra::vocal::colab_orphan_reaper::ReapDecision::Stop {
            let stopped = shirley_show::infra::vocal::colab_orphan_reaper::stop_all_runtimes(
                &colab_bin,
                &token_path,
                std::time::Duration::from_secs(60),
            )
            .await;
            println!("Barrido de GPU al arrancar: runtimes huérfanos liberados={stopped}");
        } else {
            println!("Barrido de GPU al arrancar: omitido, otra instancia sigue viva");
        }
    }
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

    let mut state = AppState::new_with_all_components_and_config(
        pool,
        storage,
        staged_audio_registry,
        youtube_downloader,
        &config,
    );
    state.frontend_dir = config.frontend_dir;

    let separation_timeout = state.separate_vocals.timeout();

    let agent_limits = AgentLimits {
        max_iterations: 5,
        request_timeout: std::time::Duration::from_secs(config.agent_timeout_secs),
        separation_timeout,
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
                eprintln!("Aviso: No se pudo inicializar el cliente de IA: {e}");
            }
        }
    } else {
        state = state.with_agent_provider(None, agent_limits, config.groq.model.clone());
        println!(
            "Asistente Ressy en modo no configurado (OPENCODE_API_KEY ni GROQ_API_KEY detectadas)."
        );
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
