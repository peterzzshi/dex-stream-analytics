# solana-ingester

Streams Raydium AMM v4 events from Solana, enriches them with pricing and token metadata, and publishes to Kafka via
Dapr.

Runs as a single instance today. The design is one instance per Solana RPC endpoint — same binary, different config,
publishing to shared Kafka topics with EVM events.

## Run

```bash
docker compose --profile live up -d solana-ingester
```

Set `SOLANA_RPC_URL` in `.env`. It embeds an API key, so it is never written to a file in the repo or image.

## Configuration

Environment variables only; `src/config.rs` is the source of truth. Groups:

- **Dapr and topics:** `DAPR_HOST`, `DAPR_HTTP_PORT`, `PUBSUB_NAME`, `TOPIC_TRADING`, `TOPIC_LIQUIDITY`, `TOPIC_DLQ`
- **Chain:** `CHAIN_ID` (CAIP-2), `CHAIN_NAME`, `CHAIN_VENUE`, `SOLANA_RPC_URL`, `SOLANA_COMMITMENT`
- **Pools:** `POOL_{n}_NAME`, `_ADDRESS`, `_TOKEN0`, `_TOKEN1`, `_VAULT0_INDEX`, `_VAULT1_INDEX`,
  `_USER_AUTHORITY_INDEX` (a single pool can use the unindexed `POOL_*` names)
- **Programs and oracle:** `RAYDIUM_AMM_V4_PROGRAM`, `TOKEN_METADATA_PROGRAM`, `PYTH_PRICE_FEEDS`,
  `PYTH_STABLECOINS`, `PYTH_MAX_AGE_SECONDS`, `KNOWN_TOKEN_SYMBOLS`
- **Finality and retry:** `FINALITY_BUFFER_SIZE`, `FINALITY_POLL_INTERVAL_SECS`, `PUBLISH_*`, `RPC_*`,
  `RECONNECT_DELAY_*`, `APP_PORT`

All are required except the four `RPC_*` retry settings, which have defaults.

## Design Notes

- **Instruction decoding**: Raydium uses 8-byte discriminators. After matching, data is deserialized via borsh.
- **Balance diffs**: Actual amounts come from `meta.preTokenBalances` vs `meta.postTokenBalances` for vault accounts,
  not instruction data (handles slippage, fees).
- **Finality gate**: Events buffer until their slot reaches the configured commitment (`finalized` = 13s latency, zero
  reorg risk).
- **Enrichment at ingest**: Pyth pricing and SPL token metadata resolved here, so Flink reprocessing reproduces the same
  values.
- **Failure visibility**: Publishes retry with backoff, then fall back to the DLQ topic (`TOPIC_DLQ`). Decode failures
  are logged. No metrics.

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy -- -D warnings
cargo fmt -- --check
```
