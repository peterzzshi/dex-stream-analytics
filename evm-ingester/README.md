# evm-ingester

Streams Uniswap V2 events from an EVM chain over a WebSocket RPC subscription, enriches them with Chainlink pricing,
and publishes to Kafka via Dapr.

Runs as a single instance today. The design is one instance per EVM chain — same binary, different config,
publishing to shared Kafka topics with Solana events.

## Run

```bash
docker compose up -d evm-ingester evm-ingester-dapr
```

Set `ETH_RPC_URL` in `.env`. It embeds an API key, so it is never written to a file in the repo or image.

## Configuration

Two sources; `src/config.rs` is the source of truth for both.

**Environment variables** (all required): `CONFIG_FILE`, `DAPR_HOST`, `DAPR_HTTP_PORT`, `PUBSUB_NAME`, `TOPIC_TRADING`,
`TOPIC_LIQUIDITY`, `TOPIC_DLQ`, `ETH_RPC_URL`.

**Config file** (`CONFIG_FILE`, `evm.toml` at the repo root): chain identity (CAIP-2 id, name), confirmations, a
`[[tokens]]` table (address, symbol, Chainlink feed, stablecoin flag) and `[[pools]]`.

**Validation at startup:**

- Unknown TOML keys → fail
- Invalid addresses (not 0x + 40 hex chars) → fail, field name in error
- Duplicate token/pool addresses (case-insensitive) → fail
- Stablecoin with chainlink_feed → fail (pick one)
- No pools configured → fail
- Pool references undeclared token → fail

RPC retry (max retries, backoff, timeout) is a default in code.
`ETH_RPC_URL` is a secret and is redacted from `Debug` output.

## Design Notes

- **Reorg handling**: logs with `removed: true` from the WebSocket subscription are dropped, not forwarded.
- **Finality gate**: events are only considered final once the chain has advanced `confirmations` blocks past them (12
  for Ethereum mainnet). Depth-based, so no separate polling task is needed, unlike the Solana ingester's
  external finality buffer.
- **Enrichment at ingest**: Chainlink pricing resolved here, so Flink reprocessing reproduces the same values.
- **Failure visibility**: Events evicted from the finality buffer on overflow are dead-lettered to `TOPIC_DLQ`.
  Publishes retry with backoff, then fall back to the same DLQ topic. If the Dapr sidecar itself is unreachable,
  the process logs and exits rather than degrading silently. Decode failures are logged. No metrics, no health
  check endpoint.

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy -- -D warnings
cargo fmt -- --check
```
