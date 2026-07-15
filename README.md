# DEX Stream Analytics

Real-time analytics pipeline for Polygon QuickSwap DEX events, demonstrating three pillars: **Dapr** as producer/consumer glue, **Apache Flink** with four window types, and a **polyglot stack** (Go, Java 21, Kotlin).

## Repository Layout

| Directory | Language | Purpose |
|---|---|---|
| `ingester/` | Go | Blockchain event listener + Chainlink enrichment |
| `aggregator/` | Java 21 / Flink 2 | Stream processing with 4 window pipelines |
| `analytics-service/` | Kotlin / Ktor | REST API, pool health scoring, Redis storage |
| `schemas/avro/` | — | Canonical Avro schema definitions |
| `dapr/` | — | Dapr component and subscription manifests |

## Documentation

| File | Scope |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design, data flow, design decisions, infrastructure |
| [DATA_MODEL.md](DATA_MODEL.md) | Schema field semantics, data quality, evolution rules |
| `ingester/README.md` | Ingestion behaviour, enrichment, runtime config |
| `aggregator/README.md` | Stream topology, window types, Flink runtime |
| `analytics-service/README.md` | Endpoints, pool health scoring, local run |
| `schemas/README.md` | Schema ownership and change workflow |

## Prerequisites

- Docker & Docker Compose
- Java 21, Maven 3.9+ (aggregator)
- Go 1.21+ (ingester)
- Gradle 8+ (analytics-service)
- Polygon WebSocket RPC URL

## Quick Start

```bash
git clone https://github.com/peterzzshi/dex-stream-analytics
cd dex-stream-analytics
cp .env.example .env   # set POLYGON_RPC_URL
docker compose up -d
```

## Build & Test

```bash
# Ingester
cd ingester && go build -o ingester ./cmd/ingester && go test ./...

# Aggregator (55 tests)
cd aggregator && mvn test

# Analytics Service (29 tests)
cd analytics-service && ./gradlew test
```

## Common Operations

```bash
# Service logs
docker compose logs -f ingester aggregator analytics-service

# List Kafka topics
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list

# Consume events
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-trading-events --from-beginning
```
