# Architecture

Technical architecture for the Bridge Security Monitor: real-time cross-chain bridge exploit detection across Ethereum and Solana.

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
        AT[bridge-alerts<br/>Aggregated threat signals]
    end

    subgraph "Aggregator · Flink 2 / Java 21"
        F1[Transfer Volume<br/>5-min tumbling]
        F2[Supply Tracking<br/>5-min tumbling]
        F3[Cross-Chain Flow<br/>5-min tumbling]
        F4[Exploit Detection<br/>60-sec session]
        F5[Reentrancy Detection<br/>5-sec tumbling]
    end

    subgraph "Alerting Service · Kotlin / Ktor"
        DAPR_A[Dapr sidecar]
        SVC[Alert Service<br/>Dapr consumer]
        REDIS[(Redis<br/>Recent alerts cache)]
        WS[WebSocket<br/>Broadcast]
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

    F1 & F2 & F3 & F4 & F5 -- Kafka sink --> AT
    AT --> DAPR_A
    DAPR_A --> SVC
    SVC --> REDIS
    SVC --> WS
```

## Event-to-Alert Mapping

| Input Topic | Event Types | Pipeline | Window | Output |
|---|---|---|---|---|
| `bridge-transfer-events` | `direction`: lock, unlock | Transfer Volume | 5-min tumbling | Aggregated lock/unlock counts and totals per bridge |
| `bridge-supply-events` | `operation`: mint, burn | Supply Tracking | 5-min tumbling | Net minted, total wrapped supply per bridge |
| Both topics | Both records | Cross-Chain Flow | 5-min tumbling | Net flow and imbalance ratio across chains |
| Both topics | Both records | Exploit Detection | 60-sec session | Threat signals (rapid transfers, large amounts, supply mismatch) |
| `bridge-transfer-events` | `direction`: lock, unlock | Reentrancy Detection | 5-sec tumbling | Multiple lock/unlock calls in one transaction |

All pipelines output to 5 separate Kafka topics (`bridge-alerts-transfer-volume`, `bridge-alerts-supply-tracking`, `bridge-alerts-cross-chain-flow`, `bridge-alerts-exploit-detection`, `bridge-alerts-reentrancy`), consumed by the alerting service via Dapr subscription. **Note**: Flink currently uses RestApiSink (Phase A migration to KafkaSink pending).

Window lengths are env vars (`*_WINDOW_SECONDS`, `EXPLOIT_SESSION_GAP_SECONDS`) with no defaults; the values here are the `docker-compose.yml` ones.

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

### Dapr Pub/Sub Pattern

The pipeline uses Dapr on both ends of the Kafka streams for consistent pub/sub abstraction:

**Ingesters (Publish):**
- Rust ingesters publish via Dapr HTTP API (`POST /v1.0/publish/{pubsubname}/{topic}`)
- Dapr sidecar handles Kafka producer configuration, retries, and CloudEvents wrapping
- Decouples ingester from Kafka client libraries and protocol details

**Alerting Service (Subscribe):**
- Dapr delivers messages via HTTP POST to subscription endpoints
- Service registers subscriptions via `/dapr/subscribe` endpoint
- Dapr sidecar handles Kafka consumer group management, offset tracking, and delivery guarantees

**Why Dapr for both?**
- Consistent pub/sub interface across Rust (ingesters) and Kotlin (alerting service)
- Simplifies testing (mock Dapr HTTP endpoints vs embedded Kafka)
- Production-ready retries, dead-lettering, and observability without custom implementation

**Flink uses native Kafka connectors** (not Dapr) because it needs exactly-once semantics via Kafka transactions and tight integration with checkpointing—capabilities that require native Kafka client protocol, not an HTTP abstraction.

### Kafka Output Topics

Flink aggregator outputs to dedicated alert topics, consumed by the alerting service:

| Pipeline | Output Topic | Schema | Retention |
|---|---|---|---|
| Transfer Volume | `bridge-alerts-transfer-volume` | TransferVolumeResult | 7 days |
| Supply Tracking | `bridge-alerts-supply-tracking` | SupplyTrackingResult | 7 days |
| Cross-Chain Flow | `bridge-alerts-cross-chain-flow` | CrossChainFlowResult | 7 days |
| Exploit Detection | `bridge-alerts-exploit-detection` | ExploitDetectionResult | 30 days |
| Reentrancy Detection | `bridge-alerts-reentrancy` | ReentrancyResult | 30 days |

All topics use JSON serialization (not Avro) for easier Dapr/HTTP consumption. BigInteger fields serialized as strings to prevent precision loss.

**Exactly-once guarantee:** Flink sinks use Kafka transactions with two-phase commit, coordinated with Flink checkpoints. A checkpoint completes only after Kafka confirms the transaction commit, so no duplicates or losses even across restarts.

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

See `GAPS.md` for comprehensive gap analysis:
- Token symbols all null (both ingesters hardcode `token_symbol: None`)
- Message hash not calculated (cross-chain correlation impossible)
- Only 3/6 exploit patterns implemented
- `LARGE_TRANSFER_THRESHOLD` compares raw token units, which are not comparable across
  tokens with different decimals
- Malformed Kafka records are dropped and counted, not routed to the DLQ topic the
  ingesters publish to
