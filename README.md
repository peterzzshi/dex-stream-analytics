# DEX Stream Analytics

Real-time analytics for DeFi liquidity pools — detect MEV attacks, score pool health, and monitor market trends with sub-3-minute latency.

## Problem

Traders lose 2-5% to sandwich attacks with no real-time warning. LPs can't identify toxic pools before committing capital. Existing tools (Dune, DEXTools) have 5-15 min lag because they aggregate after the fact; this pipeline computes the same metrics within seconds of the block, so the warning arrives while it still matters.

## Solution

Apache Flink pipeline processing live DEX events from a Uniswap V2 pool:
- **MEV detection**: Sandwich attacks and JIT liquidity within 3 seconds
- **Pool health scoring**: Composite metric (trading + liquidity + safety)
- **Market analytics**: TWAP, OHLC, volume from on-chain events
- **Chain-agnostic design**: pool identity is `(chain, pairAddress)`; the pipeline is built to accept events from other chains, including non-EVM ones (see [Roadmap](#roadmap))

Monitors a single Uniswap V2 pool (WMATIC/USDC on Polygon) today. The pipeline, topics, and
aggregations are chain-neutral; adding a chain means adding an ingester instance, not
changing the aggregator.

## Tech Stack

| Component | Language | Purpose |
|---|---|---|
| `ingester/` | Go | Blockchain listener + Chainlink pricing |
| `aggregator/` | Java 21 / Flink 2 | Stream processing (4 window pipelines) |
| `analytics-service/` | Kotlin / Ktor | REST API + pool health scoring |
| `dapr/` | YAML | Pub/sub component configs |

Avro schemas are embedded per component: `ingester/internal/avro/` and
`aggregator/src/main/resources/avro/`.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — System design, data flow, design decisions
- [DATA_MODEL.md](DATA_MODEL.md) — Schema definitions, field semantics, evolution
- Component READMEs — Build and run instructions

## Quick Start

No API key needed — the default stack runs Kafka, Flink, and the analytics API:

```bash
docker compose up -d --build
./scripts/verify_services.sh              # verify all services are healthy
python3 scripts/produce_test_events.py    # feed test events (requires fastavro, kafka-python)
```

### Access the Application

Once running, open:

- **Analytics Dashboard**: http://localhost:8080/dashboard  
  Real-time pool health, trading metrics, MEV alerts
- **Flink Web UI**: http://localhost:8081  
  Job status, task metrics, checkpoints
- **Health Check**: http://localhost:8080/health  
  Service readiness

**View logs**: `docker compose logs -f flink-jobmanager analytics-service`

### Optional: Live Chain Data

For live blockchain events instead of test data, set `POLYGON_RPC_URL` (Polygon WebSocket endpoint) in `.env` and add the `live` profile:

```bash
cp .env.example .env                          # add your RPC URL
docker compose --profile live up -d --build   # starts ingester + dapr sidecars
docker compose logs -f ingester               # monitor blockchain listener
```

The ingester is not required for local development — the pipeline works with test events by default.

## Build Individual Components

See component READMEs for standalone build instructions:
- `ingester/README.md`
- `aggregator/README.md`
- `analytics-service/README.md`

## Roadmap

The chain-agnostic seam exists in the design (see [ARCHITECTURE.md](ARCHITECTURE.md#design-decisions))
but is not yet exercised. In order:

1. **Solana ingester** — a new `solana-ingester` service decoding Raydium AMM v4 swap
   instructions and token-balance deltas. Requires `chain` to become a CAIP-2 string
   (Solana has no EIP-155-style numeric chain ID) and finality to generalize to a
   commitment level (`processed`/`confirmed`/`finalized`), not an N-confirmation count.
   Kafka, Flink, the Avro envelope, and the analytics service are reused unchanged; only
   the ingester is new.
2. **Natural-language query layer** — a read-only compiler that turns a prose question into a
   typed query plan over the analytics API. The model selects a query; the engine returns every
   number.
