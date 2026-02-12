# Ingester

Streams Polygon QuickSwap swap events for a single pair address and prints each event as structured JSON logs.

## Requirements

- Go 1.21
- A Polygon RPC endpoint with WebSocket support (public or private)

## Getting a Free RPC Endpoint

The ingester requires WebSocket support for real-time event streaming. Free options:

### Alchemy (Recommended)
1. Sign up at https://www.alchemy.com/
2. Create a new app for Polygon Mainnet
3. Copy the WebSocket URL (format: `wss://polygon-mainnet.g.alchemy.com/v2/YOUR-API-KEY`)

### Infura
1. Sign up at https://www.infura.io/
2. Create a new project
3. Select Polygon network
4. Use WebSocket endpoint (format: `wss://polygon-mainnet.infura.io/ws/v3/YOUR-PROJECT-ID`)

### Public Endpoints
**Note:** Public endpoints often have rate limits and may not support WebSocket subscriptions.

## Run

**Option 1: Using setup script (Recommended):**
```bash
# From project root - one-time setup
cd ..
./setup.sh          # Creates .env and downloads dependencies
nano .env           # Edit with your WebSocket URL

# Run ingester
./run-ingester.sh
```

**Option 2: Manual with environment variables:**
```bash
cd ingester
POLYGON_RPC_URL="wss://polygon-mainnet.g.alchemy.com/v2/YOUR-API-KEY" \
PAIR_ADDRESS="0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827" \
LOG_LEVEL="info" \
go run ./cmd/ingester
```

**Option 3: Using .env file in project root:**
```bash
# If .env exists in parent directory
cd ..
export $(cat .env | grep -v '^#' | xargs)
cd ingester
go run ./cmd/ingester
```

## Configuration

All configuration is stored in `.env` file in the project root.

**Required:**
- `POLYGON_RPC_URL` - WebSocket endpoint (format: `wss://...`)
- `PAIR_ADDRESS` - QuickSwap pair contract address (default: WMATIC/USDC)
- `LOG_LEVEL` - `debug`, `info`, `warn`, or `error` (default: `info`)

**Optional (for Kafka publishing):**
- `DAPR_HTTP_PORT`, `PUBSUB_NAME`, `TOPIC_DEX_EVENTS`, etc.

## Troubleshooting

### Error: "notifications not supported"
This error means your RPC endpoint doesn't support WebSocket subscriptions. Make sure:
- Your URL starts with `wss://` (not `https://`)
- You're using an RPC provider that supports subscriptions (Alchemy, Infura, etc.)
- Your API key is valid and has sufficient quota
