//! Reproduces one transport round trip from the local side only.
//!
//! Takes the URL the runtime reported and does exactly what the separator does
//! with it: resolve the direct link, download, decrypt. Running this against a
//! known-good URL separates "the runtime cannot upload" from "we cannot
//! download", which the product error cannot distinguish.

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
=== extract_direct_link ==="
    );
    let direct = shirley_show::infra::vocal::colab_transfer::extract_direct_link(&body);
    println!("{direct:?}");

    let Some(direct) = direct else {
        eprintln!(
            "
FALLO: no se pudo extraer enlace directo"
        );
        std::process::exit(1);
    };

    println!(
        "
=== descarga del enlace directo ==="
    );
    let enc: PathBuf = std::env::temp_dir().join("reverse_probe.enc");
    let resp = client.get(&direct).send().await?;
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
    println!(
        "looks_like_html: {}",
        shirley_show::infra::vocal::colab_transfer::looks_like_html(&bytes)
    );
    std::fs::write(&enc, &bytes)?;

    println!(
        "
=== sha256 del archivo bajado ==="
    );
    let out = Command::new("sha256sum").arg(&enc).output()?;
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
            &direct,
        ])
        .output()?;
    println!("{}", String::from_utf8_lossy(&out.stdout).trim());

    Ok(())
}
