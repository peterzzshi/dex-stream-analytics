# Setup Guide

## Prerequisites

- Docker (20.10+) and Docker Compose (2.0+)
- Go (1.21+)
- Java (17+) and Maven (3.8+)
- Node.js (16+) for contract development

## Quick Start

### 1. Clone and Setup

```bash
git clone <repository-url>
cd dex-stream-analytics
chmod +x scripts/setup.sh
./scripts/setup.sh
```

### 2. Configure Environment

Create `.env` file with your configuration:

```bash
POLYGON_RPC_URL=https://polygon-rpc.com
PAIR_ADDRESS=0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827
LOG_LEVEL=info
```

### 3. Start Services

```bash
docker-compose up -d
```

This starts:
- Kafka (port 9092)
- Schema Registry (port 8082)
- DAPR placement service (port 50006)
- Ingester (port 3000)
- Flink (JobManager on port 8081)
- API (port 8080)

### 4. Verify

```bash
docker-compose ps
docker-compose logs -f ingester
docker-compose logs -f api
```

## Accessing Services

Once all services are running:

| Service | URL | Description |
|---------|-----|-------------|
| Analytics API | http://localhost:8080 | REST API for analytics |
## Accessing Services

| Service | URL | Description |
|---------|-----|-------------|
| Analytics API | http://localhost:8080 | REST API |
| API Health | http://localhost:8080/health | Health check |
| Flink Dashboard | http://localhost:8081 | Flink UI |
| Schema Registry | http://localhost:8082 | Avro schemas |
| Ingester Health | http://localhost:3000/health | Ingester check |

## Testing

### Check Ingester

```bash
curl http://localhost:3000/health
```

### View Analytics

```bash
curl http://localhost:8080/analytics/summary
curl http://localhost:8080/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/twap
```

### Monitor Flink

Open http://localhost:8081 to view the Flink dashboard.

## Development

### Building Locally

**Ingester:**
```bash
cd ingester
go build -o bin/ingester cmd/ingester/main.go
```

**Aggregator:**
```bash
cd aggregator
mvn clean package
```

**API:**
```bash
cd api
go build -o bin/api cmd/api/main.go
```

### Running Tests

```bash
./scripts/test.sh
```

## Troubleshooting

**Port conflicts:** Change ports in docker-compose.yml

**Kafka issues:** Ensure Docker has 4GB+ memory

**No events:** Check RPC URL and pair address

**Flink not running:** Verify JAR exists in aggregator/target/
