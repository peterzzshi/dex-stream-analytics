# evm-ingester

Streams Uniswap V2 events from one EVM chain, enriches them with pricing, gas, and token
symbols, and publishes to Kafka via Dapr.

Runs as a single instance today. The design is one instance per **EVM** chain — same binary,
different config, publishing to shared Kafka topics. Chain identity and pool set come from the
YAML config, not the binary. A non-EVM chain (Solana) needs a separate ingester binary, not
another instance of this one — see [ARCHITECTURE.md](../ARCHITECTURE.md#future-work).

## Run

```bash
docker compose --profile live up -d evm-ingester
```

The `live` profile mounts `config/evm-ingester.yaml` and requires `ETH_RPC_URL` to be set in
`.env` (the config references it as `${ETH_RPC_URL}`).

## Configuration

`config/evm-ingester.yaml` holds everything structural — Dapr coordinates, topic names, chains,
pools, and finality depth. It is versioned and reviewable, and mounted read-only into the
container, so editing pools needs no rebuild.

```yaml
dapr:
  host: evm-ingester-dapr
  httpPort: 3500
  pubsubName: kafka-pubsub

topics:
  trading: dex-trading-events
  liquidity: dex-liquidity-events

app:
  port: 3000

chains:
  - chain: eip155:1                 # CAIP-2
    name: Ethereum
    rpc: ${ETH_RPC_URL}             # expanded from the environment at startup
    venue: uniswap-v2               # names the CloudEvent source
    confirmations: 64               # blocks buffered before publish; 0 disables
    pools:
      - pairAddress: "0xB4e1...C9Dc"
        name: USDC/WETH
```

Environment variables:

| Variable                         | Required | Description                                                            |
|----------------------------------|----------|------------------------------------------------------------------------|
| `CONFIG_PATH`                    | yes      | Path to the YAML config (compose sets `/app/config/evm-ingester.yaml`) |
| Any `${VAR}` named in the config | yes      | Expanded at load; startup fails if unset — today that is `ETH_RPC_URL` |

Secrets never live in the YAML: only the `${VAR}` reference does.

## Design notes

- **Enrichment is frozen at ingest.** Chainlink prices and token symbols are resolved and cached
  here, so an event's enriched fields are fixed at write time. Flink replay then reproduces the
  same numbers; moving enrichment downstream would make reprocessing depend on the oracle's
  later state.
- **Finality gate.** Events are buffered for `confirmations` blocks before publish, so reorged
  events are never emitted. The buffer is in-memory: a restart drops anything observed but not
  yet published.
- **Failure visibility.** Decode and publish failures are logged and routed to an error channel.
  There is no dead-letter topic or metric yet.
- **One listener and one finality buffer per pool.** Buffers are not shared across chains:
  release compares an event's block number to a single known tip.

## Test

```bash
go test ./...
```

## Lint

```bash
golangci-lint run
```
