# Build context is the repo root (compose: context: .)
# Stage 1: Build dependencies only (cached until Cargo.toml/Cargo.lock changes)
FROM rust:1.98-bookworm AS dependencies
WORKDIR /build
COPY common/Cargo.toml ./common/
COPY solana-ingester/Cargo.toml solana-ingester/Cargo.lock ./solana-ingester/
COPY schemas ./schemas

# Create dummy source files to build dependencies
RUN mkdir -p common/src solana-ingester/src && \
    echo "pub fn dummy() {}" > common/src/lib.rs && \
    echo "fn main() {}" > solana-ingester/src/main.rs

WORKDIR /build/solana-ingester
RUN cargo build --release
RUN rm -rf /build/common/src /build/solana-ingester/src

# Stage 2: Build actual application (only re-runs when source changes)
FROM rust:1.98-bookworm AS builder
WORKDIR /build
COPY --from=dependencies /build ./
COPY --from=dependencies /usr/local/cargo /usr/local/cargo

COPY common/src ./common/src
COPY solana-ingester/src ./solana-ingester/src

WORKDIR /build/solana-ingester
# Touch source files to force recompilation
RUN touch ../common/src/lib.rs && touch src/main.rs && cargo build --release

# Runtime stage - use same base as builder to ensure GLIBC compatibility
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/solana-ingester/target/release/solana-ingester /app/solana-ingester
COPY solana-ingester/solana-bridges.toml /app/solana-bridges.toml

EXPOSE 3001

CMD ["/app/solana-ingester"]
