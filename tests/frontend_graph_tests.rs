use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

const GUARD: &str = "scripts/verify-frontend-graph.sh";

fn run_guard(frontend_dir: &Path) -> std::process::Output {
    Command::new("sh")
        .arg(GUARD)
        .arg(frontend_dir)
        .output()
        .expect("run the frontend graph guard")
}

/// A build context that lost frontend/js/data/ builds a valid image and serves
/// a blank page, because the browser cannot resolve the import. The guard must
/// catch it.
#[test]
fn test_guard_rejects_context_missing_an_imported_module() {
    let temp = TempDir::new().expect("temp dir");
    let js_dir = temp.path().join("js");
    fs::create_dir_all(js_dir.join("data")).expect("create js/data");
    fs::create_dir_all(temp.path().join("styles")).expect("create styles");
    fs::write(temp.path().join("index.html"), "<div id=\"app\"></div>").expect("index.html");
    fs::write(temp.path().join("styles/index.css"), "").expect("index.css");
    fs::write(
        js_dir.join("main.js"),
        "import { list } from './data/list-presentations.js';\n",
    )
    .expect("main.js");

    // Complete context: the import resolves.
    fs::write(js_dir.join("data/list-presentations.js"), "export const list = 1;\n")
        .expect("module");
    let complete = run_guard(temp.path());
    assert!(
        complete.status.success(),
        "a complete context must pass: {}",
        String::from_utf8_lossy(&complete.stderr)
    );

    // Same context after the directory was dropped, which is what an unanchored
    // data/ exclusion does to the build tree.
    fs::remove_file(js_dir.join("data/list-presentations.js")).expect("remove module");
    let broken = run_guard(temp.path());
    assert!(
        !broken.status.success(),
        "a context missing an imported module must fail the guard"
    );
    let stderr = String::from_utf8_lossy(&broken.stderr);
    assert!(
        stderr.contains("list-presentations.js"),
        "the failure must name the missing module, got: {stderr}"
    );
}

/// The entry point and the stylesheet are what every screen needs; a context
/// without them must not reach the build either.
#[test]
fn test_guard_rejects_context_without_entry_point() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir_all(temp.path().join("js")).expect("create js");
    fs::create_dir_all(temp.path().join("styles")).expect("create styles");
    fs::write(temp.path().join("index.html"), "<div id=\"app\"></div>").expect("index.html");
    fs::write(temp.path().join("styles/index.css"), "").expect("index.css");

    let output = run_guard(temp.path());
    assert!(
        !output.status.success(),
        "a context without js/main.js must fail the guard"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("main.js"),
        "the failure must name the missing entry point"
    );
}

/// The repository's own frontend must satisfy the guard, otherwise the build
/// fails for everyone.
#[test]
fn test_repository_frontend_graph_is_complete() {
    let output = run_guard(Path::new("frontend"));
    assert!(
        output.status.success(),
        "the repository frontend must pass its own guard: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Anchoring the .dockerignore patterns is a precaution, not the fix for the
/// blank-page incident: Docker matches these against the full path relative to
/// the context root, so a bare "data/" only ever excluded the top-level
/// directory. It is pinned because the anchoring costs nothing and keeps the
/// intent obvious; the real protection against a truncated context is the guard
/// asserted below.
#[test]
fn test_dockerignore_anchors_runtime_directories() {
    let content = fs::read_to_string(".dockerignore").expect("read .dockerignore");
    for pattern in ["data/", "production-data/", "audio/", "posters/"] {
        let anchored = format!("/{pattern}");
        let bare = content.lines().any(|line| line.trim() == pattern);
        assert!(
            !bare,
            ".dockerignore must not contain the unanchored pattern '{pattern}': it \
             also matches a nested directory such as frontend/js/data/"
        );
        assert!(
            content.lines().any(|line| line.trim() == anchored),
            ".dockerignore must anchor '{pattern}' as '{anchored}'"
        );
    }
}

/// The guard only protects production if the build actually runs it.
#[test]
fn test_dockerfile_runs_the_frontend_graph_guard() {
    let content = fs::read_to_string("Dockerfile").expect("read Dockerfile");
    assert!(
        content.contains(GUARD),
        "Dockerfile must run {GUARD} so a truncated context cannot be shipped"
    );

    let guard_position = content.find(GUARD).expect("guard reference");
    let build_position = content
        .find("cargo build --release --locked")
        .expect("the real release build");
    assert!(
        guard_position < build_position,
        "the guard must run before the release build, not after it"
    );
}

/// scripts/ must reach the build context, otherwise the guard cannot run.
#[test]
fn test_guard_script_ships_into_the_build_context() {
    assert!(
        PathBuf::from(GUARD).exists(),
        "{GUARD} must exist in the repository"
    );
    let content = fs::read_to_string(".dockerignore").expect("read .dockerignore");
    assert!(
        !content.lines().any(|line| line.trim().trim_end_matches('/') == "scripts"),
        ".dockerignore must not exclude scripts/: the Dockerfile runs the guard from there"
    );
}

/// This one fixes a real defect, and a different one from the incident: git,
/// unlike Docker, applies ignore patterns at any depth, so a bare "data/" hides
/// a new module dropped into frontend/js/data/. It never appears in git status
/// and never gets committed. The modules lost in the incident were all tracked;
/// this pattern would have eroded the tree over time instead.
#[test]
fn test_gitignore_anchors_runtime_directories() {
    let content = fs::read_to_string(".gitignore").expect("read .gitignore");
    for pattern in ["data/", "production-data/"] {
        let anchored = format!("/{pattern}");
        let bare = content.lines().any(|line| line.trim() == pattern);
        assert!(
            !bare,
            ".gitignore must not contain the unanchored pattern '{pattern}': it \
             also hides a nested directory such as frontend/js/data/"
        );
        assert!(
            content.lines().any(|line| line.trim() == anchored),
            ".gitignore must anchor '{pattern}' as '{anchored}'"
        );
    }
}

