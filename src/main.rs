use std::fs;
use std::sync::Arc;

use shirley_show::config::Config;
use shirley_show::infra::filesystem::local_audio_storage::LocalAudioStorage;
use shirley_show::infra::filesystem::reconcile_audio_files::reconcile_audio_files;
use shirley_show::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::router::create_router;
use shirley_show::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut state = AppState::new_with_items(pool, storage);
    state.frontend_dir = config.frontend_dir;

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
