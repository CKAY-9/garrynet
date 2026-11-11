# syntax=docker/dockerfile:1

# ---- Build ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Build dependencies first against a stub crate so they stay cached until
# Cargo.toml / Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked

# Real sources. Migrations are embedded in the binary by `sqlx::migrate!`, and the
# default legal documents by `include_str!`.
# `touch` makes cargo see main.rs as newer than the stub it just compiled.
COPY src ./src
COPY migrations ./migrations
COPY legal ./legal
RUN touch src/main.rs && cargo build --release --locked

# ---- Runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --shell /usr/sbin/nologin garrynet

# Uploads with the local storage provider (STORAGE_LOCAL_PATH=./storage) land in
# /app/storage. docker-compose.yml mounts a volume here; a new volume copies this
# directory's ownership, so the non-root user can write to it.
WORKDIR /app
RUN mkdir storage && chown garrynet:garrynet storage

COPY --from=builder /app/target/release/backend /usr/local/bin/backend

USER garrynet
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
CMD ["backend"]
