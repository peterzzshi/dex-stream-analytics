# solana-ingester

Streams Raydium AMM v4 events from Solana, enriches them with pricing and token metadata, and publishes to Kafka via
Dapr.

Runs as a single instance today. The design is one instance per Solana RPC endpoint — same binary, different config,
publishing to shared Kafka topics with EVM events.

## Run

```bash
docker compose up -d solana-ingester solana-ingester-dapr
```

Set `SOLANA_RPC_URL` in `.env`. It embeds an API key, so it is never written to a file in the repo or image.

## Design Notes

- **Instruction decoding**: Raydium uses 8-byte discriminators. After matching, data is deserialized via borsh.
  Instructions arrive as `PartiallyDecoded` from routers; swaps reach the pool as inner CPI, not top-level.
- **Vault resolution**: Vaults resolved per instruction by matching AMM authority against token accounts in
  `pre/postTokenBalances` (instruction forms place vaults at different positions; fixed indices cannot work).
- **Balance diffs**: Actual amounts come from `meta.preTokenBalances` vs `meta.postTokenBalances` for vault accounts,
  not instruction data (handles slippage, fees).
- **Finality gate**: Events buffer until their slot reaches the configured commitment. Subscription uses `confirmed`;
  fetch uses `finalized` (unreconciled). `finalized` = 13s latency, zero reorg risk.
- **Enrichment at ingest**: Pyth pricing and SPL token metadata resolved here, so Flink reprocessing reproduces the same
  values.
- **Failure visibility**: Publishes retry with backoff, then fall back to the DLQ topic (`TOPIC_DLQ`). If the Dapr
  sidecar itself is unreachable, the process logs and exits rather than degrading silently. Decode failures are
  logged. No metrics.

## Configuration

Two sources; `src/config.rs` is the source of truth for both.

**Environment variables** (all required): `CONFIG_FILE`, `DAPR_HOST`, `DAPR_HTTP_PORT`, `PUBSUB_NAME`, `TOPIC_TRADING`,
`TOPIC_LIQUIDITY`, `TOPIC_DLQ`, `SOLANA_RPC_URL`, `SOLANA_COMMITMENT`, `FINALITY_BUFFER_SIZE`,
`FINALITY_POLL_INTERVAL_SECS`, `APP_PORT`.

**Config file** (`CONFIG_FILE`, `solana.toml` at the repo root): chain identity, program addresses, Pyth max age, a
`[[tokens]]` table (mint, symbol, Pyth account, Pyth feed id, stablecoin flag) and `[[pools]]`. Pool names are derived
from token symbols. Unknown keys, malformed addresses, duplicate mints or pools, and pools referencing undeclared
tokens fail at startup.

A token's `pyth_account` is a Pyth receiver `PriceUpdateV2` account and `pyth_feed_id` is the 64-hex id that account
must hold; the oracle reads the id out of the account and rejects the price on mismatch. Receiver accounts are
per-poster, so an address can go stale — the feed id is the stable identity.

**Validation at startup:**

- Unknown TOML keys → fail
- Invalid Solana addresses (base58, 32-44 chars) → fail, field name in error
- Duplicate mints (case-sensitive) → fail
- Pool references undeclared token → fail
- Stablecoin with pyth_account or pyth_feed_id → fail (pick one)
- pyth_account without pyth_feed_id, or the reverse → fail
- pyth_feed_id that is not 64 hex characters → fail
- No pools configured → fail

RPC retry, publish retry, and the reconnect delay are constants in code.
`SOLANA_RPC_URL` is a secret and is redacted from `Debug` output.

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy -- -D warnings
cargo fmt -- --check
```
