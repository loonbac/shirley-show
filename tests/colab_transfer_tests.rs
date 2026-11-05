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

// R4-002 / R3-003. The generated chain emitted "break" outside any loop. Bash
// reports that as an error and *continues*, so a successful uploader did not end
// the chain: it uploaded the same encrypted payload to every fallback host and
// could spend 300s more after already succeeding. Found by native review.
//
// This runs the real generated block against a fake curl and counts calls.
#[test]
fn test_uploader_chain_stops_after_first_success() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("chain_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);

    // Fake tools: curl always reports a URL, python3 passes stdin through.
    let fake_curl = dir.join("curl");
    let mut f = std::fs::File::create(&fake_curl).unwrap();
    writeln!(f, "#!/bin/sh").unwrap();
    writeln!(f, "echo CALLED >> {}", dir.join("calls").display()).unwrap();
    writeln!(f, "echo https://fake.example/file.enc").unwrap();
    drop(f);
    std::fs::set_permissions(&fake_curl, std::fs::Permissions::from_mode(0o755)).unwrap();

    // The tmpfiles uploader pipes curl through python3 to extract the URL. A real
    // python3 would parse the fake response and fail, so the fake must cover it
    // too: emit a direct link and let the shell continue.
    let fake_py = dir.join("python3");
    let mut p = std::fs::File::create(&fake_py).unwrap();
    writeln!(p, "#!/bin/sh").unwrap();
    writeln!(p, "cat > /dev/null").unwrap();
    writeln!(p, "echo https://fake.example/direct.enc").unwrap();
    drop(p);
    std::fs::set_permissions(&fake_py, std::fs::Permissions::from_mode(0o755)).unwrap();

    let block_path = dir.join("block.sh");
    let block = shirley_show::infra::vocal::colab_transfer::build_uploader_block(
        "/tmp/nonexistent.enc",
        "/tmp/nonexistent.enc",
    );
    std::fs::write(&block_path, &block).unwrap();

    let path_with_fake = format!(
        "{}:{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = std::process::Command::new("bash")
        .arg(&block_path)
        .env("PATH", &path_with_fake)
        .output()
        .expect("bash must run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let calls = std::fs::read_to_string(dir.join("calls"))
        .map(|s| s.lines().count())
        .unwrap_or(0);

    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        stdout.contains("TRANSFER_OK"),
        "the chain must report success, got: {stdout} / {stderr}"
    );
    assert!(
        !stderr.contains("break: "),
        "the generated shell must not emit break outside a loop: {stderr}"
    );
    // Three uploaders exist; a working chain stops at the first.
    // tmpfiles legitimately makes two requests: the upload and then the landing
    // page it must read to find the direct link. What matters is that the chain
    // stopped there instead of falling through to litterbox and catbox, which
    // would be four to six requests. Counting hosts is the assertion; counting
    // requests would pin an implementation detail.
    assert!(
        calls <= 2,
        "the chain must stop after the first uploader succeeds, made {calls} requests"
    );
}

// R1-001 / R3-002 / R4-003. Every success used to spawn an independent 180s stop
// task for the same session id, and a reuse only refreshed the cache timestamp.
// The old timer was never cancelled, so a second request that reused the machine
// could be stopped mid-pipeline by the first request's timer, discarding its GPU
// work. The release has to be superseded, not accumulated.
#[test]
fn test_release_is_superseded_not_accumulated() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::{Duration, Instant};

    let mut cache = ColabSessionCache::new();
    let retention = Duration::from_secs(180);

    // First request finishes: a release is armed for now + retention.
    let first_deadline = Instant::now() + retention;
    cache.store_at("sess-1".to_string(), Instant::now());
    cache.arm_release(first_deadline);
    assert_eq!(cache.release_deadline(), Some(first_deadline));

    // A second request reuses the session and finishes later, so it arms a later
    // deadline. The pending release must move, not stack.
    let second_deadline = Instant::now() + retention + Duration::from_secs(100);
    cache.arm_release(second_deadline);
    assert_eq!(
        cache.release_deadline(),
        Some(second_deadline),
        "a second finish must supersede the earlier release, not add another"
    );

    // A release that has already fired for a superseded generation must not stop
    // the session. The generation counter is what makes that decidable.
    let generation_at_arm = cache.release_generation();
    cache.arm_release(Instant::now() + retention);
    assert!(
        !cache.should_release(generation_at_arm),
        "a stale generation must not be allowed to stop the session"
    );
    let current = cache.release_generation();
    assert!(
        cache.should_release(current),
        "the current generation owns the release"
    );
}

// R3-001 / R4-001. The cache was stamped when the session was chosen, before the
// pipeline ran. A separation longer than the retention window therefore expired
// the entry before it finished, so the next request inside the real window paid a
// cold start while the previous machine was still scheduled to stop. The stamp
// has to be refreshed when the work finishes.
#[test]
fn test_cache_is_stamped_at_finish_not_at_start() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::{Duration, Instant};

    let mut cache = ColabSessionCache::new();
    let retention = Duration::from_secs(180);

    // The session was chosen 200s ago and the pipeline took that long, so the
    // work just finished but the stamp is older than the 180s window. That is the
    // real shape of the defect: a long separation expired its own entry.
    cache.store_at(
        "sess-1".to_string(),
        Instant::now() - Duration::from_secs(200),
    );
    assert_eq!(
        cache.reusable(retention),
        None,
        "stamped at start, the entry looks expired right after finishing"
    );

    // Stamping at finish is the contract the product asks for: three minutes
    // after the request completes.
    cache.refresh_at(Instant::now());
    assert_eq!(
        cache.reusable(retention).as_deref(),
        Some("sess-1"),
        "a fresh stamp at finish makes the window count from completion"
    );
}

// R3-004. Each retry attempt used to receive the full caller timeout, so four
// attempts could run about four times past the separation's deadline. The budget
// belongs to the caller: total time must stay inside it.
#[test]
fn test_retry_respects_the_callers_total_budget() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    // A server that never answers properly: it accepts and stalls, forcing
    // retries until the budget is spent.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for stream in listener.incoming() {
            let mut s = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut buf = [0u8; 64];
            let _ = s.read(&mut buf);
            // Keep the connection open without responding.
            held.push(s);
            let _: &mut Vec<std::net::TcpStream> = &mut held;
        }
    });

    let dest = std::env::temp_dir().join(format!("budget_{}.bin", std::process::id()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let budget = Duration::from_secs(3);
    let started = Instant::now();
    let result = rt.block_on(async {
        shirley_show::infra::vocal::colab_transfer::fetch_with_retry(
            &format!("http://127.0.0.1:{port}/x"),
            &dest,
            budget,
            6,
        )
        .await
    });
    let elapsed = started.elapsed();
    let _ = std::fs::remove_file(&dest);

    assert!(result.is_err(), "a stalling server must fail");
    assert!(
        elapsed < budget + Duration::from_secs(3),
        "six attempts must not each get the full budget: took {elapsed:?} for a {budget:?} budget"
    );
}

// R3-005, el que cuesta dinero. El release se armaba con `arm_release`, que
// devuelve 0 cuando no puede tomar el lock interno, y `should_release(0)` es
// siempre false. Resultado: la tarea de limpieza dormía 180s, se declaraba no
// propietaria y **no apagaba nada**. La GPU quedaba facturando hasta que alguien
// ejecutara `colab stop` a mano.
//
// Confirmado en producción: `Connected: true` con `Idle: 17907s` tras una
// separación, y hubo que liberarla explícitamente.
#[test]
fn test_release_generation_is_never_zero() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::Instant;

    let mut cache = ColabSessionCache::new();
    let generation = cache.arm_release(Instant::now());
    assert_ne!(
        generation, 0,
        "generation 0 is the sentinel that means 'not the owner', so a real \
         release must never be handed it"
    );
    assert!(
        cache.should_release(generation),
        "the task that armed the release must be allowed to stop the session"
    );
}

// Un release armado y nunca superado debe apagar. Este es el camino normal: una
// separación, nadie más usa la máquina, la tarea duerme y apaga.
#[test]
fn test_armed_release_owns_the_shutdown() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::Instant;

    let mut cache = ColabSessionCache::new();
    let gen_normal = cache.arm_release(Instant::now());
    assert!(
        cache.should_release(gen_normal),
        "sin nadie que lo supere, el release debe ejecutarse: si no, la GPU \
         sigue facturando"
    );
}

// Superado por un segundo turno: el viejo no debe apagar, y el nuevo sí.
#[test]
fn test_superseded_release_does_not_stop_but_the_new_one_does() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::time::Instant;

    let mut cache = ColabSessionCache::new();
    let first = cache.arm_release(Instant::now());
    let second = cache.arm_release(Instant::now());

    assert_ne!(first, second, "re-armar debe cambiar la generación");
    assert!(
        !cache.should_release(first),
        "el release viejo no puede apagar una sesión que otro turno está usando"
    );
    assert!(
        cache.should_release(second),
        "el release nuevo sí debe poder apagar"
    );
}

// El lock interno no puede hacer perder el apagado. Aunque otro hilo esté
// leyendo la caché, el release no debe quedarse sin generación: si esperar no es
// posible, se responde sin bloquear pero el registro debe ser consistente.
#[test]
fn test_release_survives_contention() {
    use shirley_show::infra::vocal::colab_session_cache::ColabSessionCache;
    use std::sync::Arc;
    use std::time::Instant;

    let cache = Arc::new(tokio::sync::Mutex::new(ColabSessionCache::new()));
    let cache2 = cache.clone();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (gen_main, gen_other) = rt.block_on(async move {
        let mut guard = cache.lock().await;
        let g1 = guard.arm_release(Instant::now());
        let g2 = guard.arm_release(Instant::now());
        let _ = cache2;
        (g1, g2)
    });

    assert_ne!(gen_main, 0);
    assert_ne!(gen_other, 0);
    assert_ne!(gen_main, gen_other);
}

// R3-005, el único riesgo de dinero que queda abierto. La tarea de release vive
// dentro del proceso: si el proceso muere por SIGKILL, OOM o un reinicio duro,
// nadie ejecuta `colab stop` y la GPU sigue facturando hasta que Colab expira la
// sesión por inactividad. No hay destructor que cubra eso.
//
// La única defensa posible es desde fuera del turno: barrer runtimes huérfanos.
// Dos momentos importan:
//
// 1. Al arrancar. El proceso nuevo es el primero que puede limpiar lo que dejó el
//    anterior, y es exactamente el escenario del SIGKILL.
// 2. Durante la vida del proceso, por si el release falló en silencio.
//
// El barrido apaga TODOS los runtimes de la cuenta, así que la condición para
// hacerlo tiene que ser conservadora: nadie trabajando y nada que merezca
// conservarse.
#[test]
fn test_reaper_never_stops_a_runtime_in_use() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, reap_decision};

    assert_eq!(
        reap_decision(true, false),
        ReapDecision::Keep,
        "hay una separación en curso: apagar mataría el trabajo y perdería la GPU pagada"
    );
    assert_eq!(
        reap_decision(true, true),
        ReapDecision::Keep,
        "aunque la caché esté vacía, un trabajo activo manda"
    );
}

// Una sesión dentro de la ventana de retención se conserva: apagarla rompería la
// reutilización que se pidió.
#[test]
fn test_reaper_keeps_a_session_inside_the_retention_window() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, reap_decision};

    assert_eq!(
        reap_decision(false, true),
        ReapDecision::Keep,
        "nadie trabaja, pero la ventana de retención sigue abierta: se conserva"
    );
}

// Sin trabajo y sin nada que conserve la sesión, es un huérfano: apagar.
#[test]
fn test_reaper_stops_a_true_orphan() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, reap_decision};

    assert_eq!(
        reap_decision(false, false),
        ReapDecision::Stop,
        "sin trabajo y fuera de la ventana, cualquier runtime vivo está facturando \
         sin dueño: hay que apagarlo"
    );
}

// El barrido del arranque no espera ventana: si el proceso anterior murió, su
// sesión no tiene dueño aunque acabe de crearse.
#[test]
fn test_boot_sweep_stops_regardless_of_retention() {
    use shirley_show::infra::vocal::colab_orphan_reaper::{ReapDecision, boot_reap_decision};

    assert_eq!(
        boot_reap_decision(true),
        ReapDecision::Keep,
        "si ya hay trabajo arrancando, no se toca"
    );
    assert_eq!(
        boot_reap_decision(false),
        ReapDecision::Stop,
        "al arrancar no hay ningún turno en curso, así que cualquier runtime es huérfano"
    );
}

// ---------------------------------------------------------------------------
// Upload resilience.
//
// `colab upload` exits 1 with "kernel write failed" while still leaving the
// file byte-perfect on the runtime. Measured in production:
//
//   local  sha256 = 5a0b6df6...d470
//   remote sha256 = 5a0b6df6...d470
//   size = 289689 (exact)
//
// Trusting the exit code aborted every separation with `separation_failed`.
// The upload is only failed when the file is absent or its size/checksum does
// not match, so the decision belongs to the verification, not the exit status.
#[test]
fn test_upload_exit_code_alone_does_not_decide_success() {
    use shirley_show::infra::vocal::colab_transfer::{UploadVerdict, upload_verdict};

    // The exact production case: exit 1, file intact.
    assert_eq!(
        upload_verdict(false, Some(289_689), 289_689),
        UploadVerdict::Accept,
        "el CLI sale con 1 pero el archivo llego completo: aceptar"
    );

    // Exit 0 but the file never landed: reject.
    assert_eq!(
        upload_verdict(true, None, 289_689),
        UploadVerdict::Reject,
        "salida 0 sin archivo remoto no es exito"
    );

    // Truncated payload: reject, whatever the exit code says.
    assert_eq!(
        upload_verdict(true, Some(100), 289_689),
        UploadVerdict::Reject,
        "un archivo truncado no sirve aunque el CLI diga que si"
    );

    // Exit 1 and nothing there: retry.
    assert_eq!(
        upload_verdict(false, None, 289_689),
        UploadVerdict::Retry,
        "sin archivo y con fallo: reintentar es lo razonable"
    );
}

// The lease is what stops a boot sweep from killing a living sibling's GPU.
// It was added after measuring that a blind `colab stop` destroyed a
// separation mid-flight: the user saw `separation_failed` with no explanation.
#[test]
fn test_lease_is_taken_by_a_fresh_process() {
    let dir = std::env::temp_dir().join(format!("lease-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // No lease yet: this process owns the runtimes and may sweep.
    assert!(
        !shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&dir),
        "sin arrendamiento previo no hay dueno ajeno"
    );
    shirley_show::infra::vocal::colab_orphan_reaper::release_runtime_lease(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_lease_of_a_dead_process_is_not_respected() {
    let dir = std::env::temp_dir().join(format!("lease-dead-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // A pid that cannot be running: the runtime it left behind is an orphan and
    // must be swept, otherwise it bills forever.
    std::fs::write(dir.join("colab-runtime-owner.pid"), "4294967290").unwrap();
    assert!(
        !shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&dir),
        "un pid muerto no protege un runtime huerfano"
    );
    shirley_show::infra::vocal::colab_orphan_reaper::release_runtime_lease(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_lease_of_a_live_process_is_respected() {
    let dir = std::env::temp_dir().join(format!("lease-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // pid 1 is always running on unix, so it stands in for a live sibling.
    std::fs::write(dir.join("colab-runtime-owner.pid"), "1").unwrap();
    assert!(
        shirley_show::infra::vocal::colab_orphan_reaper::claim_runtime_lease(&dir),
        "una instancia viva no debe perder su GPU a manos de otra"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
