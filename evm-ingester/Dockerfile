# Build context is the repo root (compose: context: .)
# Stage 1: Build dependencies only (cached until Cargo.toml/Cargo.lock changes)
FROM rust:1.98-bookworm AS dependencies
WORKDIR /build
COPY common/Cargo.toml ./common/
COPY evm-ingester/Cargo.toml evm-ingester/Cargo.lock ./evm-ingester/
COPY schemas ./schemas

# Create dummy source files to build dependencies
RUN mkdir -p common/src evm-ingester/src && \
    echo "pub fn dummy() {}" > common/src/lib.rs && \
    echo "fn main() {}" > evm-ingester/src/main.rs

WORKDIR /build/evm-ingester
RUN cargo build --release
RUN rm -rf /build/common/src /build/evm-ingester/src

# Stage 2: Build actual application (only re-runs when source changes)
FROM rust:1.98-bookworm AS builder
WORKDIR /build
COPY --from=dependencies /build ./
COPY --from=dependencies /usr/local/cargo /usr/local/cargo

COPY common/src ./common/src
COPY evm-ingester/src ./evm-ingester/src

WORKDIR /build/evm-ingester
# Touch real sources to force recompilation; common must be touched too, because the
# dependency stage's dummy lib.rs has a newer mtime and would otherwise be treated as fresh.
RUN touch ../common/src/lib.rs && touch src/main.rs && cargo build --release

# Runtime stage - use same base as builder to ensure GLIBC compatibility
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/evm-ingester/target/release/evm-ingester /app/evm-ingester
COPY evm-ingester/evm-bridges.toml /app/evm-bridges.toml
CMD ["/app/evm-ingester"]
