# Build context is the repo root (compose: context: .)
FROM rust:latest AS builder
WORKDIR /build
COPY common ./common
COPY schemas ./schemas
COPY solana-ingester ./solana-ingester
WORKDIR /build/solana-ingester
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/solana-ingester/target/release/solana-ingester /app/solana-ingester
COPY solana.toml /app/solana.toml

ENV RUST_LOG=info

EXPOSE 3001

CMD ["/app/solana-ingester"]
