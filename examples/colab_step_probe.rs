//! Step-by-step diagnostic of the Colab pipeline.
//!
//! `ColabVocalSeparator` collapses five different failures (upload, remote
//! exec, result parsing, download, decryption, sha/duration verification) into
//! the single `InvalidSeparatorOutput` error, which is correct for the product
//! but useless for diagnosis. This probe repeats the same steps while printing
//! everything the separator discards, so the failing stage becomes visible.
//!
//! It is a diagnostic tool, not production code: it duplicates the pipeline on
//! purpose, because the point is to see what the real one hides.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::colab_remote_script::generate_remote_script;
use shirley_show::infra::vocal::colab_vocal_separator_config::{
    COROS_MODELS, ColabVocalSeparatorConfig, INSTRUMENTAL_MODELS,
};

fn run(program: &str, args: &[&str]) -> std::process::Output {
    println!("  $ {program} {}", args.join(" "));
    let out = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("no se pudo ejecutar {program}: {e}"));
    println!("  exit: {:?}", out.status.code());
    let so = String::from_utf8_lossy(&out.stdout);
    let se = String::from_utf8_lossy(&out.stderr);
    if !so.trim().is_empty() {
        println!(
            "  stdout: {}",
            so.trim().chars().take(1500).collect::<String>()
        );
    }
    if !se.trim().is_empty() {
        println!(
            "  stderr: {}",
            se.trim().chars().take(1500).collect::<String>()
        );
    }
    out
}

fn stop() {
    let o = Command::new("colab").args(["stop"]).output();
    if let Ok(o) = o {
        let t = String::from_utf8_lossy(&o.stdout);
        println!("[cleanup] {}", t.trim().replace('\n', " | "));
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("uso: cargo run --example colab_step_probe -- <audio> [instrumental|chorus]");
        std::process::exit(2);
    }
    let source = PathBuf::from(&args[1]);
    let mode = match args.get(2).map(String::as_str) {
        Some("chorus") => VocalSeparationMode::Coros,
        _ => VocalSeparationMode::Instrumental,
    };
    let models = ColabVocalSeparatorConfig::models_for_mode(mode);
    let _ = (INSTRUMENTAL_MODELS, COROS_MODELS);

    let work = std::env::temp_dir().join(format!("step_probe_{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        stop();
        hook(i);
    }));

    println!("=== paso 0: validación ===");
    println!("  fuente: {}", source.display());
    println!("  modo:   {:?}", mode);
    println!("  modelos: {:?}", models);
    if !source.is_file() {
        eprintln!("  no existe");
        std::process::exit(2);
    }

    println!("\n=== paso 1: colab start ===");
    let t0 = Instant::now();
    let start = run("colab", &["start", "--gpu", "g4", "--json"]);
    if !start.status.success() {
        eprintln!("start falló, abortando");
        std::process::exit(1);
    }
    let start_txt = String::from_utf8_lossy(&start.stdout).to_string();
    let session = serde_json::from_str::<serde_json::Value>(&start_txt)
        .ok()
        .and_then(|v| v.get("session").and_then(|s| s.as_str()).map(String::from));
    let Some(session) = session else {
        eprintln!("no pude leer la session del JSON: {start_txt}");
        stop();
        std::process::exit(1);
    };
    println!("  session: {session}  ({:.1}s)", t0.elapsed().as_secs_f64());

    println!("\n=== paso 2: upload ===");
    let t1 = Instant::now();
    let up = run(
        "colab",
        &[
            "upload",
            "--session",
            &session,
            source.to_str().unwrap(),
            "/content/input_audio",
        ],
    );
    println!("  ({:.1}s)", t1.elapsed().as_secs_f64());
    if !up.status.success() {
        eprintln!("upload falló, abortando");
        stop();
        std::process::exit(1);
    }

    println!("\n=== paso 3: script remoto ===");
    let script_path = work.join("remote.py");
    // Same argument order as `colab_vocal_separator.rs`, and the same defaults
    // the config uses (SHIRLEY_COLAB_OVERLAP=12, SHIRLEY_COLAB_BATCH_SIZE=8).
    // This example went stale when `generate_remote_script` gained `batch_size`,
    // and `cargo test` does not compile `examples/` -- only `cargo build`, which
    // the Dockerfile runs in release mode -- so the breakage was invisible to
    // every "suite is green" report.
    let code = generate_remote_script(
        mode,
        models,
        12,
        8,
        "probe_password_123",
        "/content/input_audio",
    );
    std::fs::write(&script_path, &code).unwrap();
    println!(
        "  escrito: {} ({} bytes)",
        script_path.display(),
        code.len()
    );

    println!("\n=== paso 4: colab exec ===");
    let t2 = Instant::now();
    let ex = run(
        "colab",
        &[
            "exec",
            "--session",
            &session,
            "--json",
            script_path.to_str().unwrap(),
        ],
    );
    println!("  ({:.1}s)", t2.elapsed().as_secs_f64());

    let stdout = String::from_utf8_lossy(&ex.stdout).to_string();
    let stderr = String::from_utf8_lossy(&ex.stderr).to_string();
    println!("\n--- stdout completo de exec ({} bytes) ---", stdout.len());
    println!("{}", stdout.chars().take(4000).collect::<String>());
    println!("\n--- stderr completo de exec ({} bytes) ---", stderr.len());
    println!("{}", stderr.chars().take(4000).collect::<String>());

    println!("\n=== paso 5: veredicto del JSON ===");
    match serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        Ok(v) => {
            println!("  JSON parseado: {v}");
            if let Some(url) = v.get("url").and_then(|x| x.as_str()) {
                println!("  url: {url}");
                let t3 = Instant::now();
                let dl = work.join("out.enc");
                let dl_out = Command::new("curl")
                    .args(["-sSL", "-o", dl.to_str().unwrap(), url])
                    .output();
                match dl_out {
                    Ok(o) => {
                        let sz = std::fs::metadata(&dl).map(|m| m.len()).unwrap_or(0);
                        println!(
                            "  descarga: exit={:?} tamaño={} bytes ({:.1}s)",
                            o.status.code(),
                            sz,
                            t3.elapsed().as_secs_f64()
                        );
                        if sz == 0 {
                            println!("  !!! descarga vacía: el bug documentado de litterbox");
                        } else {
                            let dec = work.join("out.mp3");
                            let d = Command::new("openssl")
                                .args([
                                    "enc",
                                    "-d",
                                    "-aes-256-cbc",
                                    "-pbkdf2",
                                    "-salt",
                                    "-pass",
                                    "pass:probe_password_123",
                                    "-in",
                                    dl.to_str().unwrap(),
                                    "-out",
                                    dec.to_str().unwrap(),
                                ])
                                .output();
                            match d {
                                Ok(o) => {
                                    let dsz = std::fs::metadata(&dec).map(|m| m.len()).unwrap_or(0);
                                    println!(
                                        "  descifrado: exit={:?} tamaño={} bytes",
                                        o.status.code(),
                                        dsz
                                    );
                                    if dsz > 0 {
                                        println!("  >>> PIPELINE COMPLETO OK: {}", dec.display());
                                    }
                                }
                                Err(e) => println!("  descifrado no ejecutable: {e}"),
                            }
                        }
                    }
                    Err(e) => println!("  curl falló: {e}"),
                }
            } else {
                println!("  !!! el JSON no trae url");
            }
        }
        Err(e) => println!("  !!! el stdout no es el JSON esperado: {e}"),
    }

    println!("\n=== cleanup ===");
    stop();
}
