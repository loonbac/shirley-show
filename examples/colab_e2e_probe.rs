//! Real end-to-end probe of the Colab vocal separation pipeline.
//!
//! Complements the unit tests, which drive the separator against a fake
//! `colab` binary. Those prove the control flow; this proves the thing works
//! against the real Colab GPU, the real litterbox transfer, real openssl and
//! real ffprobe. Both matter, and they cover different failures.
//!
//! Run it explicitly, because it spends real GPU quota:
//!
//! ```sh
//! cargo run --example colab_e2e_probe -- <audio-file> [instrumental|chorus]
//! ```
//!
//! The cleanup is not left to the caller. Whatever happens — success, error,
//! panic or Ctrl-C — the Colab runtime is stopped and the source file is
//! removed, so a failed probe cannot leave a GPU billing in the background.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::colab_vocal_separator::ColabVocalSeparator;
use shirley_show::infra::vocal::colab_vocal_separator_config::ColabVocalSeparatorConfig;
use shirley_show::ports::vocal_separator::{VocalSeparationRequest, VocalSeparator};

/// Stops any Colab runtime, used both by the normal exit path and by the
/// panic hook. `colab stop` is the only thing that releases the GPU.
fn stop_everything() {
    let out = Command::new("colab").args(["stop"]).output();
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                println!("[cleanup] colab stop: sin salida");
            } else {
                println!("[cleanup] colab stop: {trimmed}");
            }
        }
        Err(e) => println!("[cleanup] colab stop falló: {e}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("uso: cargo run --example colab_e2e_probe -- <audio> [instrumental|chorus]");
        std::process::exit(2);
    }

    let source = PathBuf::from(&args[1]);
    if !source.is_file() {
        eprintln!("no existe el archivo: {}", source.display());
        std::process::exit(2);
    }

    let mode = match args.get(2).map(String::as_str) {
        Some("chorus") | Some("coros") => VocalSeparationMode::Coros,
        _ => VocalSeparationMode::Instrumental,
    };

    // A panic must not leak a GPU. The separator already has its Drop guard,
    // but a panic can escape before that guard exists, so this is the outer net.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("[panic] apagando GPU antes de abortar");
        stop_everything();
        default_hook(info);
    }));

    let work_dir = std::env::temp_dir().join(format!("colab_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&work_dir)?;

    let mut config = ColabVocalSeparatorConfig::from_env(
        work_dir.clone(),
        Arc::new(tokio::sync::Semaphore::new(1)),
    );
    // The probe measures the real cost, so it uses the documented defaults
    // rather than whatever a local shell happens to have exported.
    config.gpu_type = std::env::var("SHIRLEY_COLAB_GPU").unwrap_or_else(|_| "g4".to_string());
    let timeout_secs: u64 = std::env::var("SHIRLEY_COLAB_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);

    println!("=== probe de separación Colab ===");
    println!("fuente:   {}", source.display());
    println!("modo:     {:?}", mode);
    println!("gpu:      {}", config.gpu_type);
    println!("overlap:  {}", config.overlap);
    println!("timeout:  {}s", timeout_secs);
    println!("token:    {}", config.token_path.display());
    println!();

    config.validate_token()?;
    println!("[1/4] token válido");

    let separator = ColabVocalSeparator::new(config);
    let output = work_dir.join("output.mp3");

    let request = VocalSeparationRequest {
        source_path: source.clone(),
        output_path: output.clone(),
        mode,
        timeout: Duration::from_secs(timeout_secs),
    };

    println!("[2/4] arrancando GPU y ejecutando el ensemble...");
    let started = Instant::now();
    let result = separator.separate(request).await;
    let elapsed = started.elapsed();

    // Stop before reporting, so a slow report never leaves the GPU running.
    println!();
    stop_everything();

    match result {
        Ok(audio) => {
            println!("[3/4] separación OK en {:.1}s", elapsed.as_secs_f64());
            println!("       duración: {}s", audio.duration_secs);
            println!("       tamaño:   {} bytes", audio.size_bytes);

            let meta = std::fs::metadata(&output)?;
            println!("       en disco: {} bytes", meta.len());
            if meta.len() == 0 {
                eprintln!("FALLO: el archivo de salida está vacío");
                std::process::exit(1);
            }
            if meta.len() != audio.size_bytes {
                eprintln!(
                    "AVISO: lo reportado ({} B) no coincide con el disco ({} B)",
                    audio.size_bytes,
                    meta.len()
                );
            }
            println!("       salida:   {}", output.display());
            println!();
            println!(
                "[4/4] el archivo queda en {}. Escúchalo y luego bórralo.",
                output.display()
            );
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!(
                "[3/4] FALLÓ tras {:.1}s [{}]: {e}",
                elapsed.as_secs_f64(),
                e.code()
            );
            eprintln!(
                "       deja el archivo temporal para inspección: {}",
                work_dir.display()
            );
            eprintln!("[4/4] revisa arriba qué paso falló; la GPU ya está apagada");
            std::process::exit(1);
        }
    }
}
