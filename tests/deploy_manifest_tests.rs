use std::fs;
use std::path::Path;

/// Extracts the port from the first `EXPOSE <port>` directive.
fn dockerfile_exposed_port(content: &str) -> Option<u16> {
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("EXPOSE "))
        .and_then(|port| port.trim().parse::<u16>().ok())
}

/// Extracts the port from the HEALTHCHECK probe URL.
fn dockerfile_healthcheck_port(content: &str) -> Option<u16> {
    content
        .lines()
        .find_map(|line| line.split("localhost:").nth(1))
        .and_then(|rest| rest.split('/').next())
        .and_then(|port| port.trim().parse::<u16>().ok())
}

/// Extracts the port from the compose `SHIRLEY_BIND=0.0.0.0:<port>` entry.
fn compose_bind_port(content: &str) -> Option<u16> {
    content
        .lines()
        .find_map(|line| line.split("SHIRLEY_BIND=0.0.0.0:").nth(1))
        .and_then(|rest| rest.trim().parse::<u16>().ok())
}

#[test]
fn test_dockerfile_multi_stage_and_healthcheck() {
    let content = fs::read_to_string("Dockerfile").expect("read Dockerfile");
    // Versión-agnóstico: el builder debe ser una imagen oficial de Rust con stage
    // nombrado, para que subir la versión de Rust no rompa la verificación.
    let has_rust_builder_stage = content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("FROM rust:") && trimmed.ends_with("AS builder")
    });
    assert!(
        has_rust_builder_stage,
        "Dockerfile must have an official Rust builder stage named 'builder'"
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
        content.contains("/api/presentations"),
        "HEALTHCHECK must probe /api/presentations"
    );

    // Consistencia real de puertos: EXPOSE == HEALTHCHECK, en vez de fijar 3000 a mano.
    let exposed = dockerfile_exposed_port(&content).expect("Dockerfile must EXPOSE a port");
    let probed =
        dockerfile_healthcheck_port(&content).expect("HEALTHCHECK must probe a localhost port");
    assert_eq!(
        exposed, probed,
        "HEALTHCHECK port ({probed}) must match EXPOSE port ({exposed})"
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
    // La topología puede ser host o bridge+ports; debe estar declarada explícitamente.
    let uses_host_network = content
        .lines()
        .any(|line| line.trim() == "network_mode: host");
    let publishes_port = content.lines().any(|line| line.trim() == "ports:");
    assert!(
        uses_host_network ^ publishes_port,
        "compose.yaml must declare exactly one network strategy: host mode or published ports"
    );
    assert!(
        content.contains("shirley-data:/app/data"),
        "compose.yaml must mount persistent volume"
    );

    // The Colab token must be mounted AND writable.
    //
    // This assertion used to demand `:ro` in the file, with the message "must
    // mount SSH key as read-only". It lost its subject in 1f79429 ("mount the
    // Colab token read-write and document it"), which removed the id_ed25519
    // mount together with its `:ro`, because the SSH backend had been replaced
    // by Colab. From then on the only `:ro` left in the file was the Colab
    // token's, so the assertion started failing for a reason its message did
    // not describe.
    //
    // "Fixing" it by adding `:ro` back is the trap: colab-cli rewrites that file
    // when an access_token expires, so a read-only mount makes every separation
    // fail once the hour is up -- silently, in production, on a schedule. The
    // compose is right and the assertion was wrong.
    //
    // So this pins the real invariant: the token is mounted, and it is NOT
    // read-only. A future regression that adds `:ro` back now fails here,
    // by name, instead of breaking the product an hour into the event.
    // Match the MOUNT, not the environment variable that names the same path.
    // `SHIRLEY_COLAB_TOKEN=...:-/home/shirley/...` contains the path too, and it
    // is the first match in the file, so searching for the bare path selected the
    // env var and the assertion could never fail. A mount has "host:container";
    // the env var has ":-" in between. The leading colon is the discriminator.
    let token_mount = content
        .lines()
        .find(|line| line.contains(":/home/shirley/.config/colab/token.json"))
        .expect("compose.yaml must mount the Colab OAuth token");
    assert!(
        !token_mount.trim_end().ends_with(":ro"),
        "the Colab token must stay read-write: colab-cli rewrites it when the \
         access_token expires, and a read-only mount breaks separation hourly. \
         Mount in question: {token_mount}"
    );
    assert!(
        content.contains("JOSHUA_PC_HOST=${JOSHUA_PC_HOST:-}"),
        "compose.yaml must default JOSHUA_PC_HOST to empty"
    );
}

#[test]
fn test_container_port_consistency_across_manifests() {
    let dockerfile = fs::read_to_string("Dockerfile").expect("read Dockerfile");
    let compose = fs::read_to_string("compose.yaml").expect("read compose.yaml");

    let exposed = dockerfile_exposed_port(&dockerfile).expect("Dockerfile must EXPOSE a port");
    let compose_port =
        compose_bind_port(&compose).expect("compose.yaml must bind SHIRLEY_BIND to a port");

    assert_eq!(
        exposed, compose_port,
        "Dockerfile EXPOSE port ({exposed}) must match compose SHIRLEY_BIND port ({compose_port})"
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
