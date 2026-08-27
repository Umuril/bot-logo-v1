FROM lukemathwalker/cargo-chef:latest-rust-slim AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY shared ./shared
COPY bot ./bot
COPY worker ./worker
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p bot
COPY Cargo.toml Cargo.lock ./
COPY shared ./shared
COPY bot ./bot
COPY worker ./worker
RUN cargo build --release -p bot

FROM debian:stable-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bot /usr/local/bin/bot

# Unlike bot-poll-v1's idle-container + Dokploy-Schedule-exec pattern (built for a one-shot
# cron job), this bot must stay running continuously: it holds an HTTP API open for the
# worker CLI to call at any time, so it IS the container's main process.
CMD ["/usr/local/bin/bot"]
