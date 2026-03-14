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
    stream-aggregator (Flink 2.0.1 + Java 21)
    ├─ Trading stream: Swap → 5-min tumbling → Trading analytics
    ├─ Liquidity stream: Mint/Burn → 1-hour tumbling → LP analytics [planned]
    └─ Pattern stream: Session windows → MEV detection [planned]
         │
         ▼ Native Kafka connector
    Kafka output topics (trading/liquidity/pattern analytics)
         │
         ▼ DAPR pub/sub subscription [future]
    analytics-api (Go + DAPR) [planned]
```

**Key design:** 
- Separate topics by frequency: Trading (high ~100/min) vs. Liquidity (low ~10/min)
- Flink uses native connector (not DAPR) for exactly-once + checkpointing
- DAPR for ingester/API decoupling only

## Components

| Service           | Tech                  | Folder        | Status     |
|-------------------|-----------------------|---------------|------------|
| event-ingester    | Go, go-ethereum, DAPR | `ingester/`   | ✅ Active   |
| stream-aggregator | Flink 2.0.1, Java 21  | `aggregator/` | 🚧 Partial |
| analytics-api     | Go, DAPR              | `api/`        | 📋 Planned |

## Tech Stack

- **Languages:** Go 1.21+, Java 21
- **Streaming:** Flink 2.0.1, Kafka (KRaft mode)
- **Schema:** Avro (embedded, no registry)
- **Service Mesh:** DAPR (ingester + API only)
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

### ✅ Completed (Aggregator)
- SwapAggregator for 5-min trading windows
- Multi-source Flink architecture (dual topics)
- Event-specific deserializers (Swap, Mint, Burn)
- StreamProcessor consuming from dex-trading-events

### 📋 Planned
- LiquidityAggregator for 1-hour LP windows
- Pattern detection with session windows
- Analytics API with REST endpoints
- LP token amount calculation (requires Transfer event correlation)

## Key Constraints

- DAPR sidecars for ingester + API only, NOT for aggregator
- Event-time semantics using block timestamps, 60s watermark
- Schemas are single source of truth (`.avsc` files)
- Dual-topic architecture (no envelope wrapper)
- LP token amounts currently placeholders (Transfer correlation pending)

## Documentation Structure

- `README.md` — Developer setup and operations guide
- `ARCHITECTURE.md` — System design, decisions, infrastructure
- `DATA_MODEL.md` — Schema definitions, field semantics, transformations
- `docs/functional-programming-patterns.md` — FP patterns showcase
- `.ai-context/project-context.md` — AI tool context (NOT Git tracked)
- `.ai-context/dex-stream-analytics-prompt.md` — This file (NOT Git tracked)
