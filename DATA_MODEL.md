# Data Model

Event and alert schemas for the bridge security pipeline. All seven `.avsc` files live in root
`schemas/` and are the single source of truth: the Maven build (`aggregator`) and the Gradle build
(`alerting-service`) both codegen from that directory, and `common/` embeds the same files for the
Rust ingesters.

## Ingest Events

Two Avro records, one per topic, published by the ingesters as CloudEvents with Avro binary
`data_base64`.

### BridgeTransferEvent — `bridge-transfer-events` (22 fields, 7-day retention)

Value movement across bridges.

| Field | Type | Notes |
|---|---|---|
| eventId | string | `txHash:logIndex` |
| blockNumber / blockTimestamp | long | Block context; timestamp in seconds |
| transactionHash / logIndex | string / int | Log location; reentrancy is keyed on the hash |
| chain / chainName | string | CAIP-2 (`eip155:1`, `solana:5eykt...`) + display name |
| bridgeAddress | string | Bridge contract or program address |
| protocol | enum | `wormhole`, `layerzero`, `hop`, `allbridge` |
| direction | enum | `lock`, `unlock` |
| gasUsed / gasPrice | long / string | EVM: receipt `gas_used` / `effective_gas_price` |
| eventTimestamp | long | Ingester capture time (ms) |
| sender / recipient | string | Initiator vs receiver |
| token / tokenSymbol | string / string? | Token address; symbol currently null |
| amount | decimal | Token amount, precision preserved |
| destinationChain / sourceChain | string? | Cross-chain routing |
| messageHash / sequenceNumber | string? / long? | Set for LayerZero nonce; Wormhole VAA hash not computed |

**Retention**: 7 days (provisioned in `kafka-init`)

### BridgeSupplyEvent — `bridge-supply-events` (19 fields, 30-day retention)

Wrapped-token supply changes.

| Field | Type | Notes |
|---|---|---|
| eventId, blockNumber, blockTimestamp, transactionHash, logIndex | — | Same semantics as the transfer record |
| chain / chainName | string | CAIP-2 + display name |
| bridgeAddress | string | Bridge contract or program address |
| protocol | enum | Same four protocols |
| operation | enum | `mint`, `burn` |
| gasUsed / gasPrice | long / string | Solana: fee in `gasUsed`, `gasPrice = "0"` |
| eventTimestamp | long | Ingester capture time (ms) |
| recipient | string | Receiver of minted/burned tokens |
| wrappedToken / wrappedTokenSymbol | string / string? | Wrapped token address; symbol currently null |
| amount | decimal | Token amount |
| sourceChain | string? | Originating chain |
| messageHash | string? | Not computed |

**Retention**: 30 days

Note: the Solana ingester currently emits no `burn` events — `solana-bridges.toml` declares only
lock/unlock/mint instructions. Supply burns are only detectable on EVM. Tracked in `GAPS.md`.

## Alert Schemas

Written by the Flink sinks to five topics, Avro binary, read back by the alerting service.

| Schema | Topic | Distinguishing fields |
|---|---|---|
| `TransferVolumeAlert` | `bridge-alerts-transfer-volume` | `lockCount`, `unlockCount`, `totalLockAmount`, `totalUnlockAmount`, `token0Symbol`, `token1Symbol` |
| `SupplyTrackingAlert` | `bridge-alerts-supply-tracking` | `mintCount`, `burnCount`, `totalMinted`, `totalBurned`, `netMinted`, `tokenSymbol` |
| `CrossChainFlowAlert` | `bridge-alerts-cross-chain-flow` | `totalInflow`, `totalOutflow`, `netFlow`, `imbalanceRatio` — **no `chain` field** |
| `ExploitDetectionAlert` | `bridge-alerts-exploit-detection` | `suspiciousPatterns` (enum list), `largestSingleTransfer`, `rapidTransferCount`, `supplyMismatchDetected`, locked/minted totals |
| `ReentrancyAlert` | `bridge-alerts-reentrancy` | `transactionHash`, `lockCount`, `unlockCount` |

Every alert carries `bridgeAddress`, `protocol`, `windowStart`, `windowEnd`, `severity`
(`LOW`/`MEDIUM`/`HIGH`/`CRITICAL`) and a string `alertType` (`TRANSFER_VOLUME`, `SUPPLY_TRACKING`,
`CROSS_CHAIN_FLOW`, `EXPLOIT_DETECTION`, `REENTRANCY`). The alerting service dispatches on
`alertType` and the WebSocket filter matches on it.

`token0Symbol`/`token1Symbol` are DEX-era names retained in the transfer alert schema; the pipeline
only ever populates one token, and both are null today.

## Data Flow

```
Chains → Rust ingester → Dapr → Kafka (2 topics) → Flink (5 pipelines) → Kafka (5 alert topics)
       → Dapr subscription → Ktor service → Redis (recent alerts) + WebSocket (live)
```

**Finality**: EVM 12-block depth with dedupe cache; Solana finalized-slot polling.

**Ordering**: `logIndex` orders events within a transaction. Across transactions, event-time windows
with a 5 s bounded-out-of-orderness watermark absorb reordering; windows close on window-end, not on
arrival, so `windowStart`/`windowEnd` are derivable from `windowEnd` and the configured duration.

**Validation**: The ingester validates log/instruction structure and drops unclassifiable entries.
Avro encoding enforces the schema on write. The aggregator checks the CloudEvent `type` before
decoding; a malformed payload is counted and dropped, not routed to the DLQ (see `GAPS.md`).

## Storage

Redis holds the most recent `ALERT_RETENTION_LIMIT` (default 100) alerts per
`bridge:alerts:{bridgeAddress}:{chain}`, as a sorted set scored by `windowEnd` epoch-millis. New
entries are added with `ZADD ... GT` so a replayed window cannot overwrite a newer one, and
`ZREMRANGEBYRANK` trims the oldest. Cross-Chain Flow alerts are stored under the literal chain
segment `cross-chain` because they span chains.

## Known Limitations

- Token symbols always null (no enrichment implemented)
- Message hash not computed for Wormhole (cross-chain correlation impossible)
- No Solana burn events, so supply drains on Solana are invisible
- DLQ topic exists but the aggregator drops deserialization failures instead of routing to it
