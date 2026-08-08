# syntax=docker/dockerfile:1
FROM rust:slim AS builder
WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/arb-core/Cargo.toml crates/arb-core/
COPY crates/arb-engine/Cargo.toml crates/arb-engine/
COPY crates/arb-server/Cargo.toml crates/arb-server/

RUN mkdir -p crates/arb-core/src && \
    mkdir -p crates/arb-engine/src && \
    mkdir -p crates/arb-server/src && \
    touch crates/arb-core/src/lib.rs && \
    touch crates/arb-engine/src/lib.rs && \
    touch crates/arb-server/src/lib.rs && \
    touch crates/arb-server/src/main.rs && \
    cargo build --locked --release --bin arb-server --workspace && \
    touch -d "2026-01-01T00:00:00Z" target/release/arb-server && \
    strip target/release/arb-server

COPY crates/arb-core/src crates/arb-core/src
COPY crates/arb-engine/src crates/arb-engine/src
COPY crates/arb-server/src crates/arb-server/src

RUN touch crates/arb-core/src/lib.rs && \
    touch crates/arb-engine/src/lib.rs && \
    touch crates/arb-server/src/lib.rs && \
    touch crates/arb-server/src/main.rs && \
    cargo build --locked --release --bin arb-server --workspace && \
    strip target/release/arb-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates tzdata curl && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd -r scanner && useradd -r -g scanner -d /app -s /usr/sbin/nologin scanner && \
    mkdir -p /app && chown scanner:scanner /app

USER scanner
WORKDIR /app

COPY --from=builder --chown=scanner:scanner /app/target/release/arb-server /app/arb-server
COPY --chown=scanner:scanner config.mainnet.toml /app/config.mainnet.toml

EXPOSE 8080
ENV RUST_LOG=info
HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
  CMD curl -sf http://localhost:8080/api/status || exit 1
ENTRYPOINT ["/app/arb-server", "--onchain", "--config", "/app/config.mainnet.toml"]
