# Build context is the repo root (compose: context: .)
FROM rust:latest AS builder
WORKDIR /build
COPY common ./common
COPY schemas ./schemas
COPY evm-ingester ./evm-ingester
WORKDIR /build/evm-ingester
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/evm-ingester/target/release/evm-ingester /app/evm-ingester
COPY evm.toml /app/evm.toml
CMD ["/app/evm-ingester"]
