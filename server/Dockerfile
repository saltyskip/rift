# ── Stage 1: Build ──
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev perl make && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
# Cache dependencies by building a dummy main first
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs && cargo build --release && rm -rf src

COPY src ./src
# Touch main.rs so cargo knows it changed
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime ──
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/relay /usr/local/bin/server

ENV HOST=0.0.0.0
EXPOSE 3000

CMD ["server"]
