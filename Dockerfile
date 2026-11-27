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

# google-colab-cli drives the Google Colab GPU used for vocal separation.
# Without it every separation fails with colab_unavailable before spending GPU
# time. It is Google's official CLI, pinned to 0.8.0, and it requires Python
# >= 3.12; bookworm ships 3.11, so uv provisions a 3.12 interpreter for it.
# The binary keeps the name `colab`.
# Install uv from its standalone installer and pin the CLI release.
# uv 0.12 rejects the old bin-directory flag on `tool install`;
# UV_TOOL_BIN_DIR is the supported way to put the binary on PATH.
# UV_TOOL_DIR keeps the tool environment out of /root (mode 700): otherwise
# /usr/local/bin/colab is a symlink that the shirley user (UID 1000) cannot follow.
# UV_PYTHON_INSTALL_DIR does the same for uv's managed Python 3.12: the colab
# shebang points at that interpreter, which UID 1000 could not reach in /root.
RUN curl -LsSf https://astral.sh/uv/install.sh | sh \
    && UV_TOOL_BIN_DIR=/usr/local/bin UV_TOOL_DIR=/usr/local/lib/uv-tools UV_PYTHON_INSTALL_DIR=/usr/local/lib/uv-python /root/.local/bin/uv tool install --python 3.12 \
        'google-colab-cli==0.8.0' \
    && colab version

# Non-root user. UID/GID 1000 matches the typical deploy host user, so a bind
# mount of the data directory keeps the right owner.
RUN groupadd -g 1000 shirley && useradd -u 1000 -g 1000 -m -s /bin/bash shirley

WORKDIR /app
COPY --from=builder /build/target/release/shirley-show /app/shirley-show
COPY --from=builder /build/frontend /app/frontend

# /app/data is created here and owned by shirley so a named volume inherits the
# ownership. A bind mount from the host must be chowned 1000:1000 by hand.
# The official CLI reads its legacy token from ~/.config/colab-cli/token.json
# and caches sessions in ~/.config/colab-cli/sessions.json, so that directory
# must exist and be owned by the runtime user (UID 1000).
RUN mkdir -p /app/data /home/shirley/.ssh /home/shirley/.config/colab-cli \
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
