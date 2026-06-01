# ── Stage 1: Build ──
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

# DO NOT add `ARG/ENV GIT_SHA` to this builder stage. GIT_SHA changes on
# every commit, so any layer below it is invalidated every deploy — that
# would bust the dependency-cache layer below and force a full ~2.5min
# `cargo build --release` of all deps on every single deploy. The binary
# reads GIT_SHA at *runtime* (core/config.rs: std::env::var("GIT_SHA")), so
# it only needs to exist in the runtime stage (below). Keep it there.

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
# `rift-macros` is a path-dep proc-macro crate — cargo needs its source
# (not just the manifest) to resolve the dep, so copy it upfront. The crate
# is tiny so cache-busting on rift-macros changes is fine.
COPY rift-macros ./rift-macros

# Cache registry dependencies by building a dummy main first
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs && cargo build --release && rm -rf src

COPY src ./src
# Touch main.rs so cargo knows it changed
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime ──
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 dnsutils && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rift /usr/local/bin/server

ARG GIT_SHA=unknown
ENV GIT_SHA=$GIT_SHA
ENV HOST=0.0.0.0
EXPOSE 3000

CMD ["server"]
