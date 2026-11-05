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

No API key needed to see the pipeline work — Kafka, Flink, and the analytics API run on test
events alone:

```bash
docker compose up -d --build
./scripts/verify_services.sh              # verify all services are healthy
```

`evm-ingester` and `solana-ingester` also start with this command, but without `ETH_RPC_URL` /
`SOLANA_RPC_URL` set they fail config validation and crash-loop (restart: unless-stopped) by
design — this doesn't affect the rest of the stack. Set the RPC URLs below to run them for real.

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

For live blockchain events instead of test data, configure RPC endpoints in `.env` and restart
the ingesters so they pick up the new values:

```bash
cp .env.example .env   # add ETH_RPC_URL and/or SOLANA_RPC_URL
```

**EVM chains** (Uniswap V2 pools on Ethereum mainnet, listed under `[[pools]]` in `evm.toml` —
adding a pool is a config edit and a restart):

```bash
docker compose up -d --build evm-ingester evm-ingester-dapr
docker compose logs -f evm-ingester           # monitor blockchain listener
```

**Solana** (Raydium AMM v4 SOL/USDC):

```bash
docker compose up -d --build solana-ingester solana-ingester-dapr
docker compose logs -f solana-ingester        # monitor blockchain listener
```

**Both chains**:

```bash
docker compose up -d --build evm-ingester evm-ingester-dapr solana-ingester solana-ingester-dapr
```

Ingesters aren't required for local development — the pipeline works with test events by default,
and without RPC URLs set they just crash-loop quietly alongside the rest of the stack.

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
