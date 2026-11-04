use shirley_show::infra::vocal::colab_transfer::{
    TRANSFER_UPLOADERS, TransferUploader, build_uploader_block, extract_direct_link,
};

// T1. The uploader chain must lead with a host that actually works from a
// Google Cloud datacenter IP. Litterbox answers 412 "Invalid uploader" there,
// which is the defect this chain exists to survive, so it can never be first.
#[test]
fn test_uploader_chain_starts_with_google_cloud_safe_host() {
    let first = TRANSFER_UPLOADERS
        .first()
        .expect("the chain must not be empty");
    assert_eq!(
        first.name, "tmpfiles",
        "the first uploader must be the one verified from a Colab node"
    );
    assert!(
        first.body.contains("tmpfiles.org"),
        "the first uploader must target tmpfiles.org"
    );
}

// T2. Every uploader in the chain carries a complete, runnable shell snippet
// with the three contract placeholders substituted by the generator. A chain
// entry that cannot compile into the remote script is worse than no entry.
#[test]
fn test_every_uploader_block_is_complete() {
    let block = build_uploader_block("/tmp/in.enc", "/tmp/in.enc");
    for uploader in TRANSFER_UPLOADERS {
        assert!(
            block.contains(uploader.name),
            "the generated block must label the {} attempt",
            uploader.name
        );
        assert!(
            !uploader.body.contains("{url}")
                && !uploader.body.contains("{status}")
                && !uploader.body.contains("{file}"),
            "uploader {} still has unsubstituted placeholders",
            uploader.name
        );
    }
}

// T3. The generated block must try every uploader, keep the first success, and
// only fail when the whole chain failed. Order matters: it must not stop at the
// first uploader that merely answers.
#[test]
fn test_uploader_block_tries_chain_in_order_and_fails_last() {
    let block = build_uploader_block("/tmp/a.enc", "/tmp/a.enc");
    let mut last = 0usize;
    for uploader in TRANSFER_UPLOADERS {
        let pos = block
            .find(uploader.name)
            .unwrap_or_else(|| panic!("missing {} in block", uploader.name));
        assert!(
            pos >= last,
            "uploader {} is out of order in the generated block",
            uploader.name
        );
        last = pos;
    }
    assert!(
        block.contains("TRANSFER_OK"),
        "the block must announce success explicitly"
    );
    assert!(
        block.contains("ALL_UPLOADERS_FAILED"),
        "the block must fail loudly when every uploader failed"
    );
}

// T4. tmpfiles answers with a landing page, not the file. The direct link lives
// inside that HTML, so the extractor has to pull it out; a naive replace of
// "tmpfiles.org/" with "tmpfiles.org/dl/" returns a 302 back to the page.
#[test]
fn test_extract_direct_link_from_tmpfiles_page() {
    let html = r#"<p><a class="download"
        href="https://tmpfiles.org/dl/1793811490.b0d46bbbc4cf1ae6/wVA2o0NX7Vvy/payload.bin">
        Download (2.00 MB)</a></p>"#;
    let link = extract_direct_link(html).expect("the direct link must be found");
    assert_eq!(
        link,
        "https://tmpfiles.org/dl/1793811490.b0d46bbbc4cf1ae6/wVA2o0NX7Vvy/payload.bin"
    );
}

// T5. A response that is already a direct file URL is returned unchanged, so
// uploaders that answer with the file itself need no special case.
#[test]
fn test_extract_direct_link_accepts_plain_url() {
    let body = "https://litter.catbox.moe/abc123.enc";
    let link = extract_direct_link(body).expect("a plain URL must be accepted");
    assert_eq!(link, "https://litter.catbox.moe/abc123.enc");
}

// T6. A host that answers with an error must not be mistaken for a success.
// "Invalid uploader" has no URL in it, so extraction fails and the chain moves
// on instead of downloading 16 bytes of error text as if it were audio.
#[test]
fn test_extract_direct_link_rejects_error_bodies() {
    assert!(extract_direct_link("Invalid uploader").is_none());
    assert!(extract_direct_link("No expire time specified.").is_none());
    assert!(extract_direct_link("").is_none());
    assert!(
        extract_direct_link("Uploads paused until I can resolve storage issues. Sorry!").is_none()
    );
}

// T7. The downloader must never accept an HTML body as the payload. That is the
// exact failure mode that produced a "successful" 2797-byte transfer when the
// real file was 2 MiB.
#[test]
fn test_downloader_rejects_html_bodies() {
    let html = "<!DOCTYPE html><html><head><title>/tmp/files</title></head></html>";
    assert!(
        shirley_show::infra::vocal::colab_transfer::looks_like_html(html.as_bytes()),
        "an HTML landing page must be rejected before decryption"
    );

    let mp3_prefix = b"\xff\xfb\x90\x64";
    assert!(
        !shirley_show::infra::vocal::colab_transfer::looks_like_html(mp3_prefix),
        "an MP3 frame header is not HTML"
    );
}

// T8. The retention window is a named constant, not a literal scattered in the
// code, because it is a product decision the user stated explicitly: three
// minutes after the last request.
#[test]
fn test_retention_window_is_three_minutes() {
    assert_eq!(
        shirley_show::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION.as_secs(),
        180
    );
    let _ = TransferUploader {
        name: "x",
        body: "y",
    };
}

// T9. `colab exec --json` does not print the script's stdout: it wraps it in
// its own envelope, `{"duration":..,"gpu":..,"output":"<script stdout>","status":"success"}`.
// Reading only the innermost line means a perfectly good run is reported as
// "invalid output", which is indistinguishable from a real failure.
#[test]
fn test_parse_recovers_payload_from_colab_exec_envelope() {
    let inner = r#"{"url": "https://tmpfiles.org/dl/1.a/b/output.mp3.enc", "sha256": "e85cba", "duration": 60.03, "size": 2402264}"#;
    let envelope = serde_json::json!({
        "duration": 133.8,
        "gpu": "G4",
        "output": format!("{inner}\n"),
        "status": "success"
    })
    .to_string();

    let parsed =
        shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(&envelope)
            .expect("the envelope must be unwrapped");
    assert_eq!(parsed.url, "https://tmpfiles.org/dl/1.a/b/output.mp3.enc");
    assert_eq!(parsed.sha256, "e85cba");
    assert_eq!(parsed.size, 2402264);
}

// T10. A bare JSON line (no envelope) must still parse, because
// `colab exec` without --json and the probes both produce that shape.
#[test]
fn test_parse_accepts_bare_payload_line() {
    let bare = r#"{"url": "https://x/y.enc", "sha256": "abc", "duration": 1.0, "size": 10}"#;
    let parsed = shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(bare)
        .expect("a bare line must parse");
    assert_eq!(parsed.url, "https://x/y.enc");
}

// T11. A runtime that failed must NOT be parsed as success. The envelope says
// "success" even when the script raised, because the kernel ran fine; the real
// signal is the absence of the payload inside "output".
#[test]
fn test_parse_rejects_envelope_without_payload() {
    let envelope = serde_json::json!({
        "duration": 12.0,
        "gpu": "G4",
        "output": "RuntimeError: Litterbox upload failed: Invalid uploader\n",
        "status": "success"
    })
    .to_string();
    assert!(
        shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(&envelope)
            .is_none(),
        "an envelope whose output has no payload must not be accepted"
    );
}

// T12. The CLI pretty-prints its envelope across several lines, so a check for
// "a line that starts with { and ends with }" never matches it. This exact
// multi-line shape is what `colab exec --json` produces, and failing it made
// every real separation report "invalid output".
#[test]
fn test_parse_handles_pretty_printed_multiline_envelope() {
    let raw = r#"{
  "duration": 128.975650017,
  "gpu": "G4",
  "output": "{\"url\": \"https://tmpfiles.org/dl/1.a/b/output.mp3.enc\", \"sha256\": \"e85cba\", \"duration\": 60.029388, \"size\": 2402264}\n",
  "status": "success"
}"#;
    let parsed = shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result(raw)
        .expect("a multi-line envelope must be unwrapped");
    assert_eq!(parsed.url, "https://tmpfiles.org/dl/1.a/b/output.mp3.enc");
    assert_eq!(parsed.size, 2402264);
}

// T13. tmpfiles (behind Cloudflare) publishes AAAA records that are not routable
// from here: the connect to [::]:443 is refused while the same host over IPv4
// answers 200. reqwest offered no way to prefer IPv4, so the download runs
// through curl with -4. The flag must be there, and --fail must be there so an
// HTTP error is never written to disk as if it were the payload.
#[test]
fn test_transfer_curl_args_pin_ipv4_and_fail_on_http_errors() {
    let args = shirley_show::infra::vocal::colab_transfer::transfer_curl_args(
        "https://example.test/x.enc",
        "/tmp/x.enc",
        300,
    );
    assert!(args.contains(&"-4".to_string()), "IPv4 must be pinned");
    assert!(
        args.contains(&"--fail".to_string()),
        "an HTTP error must not be saved as a file"
    );
    assert!(args.contains(&"https://example.test/x.enc".to_string()));
    assert!(args.contains(&"/tmp/x.enc".to_string()));
}

// T14. The user's requirement, stated directly: after a request finishes, the
// machine stays up for three minutes, and a second request inside that window
// reuses it instead of paying another cold start. The cache is the piece that
// makes "reuse" possible; without it every request calls `colab start`.
#[test]
fn test_session_cache_reuses_within_retention_and_expires_after() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;

    let mut cache = ColabSessionCache::new();
    assert!(cache.current().is_none(), "nothing cached at boot");

    cache.store("sess-1".to_string());
    assert_eq!(cache.current().as_deref(), Some("sess-1"));

    // Still inside the window: the same session must come back.
    assert_eq!(
        cache
            .reusable(std::time::Duration::from_secs(150))
            .as_deref(),
        Some("sess-1"),
        "a session kept for 3 minutes must be reusable after 150s"
    );

    // Past the window is covered by test_session_cache_expires_against_real_age,
    // which can move the clock; here only the inside-window case is asserted.
}

// T15. A session that Colab dropped on its own (idle timeout, preemption) must
// not be handed back as if it were alive. Marking it gone is what prevents a
// second request from failing against a dead runtime.
#[test]
fn test_session_cache_drops_invalidated_session() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;

    let mut cache = ColabSessionCache::new();
    cache.store("sess-1".to_string());
    cache.invalidate();
    assert!(cache.current().is_none());
    assert_eq!(cache.reusable(std::time::Duration::from_secs(1)), None);
}

// T16. The retention window is a product decision, not a tuning knob that can
// silently become 10 seconds or an hour.
#[test]
fn test_retention_is_three_minutes() {
    assert_eq!(
        shirley_show::infra::vocal::colab_transfer::SESSION_IDLE_RETENTION.as_secs(),
        180
    );
}

// T14b. The retention boundary must be checked against the age the session
// actually has, not against the moment the test happens to call in. Aging is
// the whole behaviour, so the clock has to be movable.
#[test]
fn test_session_cache_expires_against_real_age() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::{Duration, Instant};

    let mut cache = ColabSessionCache::new();
    // Stored four minutes ago: past the three-minute window.
    cache.store_at(
        "sess-old".to_string(),
        Instant::now() - Duration::from_secs(240),
    );
    assert_eq!(
        cache.reusable(Duration::from_secs(180)),
        None,
        "a session stored 240s ago is past the 180s window"
    );
    // The id is still known even when it is too old to reuse; knowing it is what
    // lets the caller stop it.
    assert_eq!(cache.current().as_deref(), Some("sess-old"));

    // Stored two minutes ago: still inside the window.
    cache.store_at(
        "sess-new".to_string(),
        Instant::now() - Duration::from_secs(120),
    );
    assert_eq!(
        cache.reusable(Duration::from_secs(180)).as_deref(),
        Some("sess-new")
    );
}

// T17. A transient connection failure must be retried, because the same URL
// answered 200 on four manual checks and failed once inside a run. One refusal
// cannot be treated as "no uploader accepted the file": that discards minutes of
// GPU work.
//
// An HTTP error must NOT be retried: a 404 or a 412 answers the same forever,
// and retrying only adds latency to a decided failure. curl reports that case as
// exit 22 via --fail, so the two are distinguishable.
#[test]
fn test_fetch_with_retry_succeeds_after_transient_failures() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // A server that drops the first two connections and then serves the payload.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let payload = b"PAYLOAD-BYTES";
        let mut served = 0;
        for stream in listener.incoming() {
            let mut s = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            if served < 2 {
                served += 1;
                // Close without answering: a connection-level failure.
                drop(s);
                continue;
            }
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            let _ = s.write_all(header.as_bytes());
            let _ = s.write_all(payload);
            let _ = s.flush();
            served += 1;
            if served >= 3 {
                break;
            }
        }
    });

    let dest = std::env::temp_dir().join(format!("retry_{}.bin", std::process::id()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = format!("http://127.0.0.1:{port}/x");
    let result = rt.block_on(async {
        shirley_show::infra::vocal::colab_transfer::fetch_with_retry(
            &url,
            &dest,
            std::time::Duration::from_secs(10),
            4,
        )
        .await
    });
    let _ = std::fs::remove_file(&dest);

    match result {
        Ok(bytes) => assert_eq!(bytes, b"PAYLOAD-BYTES"),
        Err(e) => panic!("a transient failure must be retried, got: {e}"),
    }
}

// T18. An HTTP error is not transient. Retrying it wastes the caller's budget
// on a decision that is already made.
#[test]
fn test_fetch_with_retry_does_not_retry_http_errors() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = std::sync::Arc::new(AtomicUsize::new(0));
    let hits_srv = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut s = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            hits_srv.fetch_add(1, Ordering::SeqCst);
            let _ = s.write_all(b"HTTP/1.1 412 Precondition Failed\r\nContent-Length: 3\r\nConnection: close\r\n\r\nno!");
            let _ = s.flush();
        }
    });

    let dest = std::env::temp_dir().join(format!("noretry_{}.bin", std::process::id()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = format!("http://127.0.0.1:{port}/x");
    let result = rt.block_on(async {
        shirley_show::infra::vocal::colab_transfer::fetch_with_retry(
            &url,
            &dest,
            std::time::Duration::from_secs(10),
            4,
        )
        .await
    });
    let _ = std::fs::remove_file(&dest);

    assert!(result.is_err(), "an HTTP error must fail");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "an HTTP error must not be retried"
    );
}
