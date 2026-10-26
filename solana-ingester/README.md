# solana-ingester

Streams Raydium AMM v4 events from Solana, enriches them with pricing and token metadata, and publishes to Kafka via
Dapr.

Runs as a single instance today. The design is one instance per Solana RPC endpoint — same binary, different config,
publishing to shared Kafka topics with EVM events.

## Run

```bash
docker compose --profile live up -d solana-ingester
```

The `live` profile mounts `config/solana-ingester.yaml` and requires `SOLANA_RPC_URL` and `SOLANA_WS_URL` to be set in
`.env`.

## Configuration

`config/solana-ingester.yaml` holds structural config — Dapr coordinates, topic names, program addresses, and finality
settings:

```yaml
dapr:
  host: solana-ingester-dapr
  grpcPort: 50001
  pubsubName: kafka-pubsub

topics:
  trading: dex-trading-events
  liquidity: dex-liquidity-events

solana:
  chain: solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp  # CAIP-2 (mainnet-beta)
  rpcUrl: ${SOLANA_RPC_URL}
  wsUrl: ${SOLANA_WS_URL}
  venue: raydium-amm-v4
  commitment: finalized  # processed/confirmed/finalized
  programs:
    - address: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
      name: Raydium AMM v4
```

Environment variables:

| Variable         | Required | Description                                                   |
|------------------|----------|---------------------------------------------------------------|
| `CONFIG_PATH`    | yes      | Path to YAML config                                           |
| `SOLANA_RPC_URL` | yes      | HTTP RPC endpoint (e.g., https://api.mainnet-beta.solana.com) |
| `SOLANA_WS_URL`  | yes      | WebSocket endpoint (e.g., wss://api.mainnet-beta.solana.com)  |

Secrets never live in the YAML: only the `${VAR}` reference does.

## Design Notes

- **Instruction decoding**: Raydium uses 8-byte discriminators. After matching, data is deserialized via borsh.
- **Balance diffs**: Actual amounts come from `meta.preTokenBalances` vs `meta.postTokenBalances` for vault accounts,
  not instruction data (handles slippage, fees).
- **Finality gate**: Events buffer until their slot reaches the configured commitment (`finalized` = 13s latency, zero
  reorg risk).
- **Enrichment at ingest**: Pyth pricing and SPL token metadata resolved here, so Flink reprocessing reproduces the same
  values.
- **Failure visibility**: Decode and publish failures are logged. No dead-letter topic or metric yet.

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy -- -D warnings
cargo fmt -- --check
```
