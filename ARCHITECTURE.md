# Architecture

**Status**: Ingesters and Flink aggregator migrated to bridge security monitoring. Analytics service remains on DEX endpoints (not migrated).

Technical architecture for the Bridge Security Monitor ingestion and aggregation pipeline.

## System Overview

```mermaid
graph TD
    subgraph Blockchain
        ETH[Ethereum Mainnet]
        WORMHOLE[Wormhole Token Bridge]
        LAYERZERO[LayerZero Endpoint]
        HOP[Hop Protocol Bridge]
        SOL[Solana]
        WORMHOLE_SOL[Wormhole Portal]
        ALLBRIDGE[Allbridge]
    end

    subgraph "Ingesters · Rust"
        EVM[evm-ingester<br/>Bridge-only]
        SOLING[solana-ingester<br/>Bridge-only]
        DAPR_E[Dapr sidecar]
        DAPR_S[Dapr sidecar]
    end

    subgraph "Kafka · KRaft"
        KT[bridge-transfer-events<br/>BridgeTransferEvent<br/>direction: lock | unlock]
        KS[bridge-supply-events<br/>BridgeSupplyEvent<br/>operation: mint | burn]
        DLQ[bridge-dlq-events]
    end

    subgraph "Aggregator · Flink 2 / Java 21"
        F1[Transfer Volume<br/>5-min tumbling]
        F2[Supply Tracking<br/>5-min tumbling]
        F3[Cross-Chain Flow<br/>5-min tumbling]
        F4[Exploit Detection<br/>60-sec session]
        F5[Reentrancy Detection<br/>5-sec tumbling]
    end

    subgraph "Analytics Service · Kotlin / Ktor"
        API[REST API<br/>NOT MIGRATED]
        REDIS[(Redis<br/>DEX data only)]
    end

    ETH -- "WebSocket · eth_subscribe" --> EVM
    WORMHOLE -. "TransferSent/Redeemed" .-> EVM
    LAYERZERO -. "PacketSent/Received" .-> EVM
    HOP -. "TransferSentToL2/FromL1Completed" .-> EVM

    SOL -- "WebSocket · logsSubscribe<br/>RPC · finalized slot" --> SOLING
    WORMHOLE_SOL -. "transfer_wrapped/complete_transfer" .-> SOLING
    ALLBRIDGE -. "lock/unlock" .-> SOLING

    EVM --> DAPR_E
    SOLING --> DAPR_S

    DAPR_E -- "BridgeTransferEvent" --> KT
    DAPR_E -- "BridgeSupplyEvent" --> KS
    DAPR_S -- "BridgeTransferEvent<br/>BridgeSupplyEvent" --> KT & KS

    KT -- native connector --> F1
    KT & KS -- native connector --> F2
    KT & KS -- union --> F3
    KT & KS -- union --> F4
    KT -- union --> F5

    F1 -- "POST /api/bridges/transfer-volume" --> API
    F2 -- "POST /api/bridges/supply" --> API
    F3 -- "POST /api/bridges/flow" --> API
    F4 -- "POST /api/bridges/alert" --> API
    F5 -- "POST /api/bridges/reentrancy" --> API

    API -. "NOT IMPLEMENTED" .-> REDIS

    style API fill:#f88,stroke:#d44
    style REDIS fill:#f88,stroke:#d44
```

## Event-to-Analytics Mapping

| Input Topic | Event Types | Pipeline | Window | Purpose |
|---|---|---|---|---|
| `bridge-transfer-events` | `direction`: lock, unlock | Transfer Volume | 5-min tumbling | Lock/unlock counts, total amounts |
| `bridge-supply-events` | `operation`: mint, burn | Supply Tracking | 5-min tumbling | Net minted, total wrapped supply |
| Both topics | Both records | Cross-Chain Flow | 5-min tumbling | Net flow, imbalance ratio |
| Both topics | Both records | Exploit Detection | 60-sec session | Rapid transfers, large amounts, supply mismatch |
| `bridge-transfer-events` | `direction`: lock, unlock | Reentrancy Detection | 5-sec tumbling | Multiple lock/unlock calls in one transaction |

Window lengths are env vars (`*_WINDOW_SECONDS`, `EXPLOIT_SESSION_GAP_SECONDS`) with no defaults; the
values here are the `docker-compose.yml` ones.

Lock/Unlock events track **value transfer** across bridges. Mint/Burn events track **wrapped token supply** changes.

Lock and Unlock were originally separate Avro records and separate Flink streams, unioned at the
aggregator with `Either<BridgeLockEvent, BridgeUnlockEvent>`. Their schemas had identical field
lists — same 21 names, same types, same order — so the split carried no type safety, only a
duplicate struct, a duplicate Kafka consumer, and a downstream union that could not be switched
over exhaustively (`org.apache.flink.types.Either` is not a sealed type, so a `switch` needs a
`default` that would silently absorb any future subtype). The two records were consolidated into
`BridgeTransferEvent` with an Avro enum field `direction`; Mint/Burn became `BridgeSupplyEvent`
with `operation`.

What the aggregation now branches on is a compiler-checked enum rather than `Either.isLeft()`, so
a new direction is a compile error at every switch that does not handle it. The four logical event
types are still distinguished — as field values, not as record names.

## Design Decisions

### Chain-Agnostic Bridge Identity

Bridges are identified by `(chain, bridgeAddress)`. A bridge address alone is ambiguous because the same contract address can exist on different chains.

Two chain families implemented:
- **EVM** (`evm-ingester`): Wormhole, LayerZero, Hop on Ethereum mainnet
- **Solana** (`solana-ingester`): Wormhole Portal, Allbridge

Bridges declared per chain in `evm-bridges.toml` / `solana-bridges.toml`.

| Layer | Implementation |
|-------|----------------|
| Ingester | One binary per chain family; bridges in family-specific TOML |
| Event schema | `chain` (CAIP-2) + `chainName` + `protocol` enum on every event |
| Flink key | `(chain, bridgeAddress)` on all pipelines |

`chain` uses CAIP-2 format (`eip155:1`, `solana:5eykt...`) for chain-neutral identity across EVM and non-EVM chains.

**Why Solana**: Forces testing of non-EIP-155 chain IDs and non-confirmation-count finality (finalized slot polling vs block confirmations). Proves the chain-neutral abstraction works across structurally different families.

### Event Schema Split

Events separated into two topics, one record type each:
- `bridge-transfer-events` (`BridgeTransferEvent`, `direction` = lock | unlock, 7-day retention) —
  value movement
- `bridge-supply-events` (`BridgeSupplyEvent`, `operation` = mint | burn, 30-day retention) —
  wrapped token supply

Split provides independent retention, consumer scaling, and clearer semantic boundaries.

The two records are genuinely different shapes (22 vs 19 fields), so the `BridgeActivity` union
that Cross-Chain Flow and Exploit Detection consume is a real normalization across heterogeneous
topics, not two names for the same payload.

### Protocol Type Safety

`protocol` field is an Avro enum (`wormhole`, `layerzero`, `hop`, `allbridge`), enforced at:
1. **Config load** (Rust ingester): `BridgeProtocol::from_str()` validates protocol names
2. **Avro schema**: Enum with fixed symbols, deserialization fails on unknown values
3. **Flink**: No string parsing, type-safe enum used in aggregation keys

Prevents runtime string-matching bugs and typos.

### Flink Native Kafka (Not Dapr)

Aggregator uses Flink's native Kafka connector for:
- Exactly-once semantics (two-phase commit)
- Checkpoint integration
- Event-time watermarks
- Backpressure

Dapr used where its strengths apply: ingesters (decouple from Kafka) and analytics service (subscription model).

### Finality Guarantees

**EVM**: 12-block confirmation depth before events enter finality buffer
- Depth-based finality with deduplication cache
- Handles reorgs up to 12 blocks deep
- Cache cleanup after finalized block passes

**Solana**: External finality polling
- Separate task queries finalized slot every N seconds
- Finality buffer releases events when block reaches finalized slot
- No confirmation counting, cluster finality is authoritative

Both models use the same `FinalityBuffer` abstraction with different modes (`Depth` vs `External`).

### RestApiSink Pattern

Flink aggregator sinks directly to REST API (no Kafka output topics):
- POST per window close (e.g., 1 POST per bridge per 5-min window)
- 10s connection timeout, 30s request timeout
- Range inputs serialized as JSON strings — a 78-digit `BigInteger` cannot survive a
  `long`/`double` on the receiving side as a bare number

**Failure classification.** Each failure is a typed exception, so the sink `catch`es the one case
it can handle and lets the rest propagate:

| Exception | Statuses | Handling |
|---|---|---|
| `NetworkException` | no response (refused, timed out) | retry |
| `ServerException` | 5xx | retry |
| `RateLimitException` | 429, 408 | retry after a 4× longer backoff |
| `RejectedException` | any other 4xx | log, count in `recordsDroppedFatal`, drop |

`RestApiSink.post()` is the only place a status number is interpreted: 429/408 → `RateLimitException`,
5xx → `ServerException`, any other non-2xx → `RejectedException`.

A code that cannot serialize fails the job rather than counting as a drop — a broken serializer is
a defect in this pipeline, not bad input. Only a 4xx rejection increments `recordsDroppedFatal`.

There is no `retriable()` predicate. `SinkException` is `sealed` over the four subtypes, and the
retry decision is a pattern `switch` in the sink, so the compiler refuses to build a switch that
misses a failure type. A predicate would invert that: a new subtype would inherit "retry" from the
base class without anyone deciding it should.

**Retry is inline, not Flink's restart strategy.** A restart replays from the last checkpoint, so
the operator re-emits every window since then — including the ones that already succeeded — and one
failing window becomes a burst of duplicate POSTs. An inline retry re-sends only the record that
failed. Four attempts with a delay of 100ms doubling (capped at 10s, jittered) keep the worst case
far below the 10-minute checkpoint timeout, so the blocking wait cannot starve a checkpoint.
Retries are counted in the `sinkRetries` metric.

**After retries are exhausted the sink throws.** The record then reaches Flink's fixed-delay
restart strategy, which is the only remaining option; swallowing it would silently lose a window.
`RESTART_MAX_ATTEMPTS` bounds that loop so a permanently failing sink ends as a failed job.

A stateful restart still replays from the last checkpoint, so a POST that already succeeded before
the failure can be delivered again. Idempotency on the receiving side is therefore required — the
sink cannot supply it.

### TriState Field-Level Merge

`TriState<T>` models three states for enriched fields:
- `Undefined` — not yet seen
- `Defined(value)` — successfully enriched
- `Null` — enrichment failed (poison state)

Merge semantics:
- `Defined` keeps first defined value (idempotent)
- `Undefined` takes first event's value
- `Null` stays null (failed enrichment propagates)

Inspired by field-level eventual consistency in Braze profile updates (REA experience). Currently untested with real data (all `token_symbol` fields are null).

### Exploit Detection Patterns

Session window (60-second gap) with heuristics:
1. **Rapid Transfers** — >10 events in single session
2. **Large Transfer** — single transfer >1000 tokens
3. **Supply Mismatch** — netLocked vs netMinted differs by >10%

Patterns 4-6 (Unusual Size, New Address, Cross-Chain Correlation) designed but not implemented. Requires sliding windows with stddev, block metadata lookup, and message hash matching respectively.

## Known Gaps

See `INGESTER-AGGREGATOR-STATUS.md` for comprehensive gap analysis:
- Analytics service not migrated (Flink sinks receive 404 and drop the record)
- Token symbols all null (both ingesters hardcode `token_symbol: None`)
- Message hash not calculated (cross-chain correlation impossible)
- Only 3/6 exploit patterns implemented
- `LARGE_TRANSFER_THRESHOLD` compares raw token units, which are not comparable across
  tokens with different decimals
- Malformed Kafka records are dropped and counted, not routed to the DLQ topic the
  ingesters publish to
