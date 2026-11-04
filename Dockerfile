# ---- Stage 1: builder ----
# The slim Rust image ships rustc + cargo; the C toolchain is installed
# explicitly because rusqlite is built with the `bundled` feature, which
# compiles SQLite from source.
FROM rust:1.95-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        gcc libc6-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Dependencies are compiled in their own layer: touching src/ does not force a
# full re-download and re-compile of the crate graph on every code change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release \
    && rm -rf src

# Real sources. `--locked` keeps the build reproducible from Cargo.lock.
# migrations/ must be present because migrate.rs embeds the SQL with
# include_str!, so the schema ships inside the binary and the runtime image
# does not need the directory. frontend/ is copied into the runtime image.
COPY src ./src
COPY migrations ./migrations
COPY frontend ./frontend
# The app ships plain ES modules with no bundler, so a build context that lost
# a subdirectory of frontend/ produces an image that builds cleanly and serves
# a blank page: the browser fails to resolve the missing import. Fail here
# instead of shipping it.
COPY scripts/verify-frontend-graph.sh ./scripts/verify-frontend-graph.sh
RUN sh ./scripts/verify-frontend-graph.sh frontend
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# ---- Stage 2: runtime ----
FROM debian:bookworm-slim

# ffmpeg/ffprobe: audio probing and conversion.
# yt-dlp: YouTube download. It is installed from pip rather than a distro
# package because the Debian one goes stale and YouTube breaks it.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ffmpeg \
        python3 python3-pip \
        openssh-client \
        sqlite3 \
        curl ca-certificates \
    && pip install --no-cache-dir --break-system-packages yt-dlp \
    && rm -rf /var/lib/apt/lists/*

# colab-cli drives the Google Colab GPU used for vocal separation. Without it
# every separation fails with colab_unavailable before spending GPU time.
ARG COLAB_CLI_VERSION=v0.2.0
RUN curl -fsSL -o /usr/local/bin/colab \
        "https://github.com/xeodou/colab-cli/releases/download/${COLAB_CLI_VERSION}/colab-linux-amd64" \
    && chmod +x /usr/local/bin/colab \
    && colab --help >/dev/null

# Non-root user. UID/GID 1000 matches the typical deploy host user, so a bind
# mount of the data directory keeps the right owner.
RUN groupadd -g 1000 shirley && useradd -u 1000 -g 1000 -m -s /bin/bash shirley

WORKDIR /app
COPY --from=builder /build/target/release/shirley-show /app/shirley-show
COPY --from=builder /build/frontend /app/frontend

# /app/data is created here and owned by shirley so a named volume inherits the
# ownership. A bind mount from the host must be chowned 1000:1000 by hand.
RUN mkdir -p /app/data /home/shirley/.ssh \
    && chown -R shirley:shirley /app /home/shirley

USER shirley

# The app must listen on 0.0.0.0 inside the container: bound to 127.0.0.1 it is
# unreachable from the host and the proxy answers 502.
ENV SHIRLEY_BIND=0.0.0.0:3001 \
    SHIRLEY_DATA_DIR=/app/data \
    SHIRLEY_FRONTEND_DIR=/app/frontend

EXPOSE 3001

# No healthcheck in the image: the panel reads `docker inspect .State.Status`,
# not whether the app answers. Kept here only for `docker run` users.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3001/api/presentations || exit 1

CMD ["/app/shirley-show"]
