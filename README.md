# DEX Stream Analytics

Real-time analytics for DeFi liquidity pools — measure MEV extraction, score pool health, and track market trends from
finalized on-chain events.

## Problem

Traders lose 2-5% to sandwich attacks, and LPs can't identify toxic pools before committing
capital. Existing tools (Dune, DEXTools) aggregate after the fact with 5-15 min lag. This
pipeline computes per-pool metrics from finalized chain events with bounded, event-time
semantics, so the numbers are reproducible rather than sampled from a moving head.

**On latency, honestly**: this is a post-trade pipeline, not a pre-trade warning system. An
EVM event is published only after 12 confirmations (~2.4 min) and then has to clear a 60 s
watermark and a window boundary, so EVM analytics land several minutes after the block.
Solana finalizes in ~13 s and is correspondingly faster. Anything that warns a trader *before*
their transaction executes would need the mempool, not the chain.

## Solution

Apache Flink pipeline processing live DEX events from both EVM and Solana chains:

- **MEV detection**: Sandwich and JIT-liquidity patterns, computed on finalized events after the
  chain's own finality gate rather than intra-block
- **Pool health scoring**: Composite metric (trading + liquidity + safety)
- **Market analytics**: TWAP, OHLC, volume from on-chain events
- **Multi-chain support**: EVM (Uniswap V2) and Solana (Raydium AMM v4) ingesters publishing to shared Kafka topics

The pipeline, topics, and aggregations are chain-neutral; adding a chain means adding an ingester instance,
not changing the aggregator or analytics service.

**Chain families**: EVM (log-based) and Solana (account-diff-based), both implemented and both
emitting identical Avro schemas to the shared topics.

## Tech Stack

| Component            | Language          | Purpose                                |
|----------------------|-------------------|----------------------------------------|
| `evm-ingester/`      | Rust              | EVM listener + Chainlink pricing       |
| `solana-ingester/`   | Rust              | Solana listener + Pyth pricing         |
| `aggregator/`        | Java 21 / Flink 2 | Stream processing (4 window pipelines) |
| `analytics-service/` | Kotlin / Ktor     | REST API + pool health scoring         |
| `dapr/`              | YAML              | Pub/sub component configs              |

Avro event schemas have a single source of truth in the repo-root `schemas/` directory, embedded
by the shared `common/` crate. The aggregator still codegens from its own copy in
`aggregator/src/main/resources/avro/`.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — System design, data flow, design decisions
- [DATA_MODEL.md](DATA_MODEL.md) — Schema definitions, field semantics, evolution
- Component READMEs — Build and run instructions

## Quick Start

Start all services:

```bash
docker compose up -d --build
```

Without RPC URLs configured, `evm-ingester` and `solana-ingester` will fail validation and restart continuously (this is
expected). The rest of the pipeline (Kafka, Flink, analytics API) runs independently.

### Live Chain Data (Optional)

To ingest real blockchain events:

```bash
cp .env.example .env
# Edit .env: add ETH_RPC_URL and/or SOLANA_RPC_URL
docker compose up -d --build
```

### Access Points

- **Analytics API**: http://localhost:8080/health
- **Flink Web UI**: http://localhost:8081

View logs: `docker compose logs -f flink-jobmanager analytics-service`

### Cleanup

Remove all containers and images:

```bash
docker compose down --rmi all
docker image prune -f
```

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
2. **Additional EVM L2s** — config entries in `evm-ingester`, no new code
3. **Enhanced MEV detection** — Cross-chain arbitrage patterns, multi-pool sandwich detection
