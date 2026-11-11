# Data Model

Bridge security monitoring event schemas and data flow.

## Event Schemas

Two Avro schemas in `schemas/`:

### BridgeTransferEvent (`bridge-transfer-events`)

Value movement across bridges (lock/unlock operations).

| Field | Type | Notes |
|---|---|---|
| eventId | string | `txHash:logIndex` |
| blockNumber / blockTimestamp | long | Block context (timestamp in seconds) |
| transactionHash / logIndex | string / int | Log location |
| chain / chainName | string | CAIP-2 format (`eip155:1`, `solana:5eykt...`) + display name |
| bridgeAddress | string | Bridge contract address |
| protocol | enum | `wormhole`, `layerzero`, `hop`, `allbridge` |
| direction | enum | `lock`, `unlock` |
| sender / recipient | string | Initiator vs receiver |
| token / token_symbol | string / string? | Token address + symbol (symbol currently null) |
| amount | decimal | Token amount (precision preserved) |
| destinationChain / sourceChain | string? | Cross-chain routing info |
| messageHash / sequenceNumber | string? / long? | Protocol-specific correlation (currently null) |
| gasUsed / gasPrice | long / string | Transaction metrics (currently zero) |
| eventTimestamp | long | Ingester capture time (milliseconds) |

**Retention**: 7 days

### BridgeSupplyEvent (`bridge-supply-events`)

Wrapped token supply changes (mint/burn operations).

| Field | Type | Notes |
|---|---|---|
| (identification fields) | — | Same as BridgeTransferEvent |
| operation | enum | `mint`, `burn` |
| recipient | string | Who receives minted/burned tokens |
| wrappedToken / wrappedTokenSymbol | string / string? | Wrapped token address + symbol |
| amount | decimal | Token amount |
| sourceChain / messageHash | string? | Cross-chain correlation |
| gasUsed / gasPrice / eventTimestamp | — | Same as BridgeTransferEvent |

**Retention**: 30 days

## Data Flow

```
Blockchain Logs → Ingester (Rust) → Kafka (2 topics) → Flink Aggregator (5 pipelines) → REST API → Analytics Service
```

**Finality guarantees**:
- EVM: 12-block confirmation depth with deduplication cache
- Solana: External finalized-slot polling

**Event ordering**: Guaranteed within transaction (logIndex). Across transactions, event-time windows with 60s bounded-out-of-orderness watermarks absorb reordering.

**Validation**: Ingester validates log structure and skips unclassifiable logs. Avro encoding enforces schema on write. Aggregator validates CloudEvent `type` before decoding; malformed payloads are logged and dropped (not routed to DLQ).

## Aggregated Analytics

Five Flink pipelines produce window-aggregated results sent via HTTP POST to analytics service:

| Pipeline | Window | Key | Output |
|---|---|---|---|
| Transfer Volume | 5-min tumbling | `(chain, bridgeAddress)` | Lock/unlock counts, total amounts |
| Supply Tracking | 5-min tumbling | `(chain, bridgeAddress)` | Net minted, total wrapped supply |
| Cross-Chain Flow | 5-min tumbling | `bridgeAddress` | Net flow, imbalance ratio |
| Exploit Detection | 60-sec session | `bridgeAddress:chain` | Suspicious patterns detected |
| Reentrancy Detection | 5-sec tumbling | `transactionHash` | Multiple lock/unlock in single tx |

Window durations are env vars with no defaults; values shown are from `docker-compose.yml`.

## Known Limitations

- Token symbols always null (no enrichment implemented)
- Message hash not calculated (cross-chain correlation impossible)
- Gas data placeholder (no transaction receipt fetching)
- DLQ topic exists but aggregator doesn't route to it
