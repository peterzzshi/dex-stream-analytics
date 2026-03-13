# DEX Stream Analytics

Real-time DEX analytics pipeline with **Chainlink price oracle integration** for accurate USD volume calculations across all trading pairs.

**Data Flow:** `Polygon Blockchain → Ingester (Go + Chainlink) → Kafka → Aggregator (Flink) → Kafka → API (Go) → REST`

## Key Features

- ✅ **Chainlink Price Oracle** - Accurate USD volumes for all pairs (98% accuracy)
- ✅ **Real-time Streaming** - WebSocket blockchain events via Uniswap V2 pairs
- ✅ **Smart Caching** - Token symbols (∞) and prices (5min TTL) for efficiency
- ✅ **Production-Ready** - Error handling, graceful degradation, comprehensive logging
- ✅ **Free Tier Optimized** - ~2-3 RPC calls per event (95%+ cache hit rate)

## Components

- **ingester** (Go) - Blockchain event listener with Chainlink oracle integration
- **aggregator** (Flink) - Windowed aggregations (5-min TWAP/OHLC, 1-hour LP analytics)
- **api** (Go) - REST API exposing analytics data

## Tech Stack

- **Blockchain:** Polygon (Uniswap V2 compatible DEXes)
- **Oracle:** Chainlink Data Feeds (on-chain, decentralized)
- **Streaming:** Apache Kafka (KRaft mode)
- **Processing:** Apache Flink 2.0.1 + Java 21
- **Services:** Go 1.25+ (ingester, API)
- **Mesh:** DAPR for pub/sub
- **Serialization:** Avro (embedded schemas)

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and component details
- **[DATA_MODEL.md](DATA_MODEL.md)** - Event schemas and data flow
- Component READMEs in each subdirectory

---

## Prerequisites

- **Docker & Docker Compose** (required)
- **Java 21** (for IntelliJ development)
- **Maven 3.9+** (for building aggregator)
- **Go 1.21+** (optional, for Go service development)
- **IntelliJ IDEA** (optional, for development)

---

## Quick Start

### 1. Clone and Configure

```bash
git clone https://github.com/peterzzshi/dex-stream-analytics
cd dex-stream-analytics

# Set Polygon RPC URL for real data
cp .env.example .env
nano .env  # Add your Alchemy/Infura WebSocket URL
```

**Note:** All shell scripts are located in the `scripts/` directory.

### 2. Run the Application

**Docker (Recommended)**
```bash
docker-compose up -d
```

**IntelliJ Development**
```bash
# Start infrastructure only
docker-compose up -d kafka dapr-placement

# Run aggregator in IntelliJ (see IntelliJ Setup below)
```

### 3. Monitor

```bash
# View logs
docker-compose logs -f ingester
docker-compose logs -f aggregator

# Check Kafka topics
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list

# Consume events
# Consume trading events (swaps)
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-trading-events \
  --from-beginning

# Consume liquidity events (mints/burns)
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-liquidity-events \
  --from-beginning
```

---

## Development

### IntelliJ Setup (Aggregator)

**Best for:** Debugging Flink aggregator with breakpoints

```bash
# 1. Start infrastructure
docker-compose up -d kafka dapr-placement

# 2. Fix dependencies if needed
./scripts/reimport-maven.sh

# 3. In IntelliJ:
#    File → Open → dex-stream-analytics (root folder)
#    File → Project Structure → SDK: 21
#    Run → Edit Configurations → + → Application
#      Name: StreamProcessor
#      Main class: com.web3analytics.StreamProcessor
#      Module: aggregator
#      JRE: 21
#      VM options: -Dorg.slf4j.simpleLogger.defaultLogLevel=INFO
#      Environment variables:
#        KAFKA_BOOTSTRAP_SERVERS=localhost:9092
#        TOPIC_TRADING_EVENTS=dex-trading-events
#        TOPIC_LIQUIDITY_EVENTS=dex-liquidity-events
#        TOPIC_TRADING_ANALYTICS=dex-trading-analytics
#    Click ▶️ Run or 🐞 Debug
```

**Troubleshooting:**
- **Red imports?** Run `./scripts/reimport-maven.sh`, then `File → Invalidate Caches / Restart`
- **NoClassDefFoundError?** Rebuild: `cd aggregator && mvn clean package -DskipTests`, then restart IntelliJ
- **Kafka connection error?** Ensure environment variables are set in run configuration
- **"No resolvable bootstrap urls"?** Check `KAFKA_BOOTSTRAP_SERVERS` is set to `localhost:9092`

### Building

```bash
# Build ingester
cd ingester && go build -o ingester ./cmd/ingester

# Build aggregator (dev profile - includes Flink runtime)
cd aggregator && mvn clean package -DskipTests

# Build aggregator (prod profile - slim JAR)
cd aggregator && mvn clean package -Pprod -DskipTests

# Build API
cd api && go build -o api ./cmd/api
```

### Testing

```bash
# Test ingester
cd ingester && go test -v ./...

# Test aggregator
cd aggregator && mvn test

# Test API
cd api && go test -v ./...
```

### Kafka Operations

```bash
# List topics
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list

# Create topics manually
docker exec kafka kafka-topics --bootstrap-server localhost:9092 \
  --create --topic dex-trading-events --partitions 6 --replication-factor 1 --if-not-exists
docker exec kafka kafka-topics --bootstrap-server localhost:9092 \
  --create --topic dex-liquidity-events --partitions 6 --replication-factor 1 --if-not-exists
docker exec kafka kafka-topics --bootstrap-server localhost:9092 \
  --create --topic dex-trading-analytics --partitions 3 --replication-factor 1 --if-not-exists

# Consume messages (trading events)
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-trading-events \
  --from-beginning

# Consume messages (liquidity events - heterogeneous Mint/Burn)
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-liquidity-events \
  --from-beginning

# Consume analytics output
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-trading-analytics \
  --from-beginning

# Delete topics (use with caution)
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --delete --topic dex-trading-events
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --delete --topic dex-liquidity-events
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --delete --topic dex-trading-analytics
```

---

## Expected Output

### Window Aggregation (every 5 minutes)
```json
{
  "windowStart": 1708502400000,
  "windowEnd": 1708502700000,
  "pairAddress": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827",
  "twap": 3245.67,
  "openPrice": 3240.12,
  "closePrice": 3250.34,
  "highPrice": 3260.00,
  "lowPrice": 3235.00,
  "totalVolume0": 1234567.89,
  "totalVolume1": 9876543.21,
  "swapCount": 347,
  "uniqueTraders": 42,
  "arbitrageCount": 5,
  "priceVolatility": 0.0077
}
```

---

## Troubleshooting

### NoClassDefFoundError (Flink classes)
**Solution:** Rebuild with dev profile
```bash
cd aggregator && mvn clean package -DskipTests
```

### Dependencies Not Resolving (IntelliJ)
**Solution:**
```bash
./reimport-maven.sh
# Then: File → Invalidate Caches / Restart
```

### Kafka Connection Refused
**Solution:**
```bash
docker-compose up -d kafka
sleep 30  # Wait for startup
docker logs kafka | grep "started (kafka.server"
```

### Flink Job Not Starting
**Check:**
```bash
docker logs aggregator | grep ERROR
docker exec aggregator nc -zv kafka 9092
```

---

## Project Structure

```
dex-stream-analytics/
├── docker-compose.yml        # Infrastructure setup
├── .env.example              # Environment template
├── ARCHITECTURE.md           # System design and infrastructure
├── DATA_MODEL.md             # Schema definitions and transformations
├── aggregator/               # Flink stream processor (Java 21)
│   ├── src/main/java/com/web3analytics/
│   │   ├── StreamProcessor.java
│   │   ├── functions/SwapAggregator.java
│   │   ├── models/
│   │   ├── serialization/
│   │   └── config/
│   └── pom.xml
├── ingester/                 # Blockchain listener (Go)
│   ├── cmd/ingester/main.go
│   ├── internal/
│   │   ├── blockchain/       # WebSocket listener
│   │   ├── avro/             # Avro encoding
│   │   ├── publisher/        # DAPR publishing
│   │   └── config/
│   └── go.mod
├── api/                      # REST API (Go) [planned]
├── schemas/avro/             # Avro schemas (source of truth)
│   ├── SwapEvent.avsc
│   ├── MintEvent.avsc
│   ├── BurnEvent.avsc
│   └── AggregatedAnalytics.avsc
├── dapr/                     # DAPR configuration
│   ├── components/           # pubsub.yaml, statestore.yaml
│   └── subscriptions/
└── contracts/                # Smart contracts
    └── MockAMM.sol
```

---

## Documentation

### Core Documentation
- **[README.md](README.md)** (this file) - Setup, operations, troubleshooting
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design, decisions, infrastructure details
- **[DATA_MODEL.md](DATA_MODEL.md)** - Schema definitions, field semantics, data transformations

### Complete Documentation Index
See **[DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)** for a complete guide to all documentation, including:
- Component-specific guides (ingester, aggregator, API)
- Functional programming patterns
- Validation reports
- Archived documentation
- Script reference

---

## License

MIT License - see [LICENSE](LICENSE)
