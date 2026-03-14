# DEX Stream Analytics

Real-time DEX analytics pipeline for Polygon QuickSwap events.

## What This Repo Does

- Ingests on-chain `Swap`, `Mint`, `Burn`, and `Transfer` events.
- Enriches events with metadata and pricing context.
- Publishes Avro event payloads via DAPR/Kafka.
- Aggregates trading and liquidity streams with Flink.
- Serves analytics through a Kotlin sink/API service.

## Documentation Ownership

- **This file (`README.md`)**
  - Project overview, prerequisites, quick start, and common run/test commands.
- **`ARCHITECTURE.md`**
  - Design decisions, dataflow tradeoffs, roadmap, and production hardening direction.
- **`DATA_MODEL.md`**
  - Field semantics and transformation logic.
- **Service READMEs**
  - `ingester/README.md`: ingestion behavior, enrichment, runtime config.
  - `aggregator/README.md`: stream topology, windows, Flink runtime notes.
  - `api/README.md`: Kotlin API endpoints and local run instructions.
  - `schemas/README.md`: Avro schema ownership and evolution rules.

## Repository Layout

- `ingester/` - Go blockchain ingester.
- `aggregator/` - Java/Flink stream processor.
- `api/` - Kotlin sink/API service.
- `schemas/avro/` - Canonical Avro schemas.
- `dapr/` - DAPR components and subscription manifests.

## Prerequisites

- Docker and Docker Compose
- Java 21
- Maven 3.9+ (aggregator)
- Go 1.21+ (ingester local development)
- Gradle 8+ (API local development)
- Polygon WebSocket RPC URL

## Quick Start

```bash
git clone https://github.com/peterzzshi/dex-stream-analytics
cd dex-stream-analytics
cp .env.example .env
# set POLYGON_RPC_URL in .env
docker compose up -d
```

## Common Operations

View logs:

```bash
docker compose logs -f ingester
docker compose logs -f aggregator
docker compose logs -f api
```

List Kafka topics:

```bash
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list
```

Consume trading events:

```bash
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-trading-events \
  --from-beginning
```

Consume liquidity events:

```bash
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-liquidity-events \
  --from-beginning
```

## Build and Test

Build:

```bash
cd ingester && go build -o ingester ./cmd/ingester
cd ../aggregator && mvn clean package -DskipTests
cd ../api && gradle build
```

Test:

```bash
cd ingester && go test ./...
cd ../aggregator && mvn test
cd ../api && gradle test
```

## Current Status

- Ingester: active
- Aggregator: in progress
- Kotlin sink/API: in progress
