# Solana Ingester

Rust-based ingester for monitoring Raydium AMM v4 pools on Solana. Decodes swap and liquidity instructions, enriches
with Pyth oracle prices, and publishes to Kafka via Dapr using the same event schemas as the EVM ingester.

## Features

- **Raydium AMM v4 support**: Decodes SwapBaseIn/Out (V1/V2), Deposit, and Withdraw instructions
- **Multi-chain compatibility**: Emits same Avro schemas as EVM ingester (aggregator/analytics-service unchanged)
- **Pyth oracle integration**: USD price enrichment for volumeUSD calculation
- **Finality buffer**: Configurable commitment level (processed/confirmed/finalized)
- **CloudEvents + Avro**: Industry-standard event envelope with schema evolution support

## Architecture

```
logsSubscribe(Raydium program)
  → getTransaction (full tx details)
  → decode instruction (discriminator + accounts)
  → diff preTokenBalances/postTokenBalances (actual fills)
  → enrich (Pyth USD price, symbol lookup)
  → finality buffer (commitment-level release)
  → CloudEvent + Avro encode
  → Dapr HTTP publish
```

## Configuration

Edit `config/ingester.yaml`:

```yaml
solana:
  rpc: ${SOLANA_RPC_URL}  # WebSocket endpoint (wss://...)
  commitment: finalized    # processed | confirmed | finalized

pools:
  - name: SOL/USDC
    address: 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2
    token0: So11111111111111111111111111111111111111112  # Wrapped SOL
    token1: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v  # USDC
```

**Environment variables**:

- `SOLANA_RPC_URL`: Solana WebSocket endpoint (e.g., Alchemy, Helius, QuickNode)
- `CONFIG_PATH`: Path to config file (default: `config/ingester.yaml`)
- `RUST_LOG`: Log level (default: `info`)

## Event Mapping

| Raydium Instruction  | Emitted Event | Topic                  | Notes                           |
|----------------------|---------------|------------------------|---------------------------------|
| SwapBaseIn/Out V1/V2 | `SwapEvent`   | `dex-trading-events`   | Fill amounts from balance diffs |
| Deposit              | `MintEvent`   | `dex-liquidity-events` | Add liquidity                   |
| Withdraw             | `BurnEvent`   | `dex-liquidity-events` | Remove liquidity                |

**Solana-specific mappings**:

- `blockNumber` → slot number
- `logIndex` → instruction index in transaction
- `gasUsed` → compute units consumed
- `gasPrice` → prioritization fee (lamports per compute unit)

## Known Limitations

### LP Token Tracking Not Implemented

**Status**: Deferred to future enhancement

**Issue**: `totalLpTokensMinted`, `totalLpTokensBurned`, and `netLpTokenChange` fields in `LiquidityAnalytics` will be
"0" for Solana events.

**Reason**: Tracking LP tokens requires parsing SPL Token Transfer instructions nested inside Deposit/Withdraw
transactions. The current implementation only decodes top-level Raydium instructions and uses `preTokenBalances`/
`postTokenBalances` diffs for token amounts.

**Impact**:

- Pool Health scoring unaffected (uses window count, not LP token amounts)
- Primary liquidity metrics (token amounts, provider counts) work correctly
- Direct `LiquidityAnalytics` queries show zero LP tokens for Solana pools

**Future Enhancement Complexity**: Medium

1. Parse `meta.innerInstructions` from transaction
2. Find SPL Token Transfer instructions with LP mint address
3. Emit `TransferEvent` for LP mint/burn
4. Requires ~200 LOC in `balances.rs` + tests

**Decision**: Ship without LP token tracking to validate core multi-chain flow first. Add if interview questions probe
liquidity depth tracking.

## Development

### Build and Test

```bash
# Build
cargo build

# Run tests
cargo test

# Run locally (requires config + env vars)
export SOLANA_RPC_URL="wss://solana-mainnet.g.alchemy.com/v2/YOUR_KEY"
cargo run
```

### Docker Compose Integration

Start Solana ingester with full stack:

```bash
# Solana only
docker compose --profile solana up

# Both EVM and Solana
docker compose --profile live --profile solana up
```

The ingester is configured via `docker-compose.yml` with the `solana` profile.

## Testing Strategy

### Unit Tests

- **decoder.rs**: Instruction discriminator routing, account validation
- **balances.rs**: Balance diff computation, edge cases
- **finality.rs**: Buffer add/release, slot advancement
- **publisher.rs**: Avro encoding (no HTTP)

Run: `cargo test`

### Integration Tests (Manual)

**Prerequisites**:

- Solana RPC endpoint (Alchemy recommended)
- Alchemy API key set in `.env`

**Steps**:

1. Start stack: `docker compose --profile solana up -d`
2. Monitor logs: `docker logs -f solana-ingester`
3. Verify Kafka: `scripts/consume_events.sh dex-trading-events`
4. Check analytics: `curl http://localhost:8080/pairs/58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2/trading`

**Success criteria**:

- Swap events with non-zero `volumeUSD` in Kafka
- Flink processes Solana events (check logs)
- Analytics-service serves SOL/USDC pool data
- Pool Health computes after >12h data

## References

- [Raydium AMM v4 Program](https://docs.raydium.io/reference/program-addresses):
  `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
- [Raydium Instructions](https://docs.raydium.io/products/amm-v4/instructions)
- [Solana RPC logsSubscribe](https://solana.com/docs/rpc/websocket/logssubscribe)
- [Pyth Solana SDK](https://docs.pyth.network/price-feeds/use-real-time-data/solana)
- [CloudEvents Spec](https://github.com/cloudevents/spec/blob/v1.0.2/cloudevents/spec.md)

## Future Work

1. **LP token tracking**: Parse SPL Token inner instructions to emit `TransferEvent`
2. **Multi-pool discovery**: Subscribe to Raydium Factory `initialize2` events
3. **Concentrated liquidity**: Extend to Raydium CLMM pools
4. **Yellowstone gRPC**: Optional low-latency mode for production
5. **Metrics**: Prometheus metrics for observability
6. **Devnet support**: Local validator integration tests
