# Interview Narrative

> Personal talking points for showcasing this project.

## Elevator Pitch

Real-time DEX analytics pipeline on Polygon: Go ingester captures Swap/Mint/Burn events, publishes via Dapr to Kafka, Flink aggregates with four window types (tumbling, sliding, session, hopping), and a Kotlin/Ktor analytics service serves REST endpoints, pool health scoring, and a live WebSocket dashboard — all backed by Redis.

Three pillars: **Dapr** as producer and consumer, **Flink** with multiple window operations, and a **polyglot stack** (Go, Java 21, Kotlin).

## Why This Stack

| Choice | Reason |
|--------|--------|
| Flink 2.0 + Java 21 | Existing Flink strength; sealed interfaces, records, pattern matching |
| Go | Build new language credibility with channels, interfaces, error handling |
| Kotlin / Ktor | Coroutines, SharedFlow, type-safe DSL builders, `@JvmInline` value classes |
| Blockchain | Demonstrate decentralised systems and event-driven architecture |
| Dapr | Service-mesh decoupling at pipeline edges (not inside Flink) |

## Design Decision Stories

### Dual-Topic Architecture

Started with a single `DexEvent` envelope topic. Realised Swap events (~100/min) vastly outnumber Mint/Burn (~10/min). Separated into `dex-trading-events` and `dex-liquidity-events` so consumers scale independently and retention policies differ.

**Follow-ups:**
- *Why not separate Mint and Burn?* — Semantically related (both LP operations); heterogeneous schemas on one topic show when to prioritise semantic grouping.
- *How does Flink handle multi-schema?* — CloudEvent `type` header drives exact Avro schema selection before deserialization.

### SyncEvent Removal

Uniswap V2 emits Sync after every Swap with reserve values. Analysis showed TWAP uses `amount1Out / amount0In` from Swap directly — Sync added zero analytical value for windowed metrics. Removing it halved event volume.

### Flink Native vs. Dapr for Kafka

Flink's native connector provides exactly-once (two-phase commit), checkpoint integration, event-time extraction, and backpressure. Dapr would break all of these — at-least-once only, no state, no watermarks. Use Dapr where it helps (ingester, analytics-service) and native connectors where semantics matter.

## Kotlin Highlights

- **Coroutines + SharedFlow** for concurrent WebSocket fan-out without thread pools.
- **Sealed classes** for type-safe event routing with exhaustive `when`.
- **Ktor HTML DSL** — type-safe builders via lambda-with-receiver for the dashboard.
- **`@JvmInline value class PairAddress`** — zero-cost domain type safety.

## Addressing Red Flags

**"No production checkpointing"** — Conscious POC decision. Production: `env.enableCheckpointing(300000)` with S3 backend + RocksDB state. The aggregation logic is the point to demonstrate.

**"Why not The Graph?"** — The Graph is production-ready, but building from scratch demonstrates Go skills and blockchain integration understanding. In a real product, evaluate Graph vs. custom on team expertise, customisation needs, and cost at scale.
