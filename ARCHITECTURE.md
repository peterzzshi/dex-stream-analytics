# Architecture

Real-time cross-chain bridge exploit detection across Ethereum and Solana.

## System Overview

```mermaid
graph TD
    subgraph Chains
        ETH[Ethereum mainnet<br/>Wormhole · LayerZero · Hop]
        SOL[Solana<br/>Wormhole Portal · Allbridge]
    end

    subgraph "Ingesters · Rust"
        EVM[evm-ingester]
        SOLING[solana-ingester]
        DAPR_E[Dapr sidecar]
        DAPR_S[Dapr sidecar]
    end

    subgraph "Kafka · KRaft"
        KT[bridge-transfer-events<br/>direction: lock | unlock]
        KS[bridge-supply-events<br/>operation: mint | burn]
        DLQ[bridge-dlq-events]
        A1[bridge-alerts-transfer-volume]
        A2[bridge-alerts-supply-tracking]
        A3[bridge-alerts-cross-chain-flow]
        A4[bridge-alerts-exploit-detection]
        A5[bridge-alerts-reentrancy]
    end

    subgraph "Aggregator · Flink 2 / Java 21"
        F1[Transfer Volume · 5-min tumbling]
        F2[Supply Tracking · 5-min tumbling]
        F3[Cross-Chain Flow · 5-min tumbling]
        F4[Exploit Detection · 60-s session]
        F5[Reentrancy · 5-s tumbling]
    end

    subgraph "Alerting Service · Kotlin / Ktor"
        SVC[HTTP consumer<br/>5 endpoints]
        REDIS[(Redis<br/>recent alerts per bridge)]
        WS[WebSocket fan-out]
    end

    ETH -- "WebSocket · eth_subscribe" --> EVM
    SOL -- "WebSocket · logsSubscribe<br/>RPC · finalized slot" --> SOLING

    EVM --> DAPR_E
    SOLING --> DAPR_S
    DAPR_E & DAPR_S -- publish --> KT
    DAPR_E & DAPR_S -- publish --> KS

    KT --> F1
    KS --> F2
    KT & KS -- union --> F3
    KT & KS -- union --> F4
    KT --> F5

    F1 --> A1
    F2 --> A2
    F3 --> A3
    F4 --> A4
    F5 --> A5

    A1 & A2 & A3 & A4 & A5 -- "Dapr subscription" --> SVC
    SVC --> REDIS
    SVC --> WS
```

F2 (Supply Tracking) reads only `bridge-supply-events`; F1 and F5 read only
`bridge-transfer-events`. F3 and F4 read the union of both, normalized to `BridgeActivity`.

## Event-to-Alert Mapping

| Pipeline | Input | Key | Window | Output topic / schema |
|---|---|---|---|---|
| Transfer Volume | transfer | `bridgeAddress:chain` | 5-min tumbling | `bridge-alerts-transfer-volume` / `TransferVolumeAlert` |
| Supply Tracking | supply | `bridgeAddress:chain` | 5-min tumbling | `bridge-alerts-supply-tracking` / `SupplyTrackingAlert` |
| Cross-Chain Flow | union | `bridgeAddress` | 5-min tumbling | `bridge-alerts-cross-chain-flow` / `CrossChainFlowAlert` |
| Exploit Detection | union | `bridgeAddress:chain` | 60-s session | `bridge-alerts-exploit-detection` / `ExploitDetectionAlert` |
| Reentrancy | transfer | `transactionHash` | 5-s tumbling | `bridge-alerts-reentrancy` / `ReentrancyAlert` |

Window lengths and the watermark delay are required env vars with no defaults
(`TRANSFER_WINDOW_SECONDS`, `SUPPLY_WINDOW_SECONDS`, `FLOW_WINDOW_SECONDS`,
`EXPLOIT_SESSION_GAP_SECONDS`, `REENTRANCY_WINDOW_SECONDS`, `WATERMARK_DELAY_SECONDS`); the values
above are the ones in `docker-compose.yml`. Every key is a serialized String, so
`(chain, bridgeAddress)` is a composite key made of two fields, not a Flink tuple.

Cross-Chain Flow is the one pipeline keyed on the bridge alone: it exists to compare chains, so
scoping it per chain would defeat it. Its alert schema therefore carries no `chain` field.

## Design Decisions

### Chain-Agnostic Bridge Identity

Bridges are identified by `(chain, bridgeAddress)`; a contract address alone is ambiguous across
chains. `chain` is CAIP-2 (`eip155:1`, `solana:5eykt...`), and `chainName` is the display form.

| Layer | Implementation |
|---|---|
| Ingester | One binary per chain family; bridges declared in `evm-bridges.toml` / `solana-bridges.toml` |
| Event schema | `chain` (CAIP-2) + `chainName` + `protocol` enum on every event |
| Flink key | `bridgeAddress:chain` on volume, supply and exploit pipelines |

**Why Solana**: it forces the abstraction to handle non-EIP-155 chain IDs and a finality model
that is not confirmation counting. If the pipeline only ever saw EVM, "chain-neutral" would be an
untested claim.

### Two Event Topics, One Record Each

- `bridge-transfer-events` — `BridgeTransferEvent`, 22 fields, `direction` = lock | unlock, 7-day retention
- `bridge-supply-events` — `BridgeSupplyEvent`, 19 fields, `operation` = mint | burn, 30-day retention

The split gives independent retention (value transfers are replayed for correlation; supply changes
are kept longer for audit) and lets consumers subscribe only to what they need.

Lock/Unlock and Mint/Burn were originally four record types, consumed separately and re-unioned with
`Either<,>` at the aggregator. The schemas had identical field lists, so the split bought no type
safety — only a duplicate struct, a duplicate consumer, and a union that could not be switched
exhaustively (`Either` is not sealed, so any `switch` needs a `default` that silently swallows new
subtypes). They were consolidated into two records with enum discriminants: a new direction is now a
compile error at every switch that misses it, and the four logical event types still exist — as field
values, not record names.

The shapes genuinely differ (22 vs 19 fields), so the `BridgeActivity` union feeding F3 and F4 is a
real normalization, not two names for the same payload.

### Protocol Type Safety

`protocol` is an Avro enum (`wormhole`, `layerzero`, `hop`, `allbridge`), validated three times:
`BridgeProtocol::from_str()` at config load, the Avro schema on decode (unknown symbol fails), and
the Flink type system thereafter. No string comparisons in the hot path.

### Finality

**EVM** — depth-based. Events buffer until the chain head is 12 blocks past them, with a
deduplication cache; handles reorgs up to 12 blocks deep. No polling task needed.

**Solana** — external. A separate task polls the finalized slot every `FINALITY_POLL_INTERVAL_SECS`
and the buffer releases events once their block reaches it. Cluster finality is authoritative, so
there is no confirmation counting.

Both use the same `FinalityBuffer` from `common/` with `Depth` and `External` modes.

### Dapr for Pub/Sub, Native Connectors for Flink

Ingesters publish via `POST /v1.0/publish/{pubsub}/{topic}` to their sidecar; the sidecar owns the
Kafka producer, retries, and CloudEvent wrapping. The alerting service is subscribed through Dapr
(via `/dapr/subscribe` and the declarative subscriptions in `dapr/subscriptions/`), so consumer-group
and offset management stay out of the app.

Flink bypasses Dapr: it needs checkpoint-coupled consumer offsets, event-time watermarks and
backpressure, which an HTTP pub/sub abstraction cannot provide.

### Kafka Connectors and Delivery Semantics

Flink reads with `KafkaSource` (`OffsetsInitializer.earliest()`, group ids
`bridge-transfer-consumer` / `bridge-supply-consumer`) and writes with `KafkaSink` using
`AvroSerializationSchema.forSpecific(...)`. Alert values are **Avro binary**, not JSON. The alerting
service decodes them from the CloudEvent `data_base64` field.

Sinks use `DeliveryGuarantee.AT_LEAST_ONCE`, not exactly-once: two-phase-commit sinks require Kafka
transaction support and idle-transaction timeouts, and the at-least-once path keeps a failed sink from
holding back checkpoints indefinitely. The consequence is explicit — a restart can replay a window and
emit the same alert twice. Consumers deduplicate on `(alertType, bridgeAddress, windowStart, windowEnd)`.
See `GAPS.md` for the operational caveat that a duplicate is not yet suppressed at the source.

Checkpointing runs every `CHECKPOINT_INTERVAL_SECONDS`; restarts are `fixed-delay`, bounded by
`RESTART_MAX_ATTEMPTS` and `RESTART_DELAY_SECONDS`, so a permanently failing sink ends as a failed job
rather than an endless loop.

### Severity Thresholds

Computed in `AlertSeverity`, calibrated for demo-level activity and expressed in raw token units
(see the decimal caveat in `GAPS.md`):

| Alert | LOW → CRITICAL |
|---|---|
| Transfer Volume | total ≥ 10k → > 1M |
| Supply Tracking | any burn-only window (mints = 0) is CRITICAL; else abs(netMinted) ≥ 5k → > 500k |
| Cross-Chain Flow | imbalance ratio ≥ 0.15 → > 0.5 |
| Exploit Detection | 1 pattern → > 2 patterns |
| Reentrancy | 2 calls → > 5 calls |

### TriState Field-Level Merge

`TriState<T>` distinguishes three enrichment outcomes: `Undefined` (not seen), `Defined(value)`, and
`Null` (enrichment failed). Merge keeps the first `Defined` value and propagates `Null`. It exists so a
transport failure is never silently mistaken for "no data". Both ingesters currently hardcode
`token_symbol: None`, so the mechanism is exercised structurally but not with real symbols.

### Exploit Detection Patterns

Session window (60-s gap), evaluated per `(bridge, chain)`:

1. **RAPID_TRANSFERS** — more than 10 events in one session
2. **LARGE_TRANSFER** — a single transfer above the threshold
3. **SUPPLY_MISMATCH** — netLocked vs netMinted differ by more than 10%

Patterns 4–6 (unusual size via stddev, new address via block metadata, cross-chain correlation via
message hash) need data the pipeline does not have yet. See `GAPS.md`.

## Known Gaps

`GAPS.md` is authoritative; the headline items are unpopulated token symbols, no message hash, 3/6
exploit patterns, and — the one that breaks a fresh deployment — the five alert topics are not
provisioned by `kafka-init`.
