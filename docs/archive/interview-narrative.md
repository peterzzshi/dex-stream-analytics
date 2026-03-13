# Interview Narrative

> Personal talking points for showcasing this project. NOT Git tracked.

## Project Elevator Pitch (30 seconds)

"I built a real-time DEX analytics pipeline processing Polygon blockchain events through Flink stream aggregation. It captures swap and liquidity events, applies 5-minute tumbling windows, and outputs TWAP, OHLC, volume, and pattern detection analytics.

The interesting part: I refactored from a single-topic multi-schema architecture to dual topics separated by event frequency—trading events at ~100/min vs. liquidity at ~10/min—which demonstrates production-level understanding of independent scaling and consumer isolation."

## Project Purpose & Gap-Filling

**Career Context:**
- Previous: Flink application transforming database CDC events into Braze API requests
- Gap: No public Web3 experience, limited Go portfolio
- Solution: This showcase project bridges both gaps while demonstrating Flink expertise

**Why This Tech Stack:**
- **Flink 2.0 + Java 21**: Leverage existing strength, show sealed interfaces/records (FP patterns)
- **Go**: Build new language credibility with production patterns (channels, interfaces, error handling)
- **Blockchain**: Demonstrate understanding of decentralized systems, event-driven architectures
- **DAPR**: Modern service mesh pattern (alternative to direct Kafka coupling)

## Key Design Decisions (Interview Stories)

### 1. Dual-Topic Architecture (Technical Depth)

**Question:** "Walk me through a significant architectural decision you made."

**Answer:**
"Initially, I designed a single Kafka topic with a DexEvent envelope containing all event types—Swap, Sync, Mint, Burn. This is a common pattern for heterogeneous events.

But as I implemented the ingester, I realized Swap events occur at ~100/min (every trade) while Mint/Burn happen at ~10/min (liquidity operations are strategic). This frequency mismatch creates problems:
- Flink processes 10x more Swap events even for LP analytics that don't need them
- Consumers interested only in trading data still pay deserialization cost for LP events
- Retention policies can't differ—trading data is high volume but less historical value

I refactored to two topics separated by frequency:
- `dex-trading-events`: SwapEvent only (tight retention, high throughput)
- `dex-liquidity-events`: Mint + Burn heterogeneous (longer retention, lower load)

This demonstrates the production pattern where you optimize for consumer-specific workloads rather than producer simplicity. The trade-off is Flink must consume from multiple sources, but that's straightforward—and it's actually better for demonstrating multi-source stream combination, which is common in real systems."

**Follow-up Answers:**
- *"Why not separate Mint and Burn into different topics?"*
  → "They're semantically related—both LP operations. Heterogeneous schemas on one topic show I understand when to prioritize semantic grouping over schema uniformity. In production, you see this pattern for related events like order_created/order_updated/order_cancelled."

- *"How does Flink handle the multi-schema liquidity topic?"*
  → "Pattern matching on DAPR-injected schema headers (`X-Schema-Type`). Flink's deserialization layer branches to the appropriate Avro schema, then downstream operators use instanceof checks or type switches. This is similar to how Kafka Connect handles multi-schema topics with Schema Registry."

### 2. SyncEvent Removal (Analytical Thinking)

**Question:** "Tell me about a time you simplified a system."

**Answer:**
"Uniswap V2 emits a Sync event after every Swap with updated reserve values. Initially, I captured both because 'more data is better,' right?

But analyzing the use case—5-minute windowed TWAP calculation—I realized:
- TWAP formula: `Σ(price × volume) / Σ(volume)`
- Price comes from Swap: `amount1Out / amount0In`
- I never actually use Sync reserves in the aggregation

Sync was doubling message volume with zero analytical value for windowed analytics. So I removed it, which:
- Cut event volume by 50%
- Reduced Kafka storage and network costs
- Simplified the data model

This taught me to challenge assumptions like 'capture all events.' Sometimes less data is better data—especially when optimizing for a specific analytics workload."

**Follow-up Answers:**
- *"What if you needed reserve data later?"*
  → "That's a good point. Sync contains reserve1/reserve0 which could be useful for liquidity depth tracking. If that requirement emerged, I'd re-add Sync to the liquidity topic since reserves change slowly. But for MVP trading analytics, it's unnecessary complexity."

### 4. Flink Native vs. DAPR for Kafka (Deep Knowledge)

**Question:** "Why does the aggregator use Flink's native Kafka connector instead of DAPR?"

**Answer:**
"This is a common mistake I see—using DAPR for everything. DAPR is great for decoupling application code from infrastructure, but Flink + Kafka integration is special:

**Flink native connector provides:**
- Exactly-once semantics via two-phase commit with Kafka transactions
- Checkpoint integration—state snapshots for fault tolerance
- Event-time extraction from Kafka record timestamps
- Dynamic partition discovery
- Backpressure propagation

**DAPR would break:**
- Exactly-once (downgrades to at-least-once—duplicates on failure)
- Checkpointing (DAPR has no concept of Flink state)
- Watermarks (can't extract event-time from HTTP request metadata reliably)

DAPR adds HTTP overhead without the benefits. So I use DAPR where it helps (ingester, API) and native connectors where performance/semantics matter (Flink).

This shows I understand service mesh patterns aren't silver bullets—you apply them where appropriate."

## Technical Depth Highlights

### Go Patterns Demonstrated
- Goroutines + channels for concurrency (listener → publisher pipeline)
- Functional options pattern for configuration
- Interface-based dependency injection (Publisher, Codec)
- Type switches for polymorphic event handling
- Structured error handling with wrapped errors

### Flink/Java 21 Patterns
- Records for immutable events (SwapEvent, AggregatedAnalytics)
- Pattern matching with switch expressions (planned for multi-source branching)
- AggregateFunction for efficient windowed computation
- Event-time watermarks for out-of-order events
- Multi-source stream combination (trading + liquidity streams)

### Data Engineering Best Practices
- Idempotent event IDs (`blockNumber-txHash-logIndex`)
- Avro for schema evolution (backward/forward compatible)
- Partition keys for parallelism (`pairAddress`)
- Watermarks for late events (handles blockchain reorgs)
- Separate retention policies per topic

## Red Flags & How to Address Them

### "This seems like a toy project—no production checkpointing"

**Response:**
"You're absolutely right—the current implementation doesn't enable Flink checkpointing, which is critical for production. This was a conscious decision for the POC phase:
- Checkpointing requires persistent storage (S3, HDFS) or filesystem mounts
- For local Docker Compose, filesystem checkpoints don't survive container restarts anyway
- The aggregation logic is the interesting part to demonstrate

Production deployment would:
- Enable 5-10 minute checkpoints to S3/GCS
- Use RocksDB state backend for large windows
- Implement savepoint strategy for zero-downtime upgrades

I can walk through the code changes required—it's adding `env.enableCheckpointing(300000)` and configuring the state backend. But for showcasing stream processing concepts, the current implementation demonstrates the core skills."

### "Why not use The Graph instead of building your own ingester?"

**Response:**
"The Graph is great for production—it's battle-tested, has GraphQL queries, and handles all the blockchain quirks. But this project's purpose is to demonstrate Go skills and blockchain integration understanding.

Using The Graph would be:
- Faster to production
- More reliable
- Less code to maintain

But I wouldn't learn:
- go-ethereum WebSocket APIs
- Blockchain event parsing
- Concurrency patterns in Go
- Avro encoding implementation

For a showcase project where the goal is skill demonstration, building from scratch is the right choice. In a real product, I'd evaluate The Graph vs. custom based on:
- Team expertise (do we have blockchain engineers?)
- Customization needs (any non-standard events?)
- Cost (The Graph charges per query, custom ingester is fixed infrastructure cost)

Usually The Graph wins for early-stage products, custom wins at scale when costs become significant."

## Metrics for "Production-Ready" Claims

If asked "How would you make this production-ready?" use this checklist:

### Observability
- [ ] Prometheus metrics (ingester: events/sec, DAPR latency; Flink: checkpoint duration, watermark lag)
- [ ] Structured logging with correlation IDs (trace txHash through ingester → Kafka → Flink)
- [ ] Grafana dashboards (lag, throughput, error rates)
- [ ] Alerting (ingester lag > 100 blocks, Flink checkpoint failures)

### Fault Tolerance
- [ ] Flink checkpointing enabled (5-min intervals to S3)
- [ ] Ingester cursor persistence (Redis stores lastProcessedBlock)
- [ ] Restart strategies (exponential backoff for ingester, fixed delay for Flink)
- [ ] Dead letter queues (malformed events → DLQ topic for manual review)

### Security
- [ ] Kafka SASL/SCRAM authentication + TLS encryption
- [ ] DAPR mTLS between sidecars
- [ ] RPC API keys in secrets manager (AWS Secrets, Vault)
- [ ] API JWT authentication + rate limiting

### Performance
- [ ] Flink parallelism tuned (match partition count, ~6 for trading topic)
- [ ] RocksDB state backend (memory is limited for large windows)
- [ ] Kafka partition count calibrated (6 partitions = 3 Flink slots × 2 TaskManagers)
- [ ] Ingester buffered channels (don't block on slow DAPR calls)

### Testing
- [ ] Unit tests with mocked blockchain clients
- [ ] Integration tests with Testcontainers (Kafka + DAPR)
- [ ] End-to-end smoke tests (MockAMM.sol generates events, verify analytics output)
- [ ] Load testing (simulate 1000 events/sec sustained)

## Business Value Articulation

**For Product Manager Interviews:**

"This pipeline enables real-time DEX analytics with 5-6 minute latency, which is competitive with centralized platforms like Binance (1-minute candles) and significantly faster than on-chain query tools like Dune (1-hour+ latency).

**Revenue opportunities:**
1. **Dashboard SaaS**: $50-200/mo per trader for real-time DEX price feeds
2. **API for DEX aggregators**: Pricing data for 1Inch, Matcha routing algorithms
3. **MEV detection**: Security-focused analytics for protocols ($5-10K enterprise contracts)
4. **Research data**: Universities/hedge funds studying DeFi microstructure

**Market size:** DEX volume is ~$50-100B/month. If we capture 0.1% of traders paying $100/mo, that's $5-10M ARR at scale.

**Key insight:** Most DEX analytics tools query historical data (slow). We process events as they arrive (fast). Speed is the competitive advantage."

## Closing Pitch

"This project demonstrates three things:

1. **Technical breadth**: Go, Java, Flink, Kafka, DAPR, blockchain—full-stack data engineering
2. **Production thinking**: Multi-topic architecture, watermarks, idempotent events, monitoring TODOs
3. **Analytical rigor**: Removed SyncEvent after use-case analysis, chose Trade-offs consciously (LP token placeholders)

I'm ready to contribute to [Company]'s data platform team on day one. Want to see the code?"
