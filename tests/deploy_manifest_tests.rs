use std::fs;
use std::path::Path;

#[test]
fn test_dockerfile_multi_stage_and_healthcheck() {
    let content = fs::read_to_string("Dockerfile").expect("read Dockerfile");
    assert!(
        content.contains("FROM rust:1.85-slim-bookworm AS builder"),
        "Dockerfile must have rust builder stage"
    );
    assert!(
        content.contains("FROM debian:bookworm-slim"),
        "Dockerfile must have debian slim runtime stage"
    );
    assert!(
        content.contains("useradd -u 1000 -g 1000 -m -s /bin/bash shirley"),
        "Dockerfile must create non-root user shirley:1000"
    );
    assert!(
        content.contains("USER shirley"),
        "Dockerfile must run as USER shirley"
    );
    assert!(
        content.contains("--start-period=10s"),
        "HEALTHCHECK must define --start-period=10s"
    );
    assert!(
        content.contains("http://localhost:3000/api/presentations"),
        "HEALTHCHECK must probe /api/presentations"
    );
}

#[test]
fn test_dockerignore_whitelist() {
    let content = fs::read_to_string(".dockerignore").expect("read .dockerignore");
    assert!(content.contains(".git/"), "Must ignore .git/");
    assert!(content.contains("secrets/"), "Must ignore secrets/");
    assert!(
        content.contains("*.sqlite3"),
        "Must ignore sqlite database files"
    );
    assert!(
        content.contains("*id_ed25519*"),
        "Must ignore private ssh keys"
    );
}

#[test]
fn test_compose_yaml_structure_and_defaults() {
    let content = fs::read_to_string("compose.yaml").expect("read compose.yaml");
    assert!(
        content.contains("network_mode: host"),
        "compose.yaml must specify network_mode: host"
    );
    assert!(
        content.contains("shirley-data:/app/data"),
        "compose.yaml must mount persistent volume"
    );
    assert!(
        content.contains(":ro"),
        "compose.yaml must mount SSH key as read-only (:ro)"
    );
    assert!(
        content.contains("JOSHUA_PC_HOST=${JOSHUA_PC_HOST:-}"),
        "compose.yaml must default JOSHUA_PC_HOST to empty"
    );
}

#[test]
fn test_deployment_runbook_sudoers_validation_command() {
    let content =
        fs::read_to_string("docs/deployment-coolify.md").expect("read deployment-coolify.md");
    assert!(
        content.contains("sudo -n /run/current-system/sw/bin/shutdown -c"),
        "Runbook must validate sudoers rule using 'sudo -n /path/shutdown -c' not 'sudo -n true'"
    );
    assert!(
        !content.contains("sudo -n true"),
        "Runbook must NOT recommend generic 'sudo -n true'"
    );
}

#[test]
fn test_manifests_consistency() {
    assert!(Path::new("Dockerfile").exists());
    assert!(Path::new(".dockerignore").exists());
    assert!(Path::new("compose.yaml").exists());
    assert!(Path::new("docs/deployment-coolify.md").exists());
}
