# Ingester

Streams Uniswap V2-compatible Polygon events (`Swap`, `Mint`, `Burn`), enriches them, and publishes to Kafka through Dapr.

## Scope

- WebSocket subscription to one pair contract
- Enrichment:
  - block timestamp (all events)
  - gas used and gas price (`Swap`)
  - token symbols (cached)
  - USD volume (`Swap`, Chainlink + fallback)
- Publishing:
  - `Swap` -> `TOPIC_TRADING_EVENTS`
  - `Mint`/`Burn` -> `TOPIC_LIQUIDITY_EVENTS`

## Data and Encoding

- Avro schemas live in `ingester/internal/avro` and mirror `schemas/avro`.
- Ingester sends Avro bytes to Dapr.
- Kafka receives Dapr CloudEvents envelopes; Avro bytes are inside the CloudEvent `data` field.

## Pricing Logic (`Swap`)

Priority order:
1. Stablecoin shortcut (`USDC`, `USDT`, `DAI`) -> `1.0`
2. Chainlink price for `token0`
3. Chainlink price for `token1`
4. Fallback to swap-implied price (token1-denominated)

Cache behavior:
- Token symbols: no expiry
- Prices: 5-minute TTL
- Chainlink responses older than 24h are rejected

## Required Environment Variables

All are required. Missing values fail startup.

- `POLYGON_RPC_URL` (`ws://` or `wss://`)
- `PAIR_ADDRESS`
- `APP_PORT`
- `DAPR_HOST`
- `DAPR_HTTP_PORT`
- `PUBSUB_NAME`
- `TOPIC_TRADING_EVENTS`
- `TOPIC_LIQUIDITY_EVENTS`

`DAPR_GRPC_PORT` and `PRODUCER_*` are not used by ingester.

## Run

From repo root:

```bash
docker compose up -d kafka dapr-placement ingester ingester-dapr
docker logs -f ingester
```

Local script (without Dapr/Kafka publish path):

```bash
./scripts/run-ingester.sh
```

## Verify Topics

```bash
docker exec kafka kafka-console-consumer \
  --bootstrap-server kafka:9092 \
  --topic dex-trading-events --from-beginning --max-messages 1

docker exec kafka kafka-console-consumer \
  --bootstrap-server kafka:9092 \
  --topic dex-liquidity-events --from-beginning --max-messages 1
```

Expected: CloudEvents JSON envelopes with Avro payload in `data`.
