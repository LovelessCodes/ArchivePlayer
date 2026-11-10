# syntax=docker/dockerfile:1

# ---- frontend -------------------------------------------------------------
FROM node:22-bookworm-slim AS web
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@12.11.2 --activate
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY index.html tsconfig.json tsconfig.app.json tsconfig.node.json vite.config.ts ./
COPY public ./public
COPY src ./src
RUN pnpm build

# ---- server ---------------------------------------------------------------
FROM rust:1.99-slim-bookworm AS server
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Fetch dependencies in their own layer so source edits do not rebuild the
# whole Rust dependency graph. Cargo resolves the workspace, so the Tauri
# manifest must exist even though this image only builds the server crate.
COPY Cargo.toml Cargo.lock ./
COPY crates/archiveplayer-core/Cargo.toml ./crates/archiveplayer-core/Cargo.toml
COPY crates/archiveplayer-server/Cargo.toml ./crates/archiveplayer-server/Cargo.toml
COPY src-tauri/Cargo.toml ./src-tauri/Cargo.toml
# Cargo needs a target for every workspace member while resolving the graph.
RUN mkdir -p crates/archiveplayer-core/src crates/archiveplayer-server/src src-tauri/src \
    && touch crates/archiveplayer-core/src/lib.rs \
             crates/archiveplayer-server/src/lib.rs \
             crates/archiveplayer-server/src/main.rs \
             src-tauri/src/lib.rs \
             src-tauri/build.rs \
    && cargo fetch --locked

COPY crates ./crates
COPY src-tauri/build.rs ./src-tauri/build.rs
COPY src-tauri/src ./src-tauri/src
COPY src-tauri/tauri.conf.json ./src-tauri/tauri.conf.json
COPY src-tauri/capabilities ./src-tauri/capabilities
COPY src-tauri/icons ./src-tauri/icons
RUN cargo build --release --locked -p archiveplayer-server

# ---- runtime --------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /app archiveplayer \
    && mkdir -p /data \
    && chown archiveplayer:archiveplayer /data

COPY --from=server /app/target/release/archiveplayer-server /usr/local/bin/archiveplayer-server
COPY --from=web /app/dist /app/dist

ENV ARCHIVEPLAYER_BIND=0.0.0.0:8080 \
    ARCHIVEPLAYER_DATA_DIR=/data \
    ARCHIVEPLAYER_STATIC_DIR=/app/dist \
    RUST_LOG=archiveplayer_server=info,tower_http=warn

VOLUME ["/data"]
EXPOSE 8080
USER archiveplayer
WORKDIR /app

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/api/health || exit 1

ENTRYPOINT ["/usr/local/bin/archiveplayer-server"]
