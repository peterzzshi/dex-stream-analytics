# Web3 DEX Analytics — AI Tool Context

> Concise technical reference for AI development tools. For full design see `ARCHITECTURE.md`.

## Quick Facts

**Project:** Real-time DEX analytics pipeline on Polygon blockchain  
**Purpose:** Showcase project demonstrating Go, Flink, Web3, stream processing patterns  
**Status:** Ingester complete, Aggregator partial (needs Flink refactor for dual topics)

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
stream-aggregator (Flink + Java 21)    🚧 NEEDS REFACTOR
        ├→ Trading: 5-min windows → "dex-trading-analytics"
        ├→ Liquidity: 1-hour windows → "dex-liquidity-analytics" [planned]
        └→ Patterns: Session windows → "dex-pattern-analytics" [planned]
        │
        ▼ DAPR subscription
analytics-api (Go + DAPR)         📋 PLANNED
```

## Key Technical Decisions

| Aspect           | Choice                              | Location               |
|------------------|-------------------------------------|------------------------|
| Topic strategy   | Dual topics by frequency            | `ingester/` complete   |
| Events captured  | Swap, Mint, Burn (no Sync)          | `ingester/` complete   |
| Window sizes     | 5-min (trading), 1-hour (liquidity) | `aggregator/` partial  |
| Flink → Kafka    | Native connector (not DAPR)         | `aggregator/` existing |
| Ingester → Kafka | DAPR HTTP API                       | `ingester/` complete   |

**Event-to-Analytics Mapping (CRITICAL):**
- **Swap → dex-trading-analytics** (5-min windows: TWAP, OHLC, volume)
- **Mint/Burn → dex-liquidity-analytics** (1-hour windows: LP flows, TVL) [planned]
- **All events → dex-pattern-analytics** (session windows: MEV detection) [future]

**Recent Changes:**
- ✅ Removed DexEvent envelope → separate event schemas
- ✅ Removed SyncEvent → redundant with Swap price data
- ✅ Refactored ingester for dual-topic routing
- ✅ Unified codec architecture (NewCodec with event type registry)
- ✅ Aggregator dual-source refactor complete
- ✅ Created 3 new deserializers (Swap, Mint, Burn)
- ✅ Updated FlinkConfig for dual topics
- ✅ StreamProcessor now consumes from dex-trading-events

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

**Status:** ✅ Complete dual-topic refactor, ready for testing

**Current State:**
- ✅ SwapEventDeserializer.java for direct SwapEvent consumption (no envelope)
- ✅ MintEventDeserializer.java for direct MintEvent consumption
- ✅ BurnEventDeserializer.java for direct BurnEvent consumption
- ✅ StreamProcessor.java updated for dual-source pattern
- ✅ FlinkConfig.java supports TOPIC_TRADING_EVENTS, TOPIC_LIQUIDITY_EVENTS, TOPIC_TRADING_ANALYTICS
- ✅ SwapAggregator.java produces 5-min trading analytics
- 🚧 Liquidity aggregator (Mint/Burn → 1-hour windows) commented out for future implementation

**Removed Files:**
- ❌ DexEvent.java (envelope removed)
- ❌ SyncEvent.java (redundant event)
- ❌ AvroDeserializationSchema.java (replaced by event-specific deserializers)

**Architecture:**
- Consumes from `dex-trading-events` (SwapEvent)
- Publishes to `dex-trading-analytics` (AggregatedAnalytics)
- Future: Consume from `dex-liquidity-events` (Mint/Burn) → publish to `dex-liquidity-analytics`

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
| `dex-trading-analytics` | aggregator | api (future) | 3 | windowId |

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
2. **Generic Event Processing** (`EventUtils.java`) — Higher-order functions with sealed types

**Use Cases:**
- **DexEvent**: Generic functions (watermarks, logging) work across all event types
- **Pattern Matching**: Exhaustive type-safe event handling

**Files:**
- Core: `models/DexEvent.java`, `functions/EventUtils.java`
- Documentation: `docs/functional-programming-patterns.md`

**Why this matters:**
- Shows understanding of algebraic data types (sum types, product types)
- Uses modern Java features (sealed interfaces, pattern matching, records)

See `docs/functional-programming-patterns.md` for detailed explanation.

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

1. **LP Token Amounts Placeholder** (Phase 2)
   - Impact: Can't calculate LP profitability, impermanent loss
   - See ARCHITECTURE.md "Design Deep-Dive: LP Token Implementation"

2. **Liquidity Analytics Stream Not Implemented**
   - Current: Only trading analytics (Swap → 5-min windows)
   - Needed: Mint/Burn → 1-hour LP analytics
   - Files to implement: LiquidityAggregator.java, update StreamProcessor
   - Uncomment liquidity stream code in StreamProcessor.java

3. **No Checkpointing**
   - Current: State lost on restart
   - Production: Enable `env.enableCheckpointing(300000)` with S3 backend

4. **Single Pair Only**
   - Current: Hardcoded WMATIC/USDC
   - Enhancement: Multi-pair discovery via PairCreated events

## Full Documentation

- **ARCHITECTURE.md** — System design, infrastructure, patterns, real-world considerations
- **DATA_MODEL.md** — Schema definitions, field semantics, transformations, validation
- **README.md** — Developer setup, operations, troubleshooting