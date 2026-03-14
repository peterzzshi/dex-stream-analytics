# Web3 DEX Analytics - Project Context

> AI-oriented quick context. Keep this concise and defer detailed design to `ARCHITECTURE.md`.

## Quick Facts

- Project: Real-time DEX analytics pipeline on Polygon QuickSwap
- Purpose: demonstrate event-driven Web3 data engineering with Go, Flink, DAPR, and Kotlin
- Status:
  - `ingester/` active
  - `aggregator/` in progress
  - `api/` Kotlin sink/API skeleton in progress

## Source of Truth by Topic

- Runbook and setup: `README.md`
- Architecture and tradeoffs: `ARCHITECTURE.md`
- Data semantics and transformations: `DATA_MODEL.md`
- Service implementation details:
  - `ingester/README.md`
  - `aggregator/README.md`
  - `api/README.md`
  - `schemas/README.md`

## Architecture Snapshot

```text
Polygon logs (Swap/Mint/Burn)
  -> ingester (Go + DAPR)
  -> Kafka topics:
       - dex-trading-events (Swap)
       - dex-liquidity-events (Mint/Burn)
  -> aggregator (Flink + Java, native Kafka connectors)
  -> output topics:
       - dex-trading-analytics
       - dex-liquidity-analytics (in progress)
       - dex-pattern-analytics (planned)
  -> sink/api (Kotlin + DAPR)
```

## Locked Decisions

- Keep Avro as canonical payload contract (`schemas/avro/*.avsc`).
- Use DAPR at service edges (ingester and sink/API), not inside Flink runtime.
- Finality strategy: N-confirmation gate before publish (planned implementation).
- Heterogeneous liquidity decode target: CloudEvent `type` -> exact schema before Avro decode.
- Event coverage now: `Swap`, `Mint`, `Burn`; next high-value addition: `Transfer`.

## Current Gaps (Concise)

- Reorg/finality handling not implemented yet in ingest path.
- Aggregator contract hardening still in progress (schema routing + durability).
- Checkpointing/idempotency not fully implemented for production-like restart behavior.
- Multi-pair discovery and LP token accounting remain planned phases.

## Priority Order for Implementation

1. Add finality/reorg protection in ingester.
2. Add strict CloudEvent-type schema routing in aggregator decode path.
3. Enable durability/idempotency foundations (checkpointing + dedup strategy).
4. Expand event coverage with `Transfer`, then move deeper into aggregator analytics.

## Notes for AI Sessions

- Do not duplicate long design explanations here; link to canonical docs.
- Prefer updating service READMEs for service-specific behavior changes.
- Keep this file to "state + decisions + priorities" only.
# Web3 DEX Analytics — AI Tool Context

> Concise technical reference for AI development tools. For full design see `ARCHITECTURE.md`.

## Quick Facts

**Project:** Real-time DEX analytics pipeline on Polygon blockchain  
**Purpose:** Showcase project demonstrating Go, Flink, Web3, DAPR integration, and a planned Kotlin sink/API  
**Status:** Ingester complete, Aggregator in progress, Kotlin sink/API in progress

## System Architecture

```
Polygon QuickSwap (Swap/Mint/Burn Events)
        │
        ▼ WebSocket subscription
event-ingester (Go + DAPR)        ✅ COMPLETE
        ├→ SwapEvent → Kafka "dex-trading-events" (high freq ~100/min)
        └→ Mint/BurnEvent → Kafka "dex-liquidity-events" (low freq ~10/min, heterogeneous)
        │
        ▼ Native Kafka connector
stream-aggregator (Flink + Java 21)    🚧 IN PROGRESS
        ├→ Trading: 5-min windows → "dex-trading-analytics"
        ├→ Liquidity: 1-hour windows → "dex-liquidity-analytics" [in progress]
        └→ Patterns: Session windows → "dex-pattern-analytics" [planned]
        │
        ▼ DAPR subscription
analytics-sink-api (Kotlin + DAPR)    🚧 IN PROGRESS
```

## Key Technical Decisions

| Aspect           | Choice                              | Location               |
|------------------|-------------------------------------|------------------------|
| Topic strategy   | Dual topics by frequency            | `ingester/` complete   |
| Events captured  | Swap, Mint, Burn (no Sync)          | `ingester/` complete   |
| Window sizes     | 5-min (trading), 1-hour (liquidity) | `aggregator/` partial  |
| Flink → Kafka    | Native connector (not DAPR)         | `aggregator/` existing |
| Ingester → Kafka | DAPR HTTP API                       | `ingester/` complete   |
| Finality strategy| N-confirmation before publish       | `ingester/` planned    |
| Schema routing   | CloudEvent type -> exact Avro schema| `aggregator/` planned  |

**Event-to-Analytics Mapping (CRITICAL):**
- **Swap → dex-trading-analytics** (5-min windows: TWAP, OHLC, volume)
- **Mint/Burn → dex-liquidity-analytics** (1-hour windows: LP flows, TVL) [in progress]
- **All events → dex-pattern-analytics** (session windows: MEV detection) [future]

**Recent Changes:**
- ✅ Removed DexEvent envelope → separate event schemas
- ✅ Removed SyncEvent → redundant with Swap price data
- ✅ Refactored ingester for dual-topic routing
- ✅ Unified codec architecture (NewCodec with event type registry)
- 🚧 Aggregator dual-source pipeline in validation
- ✅ Created 3 new deserializers (Swap, Mint, Burn)
- ✅ Updated FlinkConfig for dual topics
- ✅ StreamProcessor consumes both trading and liquidity topics

## Current Implementation State
### Ingester (Go) — `ingester/`

**Status:** ✅ Complete, 0 errors

**Files:**
- `cmd/ingester/main.go` — Entry point, event loop with type switch
- `internal/blockchain/listener.go` — WebSocket subscription, returns `chan any` with polymorphic events
- `internal/avro/codec.go` — Unified codec with event type registry (NewCodec)
- `internal/publisher/dapr.go` — PublishSwap/Mint/Burn methods with topic routing
- `internal/config/config.go` — TopicTradingEvents, TopicLiquidityEvents fields

**Key Code Patterns:**
```go
// Unified codec with registry
func NewCodec(eventType string) (*goavro.Codec, error) {
    schemaText, ok := schemaRegistry[eventType]
    if !ok {
        return nil, fmt.Errorf("unknown event type: %s", eventType)
    }
    return goavro.NewCodec(schemaText)
}

// Event publishing with type switch
func publishEvent(event any) {
    switch e := event.(type) {
    case events.SwapEvent:
        publisher.PublishSwap(ctx, e)
    case events.MintEvent:
        publisher.PublishMint(ctx, e)
    case events.BurnEvent:
        publisher.PublishBurn(ctx, e)
    }
}
```

### Aggregator (Flink) — `aggregator/`

**Status:** 🚧 In progress (trading stable, liquidity/pattern hardening pending)

**Current State:**
- ✅ SwapEventDeserializer.java for direct SwapEvent consumption (no envelope)
- ✅ MintEventDeserializer.java for direct MintEvent consumption
- ✅ BurnEventDeserializer.java for direct BurnEvent consumption
- ✅ StreamProcessor.java updated for dual-source pattern
- ✅ FlinkConfig.java supports TOPIC_TRADING_EVENTS, TOPIC_LIQUIDITY_EVENTS, TOPIC_TRADING_ANALYTICS
- ✅ SwapAggregator.java produces 5-min trading analytics
- 🚧 LiquidityWindowFunction.java processes Mint/Burn in 1-hour windows; output contract and production hardening pending

**Architecture:**
- Consumes from `dex-trading-events` (SwapEvent)
- Publishes to `dex-trading-analytics` (AggregatedAnalytics)
- In progress: Consume from `dex-liquidity-events` (Mint/Burn) → publish to `dex-liquidity-analytics`

## Schemas (Avro)

Location: `schemas/avro/` — embedded at build time

**Active Schemas:**
- `SwapEvent.avsc` → `dex-trading-events` topic
- `MintEvent.avsc` → `dex-liquidity-events` topic
- `BurnEvent.avsc` → `dex-liquidity-events` topic
- `AggregatedAnalytics.avsc` → `dex-trading-analytics` topic

**Removed:**
- ~~`DexEvent.avsc`~~ (envelope removed)
- ~~`SyncEvent.avsc`~~ (redundant with Swap price)

**Field Notes:**
- Financial values: `string` type (avoid floating-point precision loss)
- Timestamps: `long` milliseconds since epoch
- LP token amounts: Placeholder "0" (requires Transfer event correlation)

See `DATA_MODEL.md` for full field semantics.

## Kafka Topics & Partitioning

| Topic | Producers | Consumers | Partitions | Key |
|-------|-----------|-----------|------------|-----|
| `dex-trading-events` | ingester | aggregator | 6 | pairAddress |
| `dex-liquidity-events` | ingester | aggregator | 6 | pairAddress |
| `dex-trading-analytics` | aggregator | sink-api (planned, Kotlin) | 3 | windowId |

**Why 6 partitions?** Polygon ~30 blocks/min × 3 events/block = ~90 events/min. 6 partitions = ~15 events/partition/min (comfortable margin).

## Key Files by Feature
| Feature | Files |
|---------|-------|
| Blockchain listening | `ingester/internal/blockchain/listener.go` |
| Avro encoding | `ingester/internal/avro/codec.go` |
| Topic routing | `ingester/internal/publisher/dapr.go` |
| Event aggregation | `aggregator/src/.../functions/SwapAggregator.java` |
| Schema definitions | `schemas/avro/*.avsc` |

## Functional Programming Patterns (Technical Showcase)

**Purpose:** Demonstrate advanced functional programming beyond typical enterprise Java.

**Patterns Implemented:**
1. **Sealed Interface** (`DexEvent.java`) — Type-safe event hierarchy with exhaustive pattern matching
2. **TriState Type** (`TriState.java`) — Explicit defined/null/undefined semantics for optional values

**Use Cases:**
- **DexEvent**: Generic functions (watermarks, logging) work across all event types
- **Pattern Matching**: Exhaustive type-safe event handling

**Files:**
- Core: `models/DexEvent.java`, `types/TriState.java`
- Examples: `src/test/java/com/web3analytics/types/TriStateExamples.java`

**Why this matters:**
- Shows understanding of algebraic data types (sum types, product types)
- Uses modern Java features (sealed interfaces, pattern matching, records)


## Environment Variables

### Ingester
```bash
POLYGON_RPC_URL=wss://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY
PAIR_ADDRESS=0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827
TOPIC_TRADING_EVENTS=dex-trading-events  # default
TOPIC_LIQUIDITY_EVENTS=dex-liquidity-events  # default
DAPR_HTTP_PORT=3500
```

### Aggregator (needs update for dual topics)
```bash
KAFKA_BOOTSTRAP_SERVERS=localhost:9092
TOPIC_TRADING_EVENTS=dex-trading-events
TOPIC_LIQUIDITY_EVENTS=dex-liquidity-events
TOPIC_TRADING_ANALYTICS=dex-trading-analytics
TOPIC_LIQUIDITY_ANALYTICS=dex-liquidity-analytics
```

## Common Development Tasks

**Start infrastructure:**
```bash
docker-compose up -d kafka dapr-placement
```

**Run ingester (Docker):**
```bash
docker-compose up -d ingester
docker logs -f ingester
```

**Run aggregator (IntelliJ):**
- File → Run Configurations → Environment variables (set above)
- Run StreamProcessor.java with debugger

**Check Kafka topics:**
```bash
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list
```

**Consume trading events:**
```bash
docker exec -it kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic dex-trading-events \
  --from-beginning
```

## Known Limitations & TODOs

1. **Finality/Reorg Protection Not Implemented Yet** (Phase 1)
   - Impact: Reorged logs can affect analytics correctness
   - Plan: Add configurable N-confirmation gate before publishing downstream

2. **Strict CloudEvent-Type Schema Routing Not Implemented in Aggregator Yet** (Phase 1)
   - Impact: Heterogeneous liquidity decode is less robust to schema evolution
   - Plan: Resolve exact schema from CloudEvent `type` before deserialization

3. **Durability/Idempotency Hardening Pending** (Phase 1)
   - Impact: Restart/replay risk can duplicate or skew aggregates
   - Plan: Enable Flink checkpointing and dedup strategy using event/window identity

4. **LP Token Amounts Placeholder** (Phase 2)
   - Impact: Can't calculate LP profitability, impermanent loss
   - See ARCHITECTURE.md "Design Deep-Dive: LP Token Implementation"

5. **Liquidity Analytics Stream In Progress**
   - Current: Trading analytics stable; liquidity analytics path exists but needs contract/production hardening
   - Needed: Finalize Avro output compatibility and end-to-end validation for Mint/Burn → 1-hour LP analytics

6. **Single Pair Only**
   - Current: Hardcoded WMATIC/USDC
   - Enhancement: Multi-pair discovery via PairCreated events

## Business Value and Skill Demonstration Lens

**Business value delivered now (prototype):**
- Real-time market-quality signals (TWAP/OHLC/volume) for DEX activity monitoring.
- Reusable architecture for Web3 event pipelines with oracle enrichment.

**Business value to unlock next:**
- Higher-confidence analytics under chain reorgs and service restarts.
- Better LP and MEV insights via broader event correlation.

**Technical skills demonstrated:**
- Event-driven architecture with Kafka/Flink/DAPR boundaries.
- Avro schema-first contracts and typed stream processing.
- On-chain data enrichment with caching/oracle integration.

**Technical skills to demonstrate next:**
- Reorg/finality consistency engineering.
- Contract-safe schema routing using CloudEvent metadata.
- Stream fault-tolerance and idempotency design in distributed systems.

## Full Documentation

- **ARCHITECTURE.md** — System design, infrastructure, patterns, real-world considerations
- **DATA_MODEL.md** — Schema definitions, field semantics, transformations, validation
- **README.md** — Developer setup, operations, troubleshooting