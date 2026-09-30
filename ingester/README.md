# Ingester

Streams Uniswap V2 events from one blockchain, enriches them with pricing/gas/symbols, and publishes to Kafka via Dapr. Deploy three instances (Polygon, Arbitrum, Base) pointing to different RPC endpoints and pair addresses.

## Run

```bash
docker compose up -d ingester-polygon ingester-arbitrum ingester-base
```

## Configuration

Environment variables per instance:
- `POLYGON_RPC_URL` / `ARBITRUM_RPC_URL` / `BASE_RPC_URL` (WebSocket)
- `PAIR_ADDRESS` (target Uniswap V2 pair)
- `FINALITY_CONFIRMATIONS` (default: 64 blocks)
