# DEX Stream Analytics

Real-time analytics for DeFi liquidity pools — detect MEV attacks, score pool health, and monitor market trends with sub-3-minute latency.

## Problem

Traders lose 2-5% to sandwich attacks with no real-time warning. LPs can't identify toxic pools before committing capital. Existing tools (Dune, DEXTools) have 5-15 min lag because they aggregate after the fact; this pipeline computes the same metrics within seconds of the block, so the warning arrives while it still matters.

## Solution

Apache Flink pipeline processing live DEX events from both EVM and Solana chains:
- **MEV detection**: Sandwich attacks and JIT liquidity within 3 seconds
- **Pool health scoring**: Composite metric (trading + liquidity + safety)
- **Market analytics**: TWAP, OHLC, volume from on-chain events
- **Multi-chain support**: EVM (Uniswap V2) and Solana (Raydium AMM v4) ingesters publishing to shared Kafka topics

The pipeline, topics, and aggregations are chain-neutral; adding a chain means adding an ingester (or an ingester config
entry), not
changing the aggregator or analytics service.

**Chain families**: EVM (log-based) and Solana (account-diff-based) are implemented. Cosmos SDK (ABCI
event-attribute-based,
Osmosis first) is the planned third family —
see [EVM-TO-RUST-MIGRATION.md](EVM-TO-RUST-MIGRATION.md#chain-expansion-strategy).

## Tech Stack

| Component            | Language          | Purpose                                |
|----------------------|-------------------|----------------------------------------|
| `evm-ingester/`      | Go                | EVM listener + Chainlink pricing       |
| `solana-ingester/`   | Rust              | Solana listener + Pyth pricing         |
| `aggregator/`        | Java 21 / Flink 2 | Stream processing (4 window pipelines) |
| `analytics-service/` | Kotlin / Ktor     | REST API + pool health scoring         |
| `dapr/`              | YAML              | Pub/sub component configs              |

Avro schemas are embedded per component: `evm-ingester/internal/avro/`, `solana-ingester/src/avro/`, and
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

For live blockchain events instead of test data, configure RPC endpoints in `.env` and use profiles:

**EVM chains** (Uniswap V2 on Polygon):
```bash
cp .env.example .env                          # add ETH_RPC_URL
docker compose --profile live up -d --build   # starts evm-ingester + dapr sidecar
docker compose logs -f evm-ingester           # monitor blockchain listener
```

**Solana** (Raydium AMM v4 SOL/USDC):

```bash
# Add SOLANA_RPC_URL to .env
docker compose --profile solana up -d --build # starts solana-ingester + dapr sidecar
docker compose logs -f solana-ingester        # monitor blockchain listener
```

**Both chains**:

```bash
docker compose --profile live --profile solana up -d --build
```

Ingesters are not required for local development — the pipeline works with test events by default.

## Build Individual Components

See component READMEs for standalone build instructions:

- `evm-ingester/README.md`
- `solana-ingester/README.md`
- `aggregator/README.md`
- `analytics-service/README.md`

## Roadmap

**✅ Multi-chain support** — Solana ingester now implemented. The chain-agnostic design
(see [ARCHITECTURE.md](ARCHITECTURE.md#design-decisions)) is validated: `solana-ingester` emits identical Avro schemas
to shared Kafka topics; aggregator and analytics-service process EVM and Solana events without modification.

**Next**:

1. **Natural-language query layer** — a read-only compiler that turns a prose question into a
   typed query plan over the analytics API. The model selects a query; the engine returns every
   number.
2. **Cosmos SDK chain family** — `cosmos-ingester` for Osmosis and Injective (Tendermint RPC, ABCI
   event attributes, `cosmos:osmosis-1` CAIP-2 identity). Planned after the EVM port to Rust and
   the shared-crate extraction.
3. **Additional EVM L2s** — config entries in `evm-ingester`, no new code
4. **Enhanced MEV detection** — Cross-chain arbitrage patterns, multi-pool sandwich detection
