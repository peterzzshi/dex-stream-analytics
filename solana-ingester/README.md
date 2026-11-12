# solana-ingester

Subscribes to bridge program logs on Solana over a WebSocket RPC, waits for finalized-slot
confirmation, and publishes `BridgeTransferEvent` / `BridgeSupplyEvent` records to Kafka via Dapr.

One process serves every bridge in its config file. The design is one instance per RPC endpoint —
same binary, different config — publishing to the same topics as the EVM ingester.

## Run

```bash
docker compose up -d solana-ingester solana-ingester-dapr
```

Set `SOLANA_RPC_URL` (a `wss://` URL) in `.env` at the repo root. It carries an API key, so it is
never written to a file in the repo or the image. Without it the process exits at startup — the
compose service crash-loops by design.

Health check: `GET http://localhost:<APP_PORT>/health` on the ingester itself.

## Configuration

Two sources. `src/config.rs` is the source of truth for both.

**Environment variables** — all required, no fallback defaults:

`DAPR_HOST`, `DAPR_HTTP_PORT`, `PUBSUB_NAME`, `TOPIC_TRADING`, `TOPIC_LIQUIDITY`, `TOPIC_DLQ`,
`SOLANA_RPC_URL`, `SOLANA_COMMITMENT`, `FINALITY_BUFFER_SIZE`, `FINALITY_POLL_INTERVAL_SECS`,
`APP_PORT`.

| Variable | Meaning |
|---|---|
| `SOLANA_COMMITMENT` | subscription commitment level; the stack uses `finalized` |
| `FINALITY_BUFFER_SIZE` | max buffered events before overflow is dead-lettered |
| `FINALITY_POLL_INTERVAL_SECS` | how often the finalized slot is re-read |
| `APP_PORT` | port for the health server |

`BRIDGE_CONFIG_FILE` is optional and defaults to `solana-bridges.toml`.

**Config file** (`solana-bridges.toml`) — one `[[bridge]]` table per program:

| Key | Notes |
|---|---|
| `name` | display name, copied into the record |
| `protocol` | `wormhole` or `allbridge` |
| `program_id` | base58 Pubkey |
| `chain_name` | free text, copied into the record |

Chain identity is fixed to `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp` in code, not read from the
file.

**Validated at load** (`bridge_file_config.rs`): unknown TOML keys, empty bridge list, unparseable
`program_id`, duplicate program id, and unknown protocol all fail at startup.

## Design Notes

- **Pipeline layering**: `bridge_handler` owns every I/O call for a bridge — fetch the transaction,
  walk instructions, hand off. `bridge_decoder` and `bridge_normalizer` are pure: neither awaits
  anything, so a record's contents are fixed by the transaction, not by state read later.
- **Instruction decoding**: bridge programs are reached through CPI from routers, so instructions
  appear as `PartiallyDecoded` and must be matched by program id rather than position.
- **Finality gate**: events buffer until their slot is at or below the finalized slot, which a
  separate poller refreshes every `FINALITY_POLL_INTERVAL_SECS`. `finalized` costs roughly 13 s of
  latency and carries no reorg risk.
- **Gas data**: the transaction fee goes in `gasUsed` and `gasPrice` is the string `"0"` — Solana has
  no gas price.
- **No burn events**: no burn instruction is configured, so Solana contributes lock, unlock and mint
  records only. Supply-side drain detection is currently EVM-only (see `../GAPS.md`).
- **Failure visibility**: publishes retry with backoff, then fall back to `TOPIC_DLQ`. An unreachable
  Dapr sidecar logs and exits. Decode failures are logged and the transaction is skipped. No metrics
  exporter.

## Test & Lint

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```
