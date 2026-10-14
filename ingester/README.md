# Ingester

Streams Uniswap V2 events from one blockchain, enriches them with pricing, gas, and token
symbols, and publishes to Kafka via Dapr.

Runs as a single instance today. The design is one instance per **EVM** chain — same binary,
different `RPC_URL` / `PAIR_ADDRESS`, publishing to shared Kafka topics. Chain identity
(`CHAIN` / `CHAIN_NAME`) is not yet implemented. A non-EVM chain (Solana) needs a separate
ingester binary, not another instance of this one — see
[ARCHITECTURE.md](../ARCHITECTURE.md#future-work).

## Run

```bash
docker compose --profile live up -d ingester
```

The `live` profile requires `POLYGON_RPC_URL` to be set in `.env`.

## Configuration

| Variable | Required | Description |
|---|---|---|
| `POLYGON_RPC_URL` | yes (live) | Polygon WebSocket endpoint (`wss://...`) |
| `PAIR_ADDRESS` | yes | Target Uniswap V2 pair contract |
| `FINALITY_CONFIRMATIONS` | no | Blocks to wait before publishing (default `64`; `0` disables) |
| `DAPR_HOST` / `DAPR_HTTP_PORT` | yes | Dapr sidecar address |
| `PUBSUB_NAME` | yes | Dapr pub/sub component name |
| `TOPIC_TRADING_EVENTS` / `TOPIC_LIQUIDITY_EVENTS` | yes | Output topics |
| `APP_PORT` | no | Health/readiness port (default `3000`) |
| `LOG_LEVEL` | no | Log verbosity (default `info`) |

## Design notes

- **Enrichment is frozen at ingest.** Chainlink prices and token symbols are resolved and cached
  here, so an event's enriched fields are fixed at write time. Flink replay then reproduces the
  same numbers; moving enrichment downstream would make reprocessing depend on the oracle's
  later state.
- **Finality gate.** Events are buffered for `FINALITY_CONFIRMATIONS` blocks before publish, so
  reorged events are never emitted. The buffer is in-memory: a restart drops anything observed
  but not yet published.
- **Failure visibility.** Decode and publish failures are logged and routed to an error channel.
  There is no dead-letter topic or metric yet.

## Test

```bash
go test ./...
```
