FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src
COPY config ./config

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and config
COPY --from=builder /app/target/release/solana-ingester /app/solana-ingester
COPY --from=builder /app/config /app/config

ENV CONFIG_PATH=/app/config/ingester.yaml
ENV RUST_LOG=info

EXPOSE 3001

CMD ["/app/solana-ingester"]
