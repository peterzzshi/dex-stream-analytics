# TriState Decision Record

## Decision

Use `TriState<T>` (sealed interface: Defined | Undefined | Null) for optional enrichment fields in the aggregator.

## Which Fields

| Field | Always present? | TriState? | Why |
|-------|----------------|-----------|-----|
| `pairAddress`, `blockNumber`, `amount0/1` | Yes | No | Always in blockchain event |
| `token0Symbol`, `token1Symbol` | No | **Yes** | RPC fetch may fail or be pending |
| `volumeUSD` | No | **Yes** | Requires Chainlink price; may be unavailable |

**Rule:** TriState is for fields that are legitimately absent at ingest time, not for fields that were removed from the model.

## Three States

| Go (ingester) | Java (aggregator) | Meaning |
|---|---|---|
| `nil` pointer | `TriState.undefined()` | Not fetched / not calculated yet |
| `&""` or `&0.0` | `TriState.ofNull()` | Fetch attempted, explicitly empty |
| `&"WMATIC"` or `&123.45` | `TriState.of(value)` | Successfully resolved |

## Why It Matters

Without TriState, `null` is ambiguous — "not calculated yet" vs. "explicitly no value". This corrupts aggregations:

```java
// Wrong: treats "not calculated" as zero
double total = events.stream()
    .map(e -> e.getVolumeUSD() == null ? 0.0 : e.getVolumeUSD())
    .sum();

// Correct: skip events without price data
double total = events.stream()
    .filter(e -> e.getVolumeUSD().isDefined())
    .map(e -> e.getVolumeUSD().get())
    .sum();
```

## Implementation

- `TriState.java` — sealed interface with `map`, `flatMap`, `fold` (monadic operations).
- Go ingester already uses nullable pointers; Avro `["null", "type"]` unions carry the semantics through Kafka.
- Window aggregation merges TriState fields: first defined value wins for symbols, only defined values summed for volume.
4. **Type-safe aggregation** - prevent bugs from treating undefined as zero
5. **FP patterns** - map, flatMap, pattern matching on tri-state

**Current Status:**
- ✅ Go: Using nullable pointers (maps to tri-state)
- ✅ Java: Has TriState infrastructure ready
- ✅ Schemas: Support union types (Avro tri-state)

**Recommendation:**
- ✅ **Keep current Go pattern** (nullable pointers)
- ✅ **Use TriState in Java aggregator** for optional fields
- ✅ **Don't add TriState to removed fields** (they're gone for a reason)

**Result: TriState pattern is MORE relevant now that we removed misleading placeholders!** 🎉

