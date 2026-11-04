//! Verifies the retention rule end to end: two separations in a row.
//!
//! The rule is "keep the machine three minutes; a second request reuses it".
//! The numbers that prove it are the wall time of the second run and whether
//! `colab start` was invoked, neither of which the product reports.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::colab_vocal_separator::ColabVocalSeparator;
use shirley_show::infra::vocal::colab_vocal_separator_config::ColabVocalSeparatorConfig;
use shirley_show::ports::vocal_separator::{VocalSeparationRequest, VocalSeparator};

fn stop() {
    let _ = std::process::Command::new("colab").args(["stop"]).output();
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("uso: cargo run --example colab_reuse_probe -- <audio>");
        std::process::exit(2);
    }
    let source = PathBuf::from(&args[1]);
    let work = std::env::temp_dir().join(format!("reuse_{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        stop();
        hook(i);
    }));

    let config =
        ColabVocalSeparatorConfig::from_env(work.clone(), Arc::new(tokio::sync::Semaphore::new(1)));
    let separator = ColabVocalSeparator::new(config);

    for round in 1..=2u32 {
        let output = work.join(format!("round{round}.mp3"));
        let request = VocalSeparationRequest {
            source_path: source.clone(),
            output_path: output.clone(),
            mode: VocalSeparationMode::Instrumental,
            timeout: Duration::from_secs(1800),
        };
        println!("=== ronda {round} ===");
        let t0 = Instant::now();
        match separator.separate(request).await {
            Ok(a) => {
                let secs = t0.elapsed().as_secs_f64();
                let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                println!(
                    "  OK en {secs:.1}s, {}s de audio, {size} bytes",
                    a.duration_secs
                );
                if size == 0 {
                    eprintln!("  FALLO: archivo vacio");
                    stop();
                    std::process::exit(1);
                }
                if round == 1 {
                    println!(
                        "  (no se apaga: se guarda {:?})",
                        shirley_show::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "  FALLO tras {:.1}s [{}]: {e}",
                    t0.elapsed().as_secs_f64(),
                    e.code()
                );
                stop();
                std::process::exit(1);
            }
        }
        println!(
            "  GPU ahora: {}",
            if colab_alive() {
                "encendida"
            } else {
                "apagada"
            }
        );
    }

    println!("\n=== limpieza ===");
    stop();
}

fn colab_alive() -> bool {
    std::process::Command::new("colab")
        .args(["status"])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).contains("No active runtime"))
        .unwrap_or(false)
}
