# DEX Stream Analytics

Real-time analytics for DeFi liquidity pools — detect MEV attacks, score pool health, and monitor market trends with sub-3-minute latency.

## Problem

Traders lose 2-5% to sandwich attacks with no real-time warning. LPs can't identify toxic pools before committing capital. Existing tools (Dune, DEXTools) have 5-15 min lag and lack cross-chain comparison.

## Solution

Multi-chain Apache Flink pipeline processing live DEX events across Polygon, Arbitrum, and Base:
- **MEV detection**: Sandwich attacks and JIT liquidity within 3 seconds
- **Pool health scoring**: Composite metric (trading + liquidity + safety)
- **Cross-chain analytics**: Compare MEV rates, gas costs, liquidity depth across L1/L2
- **Market analytics**: TWAP, OHLC, volume from on-chain events

Monitors production Uniswap V2 pools across three chains simultaneously.

## Tech Stack

| Component | Language | Purpose |
|---|---|---|
| `ingester/` | Go | Blockchain listener + Chainlink pricing |
| `aggregator/` | Java 21 / Flink 2 | Stream processing (4 window pipelines) |
| `analytics-service/` | Kotlin / Ktor | REST API + pool health scoring |
| `schemas/avro/` | Avro | Event schema definitions |
| `dapr/` | YAML | Pub/sub component configs |

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — System design, data flow, design decisions
- [DATA_MODEL.md](DATA_MODEL.md) — Schema definitions, field semantics, evolution
- Component READMEs — Build and run instructions

## Quick Start

No API key needed — the default stack runs Kafka, Flink, and the analytics API:

```bash
docker compose up -d --build
python3 scripts/produce_test_events.py   # feed test events (requires fastavro, kafka-python)
```

Visit http://localhost:8080/dashboard for live analytics (Flink Web UI: http://localhost:8081).

For live chain data, set `POLYGON_RPC_URL` (Polygon WebSocket endpoint) in `.env` and add the `live` profile:

```bash
cp .env.example .env   # set POLYGON_RPC_URL
docker compose --profile live up -d --build
```

**View logs**: `docker compose logs -f flink-jobmanager analytics-service` (add `ingester` with the live profile)

## Build Individual Components

See component READMEs for standalone build instructions:
- `ingester/README.md`
- `aggregator/README.md`
- `analytics-service/README.md`
