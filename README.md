# dex-stream-analytics

Real-time DEX analytics pipeline processing blockchain swap events through windowed stream aggregation.

## Overview

This project demonstrates event-driven architecture with Apache Flink stream processing. It captures swap events from Polygon DEX contracts and aggregates them into analytics through 5-minute tumbling windows.

## Architecture

```
Polygon QuickSwap → Ingester (Go + DAPR) → Kafka → Aggregator (Flink + Java) → Kafka → API (Go + DAPR) → REST
```

**Key design:** Flink uses native Kafka connectors (not DAPR) to preserve exactly-once semantics and checkpointing.

### Components

- **ingester** - Go service listening to blockchain events via WebSocket
- **aggregator** - Flink job performing 5-minute windowed aggregations
- **api** - Go REST API exposing analytics data

### Technology Stack

- Go 1.21+ for ingester and API services
- Apache Flink 2.0.1 + Java 17 for stream processing
- Apache Kafka (KRaft mode) for messaging
- DAPR for service mesh (ingester and API only)
- Avro for schema serialisation
- Docker Compose for local development

## Quick Start

### Setup

1. Clone the repository
2. Copy environment file: `cp .env.example .env`
3. Start services: `docker-compose up -d`
4. Check status: `docker-compose ps`

### Accessing Services

- Analytics API: http://localhost:8080
- Flink Dashboard: http://localhost:8081
- Schema Registry: http://localhost:8082

## Project Structure

```
dex-stream-analytics/
├── ingester/          # Blockchain event listener (Go)
├── aggregator/        # Stream processor (Flink/Java)
├── api/               # REST API (Go)
├── contracts/         # Smart contracts (Solidity)
├── schemas/avro/      # Avro schemas
├── dapr/              # DAPR configuration
├── scripts/           # Setup and deployment scripts
└── docs/              # Documentation
```

## Development

### Building Services

```bash
# Ingester
cd ingester && go build -o bin/ingester cmd/ingester/main.go

# Aggregator
cd aggregator && mvn clean package

# API
cd api && go build -o bin/api cmd/api/main.go
```

### Running Tests

```bash
./scripts/test.sh
```

Or test individual services:

```bash
cd ingester && go test -v ./...
cd aggregator && mvn test
cd api && go test -v ./...
```

## Documentation

- [Setup Guide](docs/setup.md) - Detailed setup instructions
- [Architecture](docs/architecture.md) - System design and components
- [API Documentation](docs/api.md) - REST API reference

## Licence

MIT