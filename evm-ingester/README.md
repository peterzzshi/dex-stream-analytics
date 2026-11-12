# evm-ingester

Subscribes to bridge contract events on an EVM chain over a WebSocket RPC, waits for block-depth
finality, and publishes `BridgeTransferEvent` / `BridgeSupplyEvent` records to Kafka via Dapr.

One process serves every bridge in its config file, each with its own listener task. The design is
one instance per EVM chain — same binary, different config — publishing to the same topics as the
Solana ingester.

## Run

```bash
docker compose up -d evm-ingester evm-ingester-dapr
```

Set `ETH_RPC_URL` (a `wss://` URL) in `.env` at the repo root. It carries an API key, so it is never
written to a file in the repo or the image. Without it the process exits at startup — the compose
service is configured to crash-loop rather than idle.

## Configuration

Two sources. `src/config.rs` is the source of truth for both.

**Environment variables** — all required, no fallback defaults:

`DAPR_HOST`, `DAPR_HTTP_PORT`, `PUBSUB_NAME`, `TOPIC_TRADING`, `TOPIC_LIQUIDITY`, `TOPIC_DLQ`,
`ETH_RPC_URL`.

| Variable | Meaning |
|---|---|
| `TOPIC_TRADING` | transfers topic (`bridge-transfer-events`) |
| `TOPIC_LIQUIDITY` | supply topic (`bridge-supply-events`) |
| `TOPIC_DLQ` | dead-letter topic for evicted and undeliverable events |

The names are legacy from the DEX version; the topics they point at carry bridge records.

`BRIDGE_CONFIG_FILE` is optional and defaults to `evm-bridges.toml`.

**Config file** (`evm-bridges.toml`) — one `[[bridge]]` table per contract:

| Key | Notes |
|---|---|
| `name` | display name, copied into the record |
| `protocol` | `wormhole`, `layerzero`, `hop` or `allbridge` |
| `address` | checksummed or lowercase `0x` + 40 hex chars |
| `chain_id` | numeric; rendered as CAIP-2 `eip155:<id>` |
| `chain_name` | free text, copied into the record |

**Validated at load** (`bridge_file_config.rs`): unknown TOML keys, empty bridge list, malformed
address, duplicate address (case-insensitive), and unknown protocol all fail at startup with the
offending field and value in the message.

## Design Notes

- **Reorg handling**: logs flagged `removed: true` by the subscription are dropped, not forwarded.
- **Finality gate**: an event becomes final only once the chain has advanced 12 blocks past it
  (`confirmations` is a constant in `config.rs`, not a TOML key). Depth-based, so no separate poller
  is needed — unlike the Solana ingester's external-slot model.
- **Gas data**: populated from the transaction receipt (`gas_used`, `effective_gas_price`), fetched
  when the event is emitted, not when it is first seen.
- **Failure visibility**: events evicted from the finality buffer on overflow are dead-lettered to
  `TOPIC_DLQ`; publishes retry with backoff and then fall back to that same topic. An unreachable
  Dapr sidecar logs and exits rather than degrading silently. Decode failures are logged and the log
  is skipped. There is no metrics exporter and no health endpoint.

## Test & Lint

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```
