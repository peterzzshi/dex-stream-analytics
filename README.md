# Bridge Security Monitor

Real-time exploit detection for cross-chain bridges. Ingests lock/unlock and mint/burn events from
Wormhole, LayerZero and Hop on Ethereum and from Wormhole Portal and Allbridge on Solana, aggregates
them in Flink, and pushes severity-ranked alerts to a WebSocket.

Bridges are the highest-value target in DeFi, and the damage is done in the first minutes of an
exploit — so the interesting question is not "what happened last week" but "is this bridge behaving
outside its normal envelope right now". This pipeline answers that with windows over finalized
events, not sampled block-by-block state.

## What It Detects

Five windowed pipelines, each emitting to its own Kafka topic as an Avro alert:

| Signal | Window | What it fires on |
|---|---|---|
| Transfer volume | 5 min | Lock/unlock totals per `(bridge, chain)` cross size thresholds |
| Supply tracking | 5 min | Net minted/burned; **burn-only with no mints is critical** (drain pattern) |
| Cross-chain flow | 5 min | Inflow/outflow imbalance per bridge across chains |
| Exploit patterns | 60 s session | Rapid transfers (>10 events), large single transfer, lock/mint supply mismatch |
| Reentrancy | 5 s | More than one lock or unlock in a single transaction hash |

## Pipeline

```
Ethereum ─┐                          ┌─ 5 alert topics ─┐
          ├─ Rust ingesters ─ Kafka ─┤                  ├─ Dapr ─ Kotlin service ─ Redis + WebSocket
Solana  ──┘   (via Dapr)   3 topics  └─ Flink 5 jobs ──┘        (subscription)     (recent alerts, live)
```

| Component | Language | Role |
|---|---|---|
| `evm-ingester/` | Rust | `eth_subscribe` logs from Ethereum; 12-block depth finality |
| `solana-ingester/` | Rust | `logsSubscribe` + finalized-slot polling |
| `aggregator/` | Java 21 / Flink | 5 windowed pipelines, Avro sinks |
| `alerting-service/` | Kotlin / Ktor | Dapr consumer, Redis recent-alert cache, WebSocket fan-out |
| `dapr/` | YAML | Kafka pub/sub component and alert subscriptions |

`common/` is the shared Rust crate: config, Avro encoding, CloudEvents envelope, retry, finality
buffer, publisher. Event and alert schemas live once in root `schemas/` and are codegen'd by both the
Maven and Gradle builds.

## Quick Start

```bash
cp .env.example .env          # add ETH_RPC_URL and/or SOLANA_RPC_URL
docker compose up -d --build
```

Without RPC URLs the two ingesters fail validation and restart continuously; that is the intended
fail-fast behavior, and the rest of the stack still runs.

| Endpoint | URL |
|---|---|
| Health (also reports Redis and live WS connections) | `http://localhost:8080/health` |
| WebSocket alerts | `ws://localhost:8080/ws/alerts` |
| Recent alerts for a bridge | `http://localhost:8080/alerts/{bridgeAddress}/{chain}` |
| Flink UI | `http://localhost:8081` |

WebSocket clients can filter server-side with `?bridgeAddress=`, `?chain=`, `?severity=` (minimum)
and `?alertTypes=`.

## Latency, Honestly

This is a post-finality detector, not a pre-execution shield. An EVM event is published only after 12
confirmations (~2.4 min) and then has to clear a 5 s watermark and a window boundary. Solana finality
polling lands around 13 s. Nobody is front-run — the goal is to fire before an exploit has finished
draining, which most of these windows can do, and before a human watching dashboards would notice,
which they reliably do.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — data flow, design decisions, alert semantics
- [DATA_MODEL.md](DATA_MODEL.md) — event and alert schemas
- [GAPS.md](GAPS.md) — known limitations and outstanding defects
- [E2E-TEST.md](E2E-TEST.md) — end-to-end verification walkthrough
- [AGENT.md](AGENT.md) — conventions and commands

## Build Individual Components

- `evm-ingester/README.md`, `solana-ingester/README.md`
- `aggregator/README.md`
- `alerting-service/README.md`
