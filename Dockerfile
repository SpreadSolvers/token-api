# syntax=docker/dockerfile:1

# Rust edition 2024 needs recent stable (1.85+).
# Official image: https://hub.docker.com/_/rust
FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install diesel_cli --no-default-features --features sqlite --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home-dir /nonexistent --shell /usr/sbin/nologin appuser \
    && mkdir -p /data \
    && chown appuser:appuser /data

COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel
COPY --from=builder /app/target/release/token-api /usr/local/bin/token-api
COPY migrations /app/migrations
COPY diesel.toml /app/diesel.toml
COPY diesel.docker.toml /app/diesel.docker.toml
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

WORKDIR /data
USER appuser

ENV RUST_LOG=debug \
    HOST=0.0.0.0 \
    PORT=8080 \
    WORKERS=2 \

EXPOSE 8080

ENTRYPOINT ["/entrypoint.sh"]
