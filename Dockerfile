# syntax=docker/dockerfile:1

# Rust edition 2024 needs recent stable (1.85+).
FROM rust:1-bookworm AS chef
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install cargo-chef --locked

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home-dir /nonexistent --shell /usr/sbin/nologin appuser \
    && mkdir -p /data \
    && chown appuser:appuser /data

COPY --from=builder /app/target/release/token-api /usr/local/bin/token-api
COPY migrations /app/migrations

WORKDIR /data
USER appuser

ENV RUST_LOG=info \
    HOST=0.0.0.0 \
    PORT=8080 \
    WORKERS=2 \
    DATABASE_URL=file:/data/token-api.db

EXPOSE 8080

# Run SQL migrations against the same DATABASE_URL before first boot (e.g. `diesel migration run`
# from your machine, or a CI step), then start this container.

CMD ["token-api"]
