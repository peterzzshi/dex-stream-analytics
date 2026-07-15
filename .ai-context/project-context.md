# Project Context

> AI-oriented quick reference. Detailed design lives in `ARCHITECTURE.md`, field semantics in `DATA_MODEL.md`.

## Quick Facts

| Aspect | Value |
|--------|-------|
| Project | Real-time DEX analytics pipeline on Polygon QuickSwap |
| Purpose | Demonstrate Dapr (producer + consumer), Flink (4 window types), polyglot (Go, Java 21, Kotlin) |
| Status | Ingester complete · Aggregator 55 tests · Analytics Service 29 tests |

## Source of Truth

| Topic | File |
|-------|------|
| Setup & run | `README.md` |
| Design & tradeoffs | `ARCHITECTURE.md` |
| Schemas & semantics | `DATA_MODEL.md` |
| Service specifics | `ingester/README.md`, `aggregator/README.md`, `analytics-service/README.md`, `schemas/README.md` |

## Component Status

| Service | Lang | Key Tech | Tests | State |
|---------|------|----------|-------|-------|
| `ingester/` | Go | go-ethereum, Dapr, Avro | — | Complete |
| `aggregator/` | Java 21 | Flink 2.0, native Kafka connector | 55 | Complete |
| `analytics-service/` | Kotlin | Ktor, Dapr, Redis, WebSockets, coroutines | 29 | Complete |
| `schemas/avro/` | — | Avro `.avsc` files | — | Stable |

## Locked Decisions

- Avro is the canonical data contract (`schemas/avro/*.avsc`).
- Dapr at service edges only (ingester + analytics-service); Flink uses native Kafka connector.
- Dual Kafka topics: `dex-trading-events` (high-freq Swap) and `dex-liquidity-events` (low-freq Mint/Burn/Transfer).
- CloudEvent `type` drives exact Avro schema selection in the aggregator.
- Finality: N-confirmation buffer (default 64 blocks) in ingester.

## Open Work

- Checkpointing/idempotency primitives (Flink checkpoint backend, dedup strategy).
- Multi-pair discovery and LP token accounting (requires Transfer correlation).
- Operational observability (Prometheus metrics, distributed tracing).

## Notes for AI Sessions

- Do not duplicate design detail already in ARCHITECTURE.md or DATA_MODEL.md.
- Prefer updating the relevant service README for service-specific changes.
- Keep this file to state, decisions, and priorities only.
- **README.md** — Developer setup, operations, troubleshooting