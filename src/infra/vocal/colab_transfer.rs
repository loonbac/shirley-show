//! Transport of the separated audio back from the Colab runtime.
//!
//! Colab runs on Google Cloud, and the previous single uploader (litterbox) is
//! blocked there: it answers HTTP 412 with the body `Invalid uploader`. The
//! separation itself succeeds, the GPU time is spent, and then the result is
//! lost because the only way home refuses the request.
//!
//! This module builds the remote-side shell that tries several hosts in order
//! and keeps the first one that yields a real file URL. The same generator is
//! used by the product and by the probes, so what is tested is what runs.

use std::time::Duration;

/// One candidate host for moving the encrypted file out of the runtime.
///
/// `body` is a shell fragment. It must leave the final direct download URL in
/// `$CANDIDATE_URL` and exit 0 on success, or exit non-zero on failure. The
/// generator wires those into the retry chain, so a fragment never has to
/// decide what happens next.
pub struct TransferUploader {
    pub name: &'static str,
    pub body: &'static str,
}

/// How long the runtime is kept alive after a separation finishes.
///
/// A second request inside this window reuses the running machine instead of
/// paying for another cold start. It is a product decision, stated explicitly:
/// three minutes.
pub const SESSION_IDLE_RETENTION: Duration = Duration::from_secs(180);

/// The uploader chain, in the order they are attempted.
///
/// tmpfiles leads because it is the only host verified to work from a Colab
/// node. litterbox and catbox stay as fallbacks: they fail from Google Cloud,
/// but they work from a normal network, so they cost nothing to keep and they
/// cover the case where the runtime is reached from somewhere else.
pub const TRANSFER_UPLOADERS: &[TransferUploader] = &[
    TransferUploader {
        name: "tmpfiles",
        body: r#"CANDIDATE_URL=$(
  curl -sS --max-time 300 -F "file=@__FILE__" https://tmpfiles.org/api/v1/upload \
  | python3 -c 'import sys,json
try:
    print(json.load(sys.stdin)["data"]["url"])
except Exception:
    pass'
)
if [ -n "$CANDIDATE_URL" ]; then
  # tmpfiles returns a landing page. The direct link is inside its HTML and is
  # the only form that downloads the bytes instead of a redirect back to the page.
  PAGE=$(curl -sSL --max-time 120 "$CANDIDATE_URL" || true)
  DIRECT=$(printf '%s' "$PAGE" | python3 -c 'import sys,re
html = sys.stdin.read()
m = re.search(r"""https?://[^"'\'' <>]*?/dl/[^"'\'' <>]+""", html)
print(m.group(0) if m else "")')
  if [ -n "$DIRECT" ]; then CANDIDATE_URL="$DIRECT"; else CANDIDATE_URL=""; fi
fi
[ -n "$CANDIDATE_URL" ]"#,
    },
    TransferUploader {
        name: "litterbox",
        body: r#"CANDIDATE_URL=$(
  curl -sS --max-time 300 -F "reqtype=fileupload" -F "time=1h" \
       -F "fileToUpload=@__FILE__" \
       https://litterbox.catbox.moe/resources/internals/api.php \
  | grep -Eo '^https?://[^[:space:]]+$' || true
)
[ -n "$CANDIDATE_URL" ]"#,
    },
    TransferUploader {
        name: "catbox",
        body: r#"CANDIDATE_URL=$(
  curl -sS --max-time 300 -F "reqtype=fileupload" \
       -F "fileToUpload=@__FILE__" \
       https://catbox.moe/user/api.php \
  | grep -Eo '^https?://[^[:space:]]+$' || true
)
[ -n "$CANDIDATE_URL" ]"#,
    },
];

/// Builds the shell that walks the uploader chain.
///
/// `file` is the remote path of the encrypted payload. The block prints
/// `TRANSFER_OK <url>` on success and `ALL_UPLOADERS_FAILED` when every host
/// failed, so the caller never has to guess which happened.
pub fn build_uploader_block(file: &str, _unused: &str) -> String {
    let mut out = String::new();
    // The chain runs inside a function so the first success can return from it.
    out.push_str("transfer_attempt() {\n");
    out.push_str("TRANSFER_URL=\"\"\n");
    for uploader in TRANSFER_UPLOADERS {
        let body = uploader.body.replace("__FILE__", file);
        out.push_str(&format!("\n# --- uploader: {} ---\n", uploader.name));
        out.push_str("CANDIDATE_URL=\"\"\n");
        out.push_str(&body);
        out.push_str("\n");
        // No break: the attempts live inside a function, and the first success
        // returns from it. An earlier version emitted `break` outside any loop,
        // which bash reports as an error and then *continues*, so the payload was
        // uploaded to every fallback host after already succeeding.
        out.push_str(
            "if [ -n \"$CANDIDATE_URL\" ]; then\n             \tTRANSFER_URL=\"$CANDIDATE_URL\"\n             \treturn 0\n             fi\n",
        );
    }
    out.push_str("return 1\n}\n");
    out.push_str("transfer_attempt\n");
    out.push_str(
        "\nif [ -n \"$TRANSFER_URL\" ]; then\n         \techo \"TRANSFER_OK $TRANSFER_URL\"\n         else\n         \techo \"ALL_UPLOADERS_FAILED\" >&2\n         \texit 1\n         fi\n",
    );
    out
}

/// Extracts the real file URL from an uploader response body.
///
/// Some hosts answer with the URL itself; tmpfiles answers with an HTML landing
/// page whose download anchor holds a tokenised `/dl/` link. Both shapes are
/// accepted, and anything without a URL is rejected so an error body can never
/// be mistaken for a file.
pub fn extract_direct_link(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if trimmed.contains(char::is_whitespace) {
            return None;
        }
        return Some(trimmed.to_string());
    }

    // HTML: prefer the explicit download anchor, then any /dl/ link.
    for marker in ["class=\"download\"", "/dl/"] {
        for segment in trimmed.split("href=\"").skip(1) {
            let Some(end) = segment.find('"') else {
                continue;
            };
            let candidate = &segment[..end];
            if candidate.starts_with("http") && candidate.contains(marker) {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// True when the downloaded bytes are an HTML page rather than the payload.
///
/// This is the check whose absence produced a "successful" 2797-byte transfer
/// for a 2 MiB file: the server answered 200 with a landing page, and the
/// decryption step then failed with a confusing error.
pub fn looks_like_html(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let text = String::from_utf8_lossy(head)
        .trim_start()
        .to_ascii_lowercase();
    text.starts_with("<!doctype html")
        || text.starts_with("<html")
        || text.contains("<head>")
        || text.contains("<body")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_embeds_the_payload_path() {
        let block = build_uploader_block("/content/payload.enc", "/content/payload.enc");
        assert!(block.contains("/content/payload.enc"));
        assert!(!block.contains("__FILE__"));
    }

    #[test]
    fn test_looks_like_html_handles_leading_whitespace() {
        assert!(looks_like_html(b"\n\n  <!DOCTYPE html><html></html>"));
        assert!(!looks_like_html(&[0xff, 0xfb, 0x90, 0x64, 0x00, 0x01]));
    }
}

/// Builds the HTTP client used to move the payload.
///
/// IPv4 is pinned deliberately. Some transfer hosts sit behind Cloudflare and
/// publish AAAA records that are not routable from a container without IPv6
/// egress; `reqwest` takes the first address the resolver returns, so an
/// otherwise valid URL fails with `connection refused` on `[::]:443` while the
/// same host answers 200 over IPv4.
///
/// The policy lives here, in one place, so it cannot drift between the two call
/// sites that need it.
pub fn build_transfer_client(timeout: std::time::Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        // `local_address` alone is not enough: it fixes the source address but
        // leaves name resolution free to pick an AAAA record, which is exactly
        // how the connect ended up going to `[::]:443`. reqwest's bundled
        // resolver does not expose the IP strategy, so this client is only used
        // for small HTML pages; the payload goes through
        // `transfer_curl_args`, which pins IPv4 explicitly.
        .local_address(Some(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)))
        .build()
}

/// Arguments for fetching the payload.
///
/// `curl` is used instead of an HTTP client built in-process for two reasons
/// that both bit this pipeline:
///
/// 1. `-4`. Some hosts publish AAAA records that are unroutable from a
///    container without IPv6 egress, and the connect then fails with
///    `connection refused` on a URL that works fine over IPv4. Neither
///    `reqwest`'s `local_address` nor its bundled resolver expose the IPv4
///    preference, so the client could not be told what to do.
/// 2. `--fail`. A profile built on `rustls` without system roots turned a
///    perfectly good URL into a connect error; `curl` uses the system trust
///    store, which is what an operator expects.
///
/// `--fail` also matters on its own: without it an HTTP 4xx is written to the
/// output file as if it were the payload.
pub fn transfer_curl_args(url: &str, dest: &str, timeout_secs: u64) -> Vec<String> {
    vec![
        "-4".to_string(),
        "--fail".to_string(),
        "-sSL".to_string(),
        "--max-time".to_string(),
        timeout_secs.to_string(),
        "-o".to_string(),
        dest.to_string(),
        url.to_string(),
    ]
}

/// Fetches `url` into `dest` with a bounded retry policy.
///
/// A single attempt is not enough here, and the reason is empirical: the same
/// URL that answers 200 on four consecutive manual checks failed once during a
/// run with `curl: (7) Could not connect`. Treating one transient refusal as a
/// definitive "no uploader accepted the file" throws away two and a half minutes
/// of GPU work for nothing.
///
/// The distinction that matters is between a transport error, which is worth
/// retrying, and an HTTP error, which is not: a 404 or a 412 will answer the
/// same way forever. `--fail` gives curl exit 22 for the second kind, so the
/// retry only fires on connection-level failures.
pub async fn fetch_with_retry(
    url: &str,
    dest: &std::path::Path,
    timeout: Duration,
    attempts: u32,
) -> Result<Vec<u8>, String> {
    use std::process::Stdio;
    use tokio::process::Command;

    // The budget is the caller's, not each attempt's. Giving every attempt the
    // full timeout meant four attempts could run roughly four times past
    // `VocalSeparationRequest.timeout`, so a separation could outlive its own
    // deadline instead of honouring it.
    let deadline = tokio::time::Instant::now() + timeout;
    let mut last_error = String::from("no attempt was made");

    for attempt in 1..=attempts.max(1) {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            last_error = format!("out of time after {} attempt(s)", attempt - 1);
            break;
        }

        let args = transfer_curl_args(
            url,
            dest.to_str().ok_or("destination is not valid UTF-8")?,
            left.as_secs().max(1),
        );
        let mut cmd = Command::new("curl");
        cmd.kill_on_drop(true);
        cmd.args(&args);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let outcome = tokio::time::timeout(left, cmd.output()).await;

        match outcome {
            Ok(Ok(o)) if o.status.success() => {
                let bytes = tokio::fs::read(dest)
                    .await
                    .map_err(|e| format!("read after download: {e}"))?;
                if bytes.is_empty() {
                    last_error = "empty body".to_string();
                } else {
                    return Ok(bytes);
                }
            }
            Ok(Ok(o)) => {
                let code = o.status.code().unwrap_or(-1);
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                // 22 is curl's --fail: the server answered with an HTTP error.
                // Retrying that is pointless and only adds latency to a failure
                // that is already decided.
                if code == 22 {
                    return Err(format!("http error: {}", stderr.trim()));
                }
                last_error = format!("curl exit {code}: {}", stderr.trim());
            }
            Ok(Err(e)) => last_error = format!("exec: {e}"),
            Err(_) => last_error = format!("timeout after {left:?}"),
        }

        if attempt < attempts {
            let pause = Duration::from_secs(2);
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left <= pause {
                break;
            }
            tokio::time::sleep(pause).await;
        }
    }
    Err(last_error)
}
