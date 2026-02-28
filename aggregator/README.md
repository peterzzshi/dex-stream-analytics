# Aggregator

Apache Flink job that aggregates swap events into analytics windows and writes the results back to Kafka.

## Requirements

- Java 21
- Maven
- Flink 2.x
- Kafka endpoint

## Build

```bash
cd aggregator
mvn -DskipTests package
```

## Run

```bash
flink run target/aggregator-1.0-SNAPSHOT.jar
```

## Configuration

- `KAFKA_BOOTSTRAP`: Kafka bootstrap servers (default: `kafka:9092`)
- `TOPIC_TRADING_EVENTS`: Input topic for swap events (default: `dex-trading-events`)
- `TOPIC_LIQUIDITY_EVENTS`: Input topic for mint/burn events (default: `dex-liquidity-events`)
- `TOPIC_TRADING_ANALYTICS`: Output topic for aggregated analytics (default: `dex-trading-analytics`)
- `FLINK_CONSUMER_GROUP`: Kafka consumer group (default: `dex-processor`)
- `FLINK_PARALLELISM`: Job parallelism (default: `2`)
- `FLINK_CHECKPOINT_MS`: Checkpoint interval in milliseconds (default: `10000`)

## Schema Management

Avro schemas are embedded at compile time via Maven plugin. Schema files are located in `../schemas/avro/` and automatically generated as Java classes during build.

## Architecture

### Current Implementation (Phase 1)
Single stream processing Swap events with 5-minute tumbling windows for trading analytics.

### Planned Enhancement (Phase 2)
Multi-stream architecture processing different event types with appropriate windowing strategies:

```
Kafka (dex-trading-events + dex-liquidity-events)
    ↓
    ├─→ [Swap Stream] → 5-min tumbling → Trading Analytics
    │                     (TWAP, OHLC, volume, volatility)
    │
    ├─→ [Mint/Burn Stream] → 1-hour tumbling → Liquidity Analytics  
    │                          (LP behavior, liquidity depth, provision/removal patterns)
    │
    └─→ [Cross-Event Stream] → Session windows → Market Activity Patterns
                                (e.g., large swap followed by liquidity removal)
```

**Benefits:**
- Single Flink application with multiple analytics jobs
- Each stream optimized for its time scale and business logic
- Demonstrates stream branching and heterogeneous event processing
- Production-like architecture in showcase project

## TODO: Production Enhancements

### Watermark Strategy Tuning
- [ ] **Current:** 30s bounded out-of-orderness
- [ ] **Evaluate:** Consider 60s for production given:
  - Polygon chain reorganizations (rare but possible)
  - RPC provider lag/delays
  - WebSocket reconnection windows
- [ ] **Add metrics:** Track late event arrivals and adjust watermark accordingly
- [ ] **Consider:** Per-source watermarks if ingesting from multiple RPC providers

### State Management & Fault Tolerance
- [ ] **Checkpointing:** Currently using in-memory state
  - Add persistent checkpoint storage (S3, HDFS, or local filesystem)
  - Configure checkpoint retention (keep last 3-5 for recovery)
  - Set checkpoint timeout and cleanup policies
- [ ] **Savepoints:** Implement savepoint strategy for:
  - Application upgrades without data loss
  - Schema evolution migrations
  - Cluster maintenance windows
- [ ] **State Backend:** Evaluate RocksDB for large state scenarios
  - Current: Heap-based state (good for demo)
  - Consider: RocksDB for production with larger windows or richer state
- [ ] **Recovery:** Test failure scenarios
  - Ingester restarts
  - Flink job failures mid-window
  - Kafka partition rebalances

### Monitoring & Observability
- [ ] **Metrics:** Export to Prometheus/Grafana
  - Events processed per second
  - Window trigger latency
  - Watermark lag
  - Checkpoint duration and size
- [ ] **Alerting:** Set up alerts for:
  - High watermark lag (> 2 minutes)
  - Checkpoint failures
  - Backpressure detection
- [ ] **Logging:** Structured logging with correlation IDs

### Performance Optimization
- [ ] **Parallelism tuning:** Match Kafka partition count (currently 6)
- [ ] **Operator chaining:** Review and optimize operator fusion
- [ ] **Network buffers:** Tune for throughput vs. latency trade-offs
