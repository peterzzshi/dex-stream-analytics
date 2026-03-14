# dex-stream-analytics - AI Session Prompt

Use this file as a lightweight orientation for AI sessions. Keep details concise and delegate depth to canonical docs.

## Session Intent

- Build and refine a real-time DEX analytics pipeline.
- Favor practical delivery over speculative architecture changes.
- Keep docs consistent: root runbook in `README.md`, technical depth elsewhere.

## System Summary

- Ingest: `ingester/` (Go) reads Polygon Swap/Mint/Burn and publishes via DAPR to Kafka.
- Process: `aggregator/` (Flink + Java) performs event-time windows and produces analytics topics.
- Sink/API: `api/` (Kotlin) consumes analytics and serves query endpoints.
- Contracts: Avro schemas in `schemas/avro/`.

## Non-Negotiables

- Avro remains the canonical data contract.
- DAPR remains part of the demonstrated architecture at service edges.
- Aggregator stays native Kafka/Flink (no DAPR sidecar processing path).
- Treat aggregator as in-progress; prioritize correctness and contract clarity over feature sprawl.

## Current Priorities

1. Implement finality-first ingestion (N-confirmation policy).
2. Implement CloudEvent `type` -> exact schema selection before deserialization in aggregator.
3. Implement durability/idempotency primitives (checkpointing + dedup strategy).
4. Add next event type (`Transfer`) before advanced pattern analytics.

## Expected Documentation Boundaries

- `README.md`: what it does + how to run.
- `ARCHITECTURE.md`: design decisions, tradeoffs, roadmap.
- `DATA_MODEL.md`: field semantics and transformations.
- Service READMEs: service-specific behavior and configuration.
- `.ai-context/*`: only AI guidance, concise status, and priorities.

## Pointers

- Root runbook: `README.md`
- Architecture: `ARCHITECTURE.md`
- Data model: `DATA_MODEL.md`
- Service details: `ingester/README.md`, `aggregator/README.md`, `api/README.md`, `schemas/README.md`
# dex-stream-analytics — AI Session Prompt

> Quick reference for AI development sessions. For full architecture see `ARCHITECTURE.md`.

## What

Real-time DEX analytics pipeline processing Polygon QuickSwap events through windowed stream aggregation, outputting TWAP, volume, volatility, and pattern detection.

## Architecture

```
Polygon QuickSwap ──WebSocket──▶ event-ingester (Go + DAPR)
    ├─ SwapEvent ──▶ Kafka "dex-trading-events"
    └─ Mint/BurnEvent ──▶ Kafka "dex-liquidity-events" (heterogeneous)
         │
         ▼ Native Kafka connector (NOT DAPR)
    stream-aggregator (Flink + Java 21) [in progress]
    ├─ Trading stream: Swap → 5-min tumbling → Trading analytics
    ├─ Liquidity stream: Mint/Burn → 1-hour tumbling → LP analytics [in progress]
    └─ Pattern stream: Session windows → MEV detection [planned]
         │
         ▼ Native Kafka connector
    Kafka output topics (trading/liquidity/pattern analytics)
         │
         ▼ DAPR pub/sub subscription
    analytics-sink-api (Kotlin + DAPR) [in progress]
```

**Key design:** 
- Separate topics by frequency: Trading (high ~100/min) vs. Liquidity (low ~10/min)
- Flink uses native connector (not DAPR) for exactly-once + checkpointing
- DAPR is a core demonstration for service decoupling at pipeline edges (ingester + sink/API)

## Components

| Service           | Tech                  | Folder        | Status     |
|-------------------|-----------------------|---------------|------------|
| event-ingester    | Go, go-ethereum, DAPR | `ingester/`   | ✅ Active   |
| stream-aggregator | Flink, Java 21        | `aggregator/` | 🚧 In progress |
| analytics-sink-api| Kotlin, DAPR          | `api/` | 🚧 In progress |

## Tech Stack

- **Languages:** Go 1.21+, Java 21, Kotlin (planned sink/API)
- **Streaming:** Flink, Kafka (KRaft mode)
- **Schema:** Avro (embedded, no registry)
- **Service Mesh:** DAPR (ingester + sink/API edges; aggregator uses native Kafka)
- **Blockchain:** go-ethereum, Polygon mainnet
- **DevOps:** Docker Compose

## Key Contracts (Polygon)

- WMATIC/USDC Pair: `0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827`
- Factory: `0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32`
- Router: `0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff`

## Schemas

Defined in `schemas/avro/` (see `DATA_MODEL.md` for full specs):

- `SwapEvent.avsc` — Trading events → `dex-trading-events` topic
- `MintEvent.avsc` — LP provision → `dex-liquidity-events` topic
- `BurnEvent.avsc` — LP removal → `dex-liquidity-events` topic
- `AggregatedAnalytics.avsc` — Output from stream-aggregator

**Events removed:** `SyncEvent` (redundant with Swap price data)

## Kafka Topics

| Topic | Purpose | Schema(s) | Frequency |
|-------|---------|-----------|-----------|
| `dex-trading-events` | Swap trading activity | SwapEvent | High (~100/min) |
| `dex-liquidity-events` | LP operations (heterogeneous) | MintEvent, BurnEvent | Low (~10/min) |
| `dex-trading-analytics` | 5-min trading windows | AggregatedAnalytics | Per window |
| `dex-liquidity-analytics` | 1-hour LP windows [planned] | LiquidityAnalytics | Per window |
| `dex-pattern-analytics` | MEV detection [planned] | PatternAnalytics | Per session |

## Current Implementation Status

### ✅ Completed (Ingester)
- Blockchain WebSocket listener
- Event parsing (Swap, Mint, Burn)
- Unified Avro codec with event type registry
- Dual-topic routing via DAPR
- SyncEvent removed from pipeline
- Configuration for dual topics

### 🚧 In Progress (Aggregator)
- SwapAggregator for 5-min trading windows
- Multi-source Flink architecture (dual topics)
- Event-specific deserializers (Swap, Mint, Burn)
- StreamProcessor consuming from dex-trading-events and dex-liquidity-events

### 📋 Planned
- Pattern detection with session windows
- Kotlin sink/API with DAPR subscription + REST endpoints
- LP token amount calculation (requires Transfer event correlation)

## Key Constraints

- DAPR sidecars for ingester + sink/API services only, NOT for aggregator
- Event-time semantics using block timestamps, 60s watermark
- Avro schemas are single source of truth (`.avsc` files)
- Dual-topic architecture (no envelope wrapper)
- LP token amounts currently placeholders (Transfer correlation pending)

## Real-World Gap Closure Priorities

1. **Finality-first ingestion**
   - Apply N-confirmation gate before publishing analytics inputs.
   - Value: materially improves trust in downstream metrics.

2. **CloudEvent-type-driven schema resolution**
   - Use CloudEvent event type metadata to select exact Avro schema before deserialization.
   - Value: safer schema evolution for heterogeneous topics.

3. **Durability and idempotency**
   - Enable checkpointing + durable backend and add dedup semantics via event/window identity.
   - Value: stable analytics under restarts and replay scenarios.

4. **Operational observability**
   - Add lag/checkpoint/decode/reconnect metrics and alerting.
   - Value: faster incident detection and operational confidence.

5. **Coverage expansion**
   - Add PairCreated multi-pool discovery and Transfer-based LP token accounting.
   - Value: broader market coverage and richer LP analytics.

## Documentation Structure

- `README.md` — Developer setup and operations guide
- `ARCHITECTURE.md` — System design, decisions, infrastructure
- `DATA_MODEL.md` — Schema definitions, field semantics, transformations
- `fp-patterns-skill.md` — FP patterns showcase notes
- `.ai-context/project-context.md` — AI tool context (NOT Git tracked)
- `.ai-context/dex-stream-analytics-prompt.md` — This file (NOT Git tracked)
