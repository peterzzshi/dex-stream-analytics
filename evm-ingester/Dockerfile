# Build stage
FROM rustlang/rust:nightly AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY schemas ./schemas
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/evm-ingester /app/evm-ingester
COPY config ./config
CMD ["/app/evm-ingester"]
