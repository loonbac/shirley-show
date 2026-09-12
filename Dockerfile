# ---- Stage 1: builder ----
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY frontend ./frontend
RUN cargo build --release

# ---- Stage 2: runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ffmpeg \
        python3 python3-pip \
        openssh-client \
        sqlite3 \
        curl ca-certificates \
    && pip install --no-cache-dir --break-system-packages yt-dlp \
    && rm -rf /var/lib/apt/lists/*

# Non-root user (UID/GID 1000 matches typical Coolify host user)
RUN groupadd -g 1000 shirley && useradd -u 1000 -g 1000 -m -s /bin/bash shirley
WORKDIR /app
COPY --from=builder /build/target/release/shirley-show /app/shirley-show
COPY --from=builder /build/frontend /app/frontend
COPY --from=builder /build/migrations /app/migrations
RUN mkdir -p /app/data && chown -R shirley:shirley /app
USER shirley

ENV SHIRLEY_BIND=0.0.0.0:3001 \
    SHIRLEY_DATA_DIR=/app/data \
    SHIRLEY_FRONTEND_DIR=/app/frontend

EXPOSE 3001
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3001/api/presentations || exit 1

CMD ["/app/shirley-show"]
