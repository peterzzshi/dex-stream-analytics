# Code Review Analysis — Flink Aggregator

Static analysis notes for the aggregator module. See `ARCHITECTURE.md` for design rationale.

## Metrics

| Metric | Value |
|--------|-------|
| Main classes | 25 |
| Test classes | 9 |
| Tests | 55 (all passing) |
| Total lines | ~3 000 |

## Noteworthy Patterns

### Dual-Topic Deserialization

`LiquidityEventDeserializer` handles a heterogeneous topic (Mint + Burn) by trying both Avro schemas with business-logic validation (`isLikelyMint`/`isLikelyBurn`). Errors from both attempts are aggregated via `addSuppressed()`.

### CloudEvent Payload Extraction

`CloudEventPayloadExtractor` supports five encoding variants: `data_base64`, textual Base64, ISO-8859-1 binary escape (a Dapr quirk), raw binary, and nested JSON objects.

### Event-Time OHLC

`SwapAggregator` determines open/close prices using `blockTimestamp` with `logIndex` as tie-breaker — multiple events in the same block get deterministic ordering regardless of arrival order.

### Incremental Aggregation

`AggregateFunction` + `ProcessWindowFunction` composition: O(1) memory per window, full parallelism, window metadata attached at flush time.

### Dead-Letter Side Outputs

`SafeDecodeProcessFunction` routes malformed events to a side output instead of failing the job. Error records include a hex preview of the first 16 bytes for debugging without logging full payloads.

### TriState Sealed Interface

Three-variant algebraic type (`Defined | Undefined | Null`) with `map`/`flatMap`/`fold`. Distinguishes "not yet calculated" from "explicitly null" for optional enrichment fields like `volumeUSD` and `token0Symbol`.

### Validation Layers

1. Avro schema deserialization
2. `AvroFieldReader` runtime type checks with clear error messages
3. Business-logic heuristics (`isSignedInteger`, `isLikelyMint`)
4. OHLC tie-breaking for deterministic financial data

## Open Items

- Prometheus metrics and distributed tracing not wired.
- Checkpoint backend is filesystem (swap to S3/RocksDB for production).
- Integration tests with Testcontainers not yet added.
