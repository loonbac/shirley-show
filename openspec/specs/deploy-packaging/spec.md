# Deploy Packaging Specification

## Purpose

Defines the reproducible Docker packaging, non-root runtime, persistent volume composition, healthcheck, and network routing invariants for deploying Shirley Show on Coolify.

## Requirements

### Requirement: Multi-Stage Dockerfile and Non-Root Runtime
1. Build MUST be multi-stage (`rust:1.85-slim-bookworm` builder, `debian:bookworm-slim` runtime).
2. Runtime MUST install `ffmpeg`, `ffprobe`, `yt-dlp`, `openssh-client`, `ca-certificates`, `curl`, `sqlite3`.
3. Process MUST run as user `shirley` UID/GID 1000; layout `/app/shirley-show`, `/app/frontend`, `/app/data`.
4. Image MUST NOT contain SSH keys, `.env` files, or API credentials.

#### Scenario: Image runs healthy as non-root
- GIVEN a built image
- WHEN the container starts
- THEN process runs as UID 1000 and `GET /api/presentations` healthcheck passes.

### Requirement: Persistent Storage and Volume Composition
1. Data MUST live in named volume `shirley-data:/app/data` (SQLite + audio + staging).
2. SSH key MUST mount read-only (`:ro`); envs map `SHIRLEY_*`, `GROQ_*`, `JOSHUA_PC_*`.
3. Recreating the container MUST preserve all records and audio.

#### Scenario: Data survives recreation
- GIVEN data in the volume
- WHEN the container is destroyed and recreated
- THEN presentations and audio remain accessible.

### Requirement: Healthcheck and Network Routing
1. Healthcheck: `curl -f http://localhost:3000/api/presentations`, 30s/5s/10s/3.
2. Primary topology: `network_mode: host` to reach tailnet; bridge/sidecar documented as fallback.

#### Scenario: Host networking reaches GPU node
- GIVEN Coolify host in tailnet with `network_mode: host`
- WHEN the app probes `100.73.247.39:22`
- THEN traffic routes via `tailscale0` without NAT.