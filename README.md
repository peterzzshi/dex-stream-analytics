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

### Prerequisites

- Go 1.21+ for ingester and API services
- Java 17 + Maven for Flink aggregator (optional for local dev)
- Docker & Docker Compose for full stack
- WebSocket-enabled Polygon RPC endpoint (get free from [Alchemy](https://www.alchemy.com/) or [Infura](https://www.infura.io/))

### Setup

1. **Clone repository:**
   ```bash
   git clone https://github.com/peterzzshi/dex-stream-analytics
   cd dex-stream-analytics
   ```

2. **Run setup script:**
   ```bash
   ./setup.sh
   ```
   This creates `.env` file and downloads dependencies.

3. **Configure your API key:**
   ```bash
   # Edit .env with your actual API key
   nano .env
   
   # CRITICAL: Set POLYGON_RPC_URL to a WebSocket endpoint
   # Example: wss://polygon-mainnet.g.alchemy.com/v2/YOUR-API-KEY
   ```
   
   Get a free WebSocket endpoint from:
   - **Alchemy** (recommended): https://www.alchemy.com/
   - **Infura**: https://www.infura.io/

4. **Run services:**

   **Terminal 1 - Ingester:**
   ```bash
   ./run-ingester.sh
   ```

   **Terminal 2 - API:**
   ```bash
   ./run-api.sh
   ```

   **Or with Docker (full stack):**
   ```bash
   docker-compose up
   ```

5. **Verify it's working:**
   
   The ingester should show logs like:
   ```json
   {"level":"INFO","msg":"Configuration loaded"...}
   {"level":"INFO","msg":"Pair metadata loaded"...}
   {"level":"INFO","msg":"Swap event captured"...}
   ```

### Quick Commands

```bash
./setup.sh          # Initial setup (creates .env, downloads deps)
./run-ingester.sh   # Run ingester with .env config
./run-api.sh        # Run API with .env config

# Build binaries
cd ingester && go build -o bin/ingester ./cmd/ingester
cd api && go build -o bin/api ./cmd/api

# Run tests
cd ingester && go test ./...
cd api && go test ./...

# Docker
docker-compose up -d    # Start all services
docker-compose logs -f  # View logs
docker-compose down     # Stop all services
```

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