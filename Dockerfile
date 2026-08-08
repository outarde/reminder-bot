FROM debian:trixie-slim

RUN apt-get update && apt_get install -y \
    ca-certificates libssl3 libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 appuser && \
    useradd -u 1000 -g appuser -s /bin/sh appuser

WORKDIR /app

ARG BINARY_PATH

COPY ${BINARY_PATH} /app/bot
COPY locales ./locales

RUN mkdir -p /app/data && chown -R appuser:appuser /app

ENV XDG_DATA_HOME=/app/data
ENV RUST_LOG=info

USER appuser

ENTRYPOINT ["/app/bot"]
