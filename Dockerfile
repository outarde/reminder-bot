FROM rust:1.96-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY src ./src
# RUN cargo build --release --verbose

RUN rm -f /app/target/release/reminder-bot
RUN rm -rf /app/target/release/.fingerprint/reminder-bot-*

#
RUN cargo build --release --bin reminder-bot

#
RUN ls -lh /app/target/release/reminder-bot
RUN file /app/target/release/reminder-bot

# ----------
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 appuser && \
    useradd -u 1000 -g appuser -s /bin/sh appuser

WORKDIR /app

COPY --from=builder /app/target/release/reminder-bot /app/bot

#
RUN ls -lh /app/bot && file /app/bot

# app folder
RUN mkdir -p /app/data && chown -R appuser:appuser /app

# for dirs::data_dir
ENV XDG_DATA_HOME=/app/data
ENV RUST_LOG=info

USER appuser

ENTRYPOINT ["/app/bot"]