//! Generic HTTP probe for a URL the Colab runtime reports.
//!
//! Diagnostics only. It says nothing about how a separation result travels: the
//! official `colab download -s NAME REMOTE LOCAL` fetches the plain MP3 from the
//! runtime, and the old catbox/litterbox + AES arc is gone. What is left here is
//! the part that is still useful when a transfer misbehaves — fetch a URL and
//! report exactly what came back (status, content type, size, body preview,
//! sha256) — so a failed separation can be attributed to the upload, to the
//! transfer, or to the audio itself instead of being one opaque product error.

use std::path::PathBuf;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("uso: cargo run --example colab_reverse_probe -- <url>");
        std::process::exit(2);
    }
    let url = &args[1];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    println!("=== url reportada por el runtime ===");
    println!("{url}");

    let resp = client.get(url).send().await?;
    let status = resp.status();
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    println!(
        "
=== respuesta ==="
    );
    println!("status: {status}");
    println!("content-type: {ctype}");

    let body = resp.text().await?;
    println!("cuerpo ({} bytes):", body.len());
    println!("{}", body.chars().take(600).collect::<String>());

    println!(
        "
=== descarga completa ==="
    );
    let fetched: PathBuf = std::env::temp_dir().join("colab_reverse_probe.bin");
    let resp = client.get(url).send().await?;
    let st = resp.status();
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp.bytes().await?;
    println!("status: {st}");
    println!("content-type: {ct}");
    println!("bytes: {}", bytes.len());
    std::fs::write(&fetched, &bytes)?;

    println!(
        "
=== sha256 del archivo bajado ==="
    );
    let out = Command::new("sha256sum").arg(&fetched).output()?;
    println!("{}", String::from_utf8_lossy(&out.stdout).trim());

    println!(
        "
=== desde la shell (control independiente) ==="
    );
    let out = Command::new("curl")
        .args([
            "-sSL",
            "-o",
            "/dev/null",
            "-w",
            "http=%{http_code} size=%{size_download} type=%{content_type}\n",
            url,
        ])
        .output()?;
    println!("{}", String::from_utf8_lossy(&out.stdout).trim());

    Ok(())
}
