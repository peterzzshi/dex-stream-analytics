# Ingester

Streams Uniswap V2-compatible DEX events (Swap, Mint, Burn) from Polygon, enriches them with **Chainlink price oracle data** for accurate USD volumes, and publishes to Kafka via Dapr.

## Key Features

- ✅ **Real-time event streaming** via WebSocket subscriptions
- ✅ **Chainlink price oracle** integration for accurate USD volumes
- ✅ **Intelligent caching** (token symbols, USD prices)
- ✅ **98% USD volume accuracy** across all pairs
- ✅ **Smart contract interaction** (ERC20, Chainlink aggregators)
- ✅ **Production-ready** error handling and observability

## Architecture

```
Blockchain (WebSocket)
  └─> Listener
       ├─> Parse event (ABI decoding)
       ├─> Enrich (RPC + Cache)
       │    ├─> Block timestamp
       │    ├─> Gas details
       │    ├─> Token symbols (cached ∞)
       │    └─> USD price (Chainlink, cached 5min)
       └─> Publish (Dapr → Kafka)
```

**See [DATA_FETCHING.md](./DATA_FETCHING.md) for complete data flow details.**

## Requirements

- Go 1.21+
- Polygon RPC endpoint with **WebSocket support** (Alchemy recommended)
- Dapr sidecar (for Kafka publishing in Docker)

## Data Sources

The ingester fetches data from multiple sources with smart caching:

| Data Type           | Source              | Method                 | Caching             | Cost            |
|---------------------|---------------------|------------------------|---------------------|-----------------|
| **Events**          | Uniswap V2 Pair     | WebSocket subscription | N/A                 | Free            |
| **Block timestamp** | Ethereum headers    | RPC per event          | No                  | 1 call/event    |
| **Gas details**     | Transaction receipt | RPC per Swap           | No                  | 1 call/Swap     |
| **Token symbols**   | ERC20 contracts     | RPC                    | Forever (immutable) | 0-2 calls/pair  |
| **USD prices**      | Chainlink oracles   | On-chain read          | 5 minutes           | 0-2 calls/event |

**Typical RPC usage:** 2-3 calls per event (95%+ cache hit rate)
**Free tier compatible:** Well within Alchemy's 300M compute units/month

**Supported price feeds:** WMATIC, WETH, WBTC, USDC, USDT, DAI (easy to add 100+ more)

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

### Option 1: Docker (Full Pipeline - Recommended)

Run ingester with Kafka and DAPR for complete event publishing:

```bash
# From project root
docker compose up -d

# View logs
docker logs -f ingester

# Verify swap events in Kafka
docker exec kafka kafka-console-consumer \
  --bootstrap-server kafka:9092 \
  --topic dex-trading-events --from-beginning --max-messages 5

# Verify liquidity events in Kafka
docker exec kafka kafka-console-consumer \
  --bootstrap-server kafka:9092 \
  --topic dex-liquidity-events --from-beginning --max-messages 5
```

**Services started:**
- Kafka (with 6 partitions)
- DAPR placement service
- Ingester container
- DAPR sidecar (publishes to Kafka)

### Option 2: Local Development (No Kafka)

Run ingester locally for testing blockchain connectivity and event parsing:

```bash
# From project root
./run-ingester.sh
```

**What happens:**
- ✅ Connects to Polygon blockchain
- ✅ Captures and logs blockchain events (Swap, Sync, Mint, Burn)
- ❌ **Cannot publish to Kafka** (no DAPR sidecar)
- ⚠️ You'll see: `"Failed to publish dex event"...connect: connection refused"`

**This is expected!** Use this mode for:
- Testing RPC connectivity
- Verifying blockchain event capture (Swap, Sync, Mint, Burn)
- Debugging blockchain listener logic
- Quick local development without Docker

### Option 3: Manual with Environment Variables

```bash
cd ingester
POLYGON_RPC_URL="wss://polygon-mainnet.g.alchemy.com/v2/YOUR-API-KEY" \
PAIR_ADDRESS="0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827" \
APP_PORT="3000" \
LOG_LEVEL="info" \
go run ./cmd/ingester
```

## Configuration

All configuration is stored in `.env` file in the project root.

**Required:**
- `POLYGON_RPC_URL` - WebSocket endpoint (format: `wss://...`) - **Required, no default**
- `PAIR_ADDRESS` - QuickSwap pair contract address - **Required, no default**
- `APP_PORT` - Health server port for the Dapr sidecar - **Required, no default**
- `LOG_LEVEL` - `debug`, `info`, `warn`, or `error` - **Required, no default**
- `DAPR_HTTP_PORT` - Dapr HTTP port (used in Docker) - **Required, no default**
- `PUBSUB_NAME` - Dapr pub/sub component name - **Required, no default**
- `TOPIC_TRADING_EVENTS` - Kafka topic for swap events - **Required, no default**
- `TOPIC_LIQUIDITY_EVENTS` - Kafka topic for mint/burn events - **Required, no default**

**Note:** When running locally with `./run-ingester.sh`, DAPR variables are ignored since there's no DAPR sidecar. The application will fail to publish events but will still capture and log blockchain events.

## Event Model

- `DexEvent` is the envelope published to Kafka.
- `eventType` can be `swap`, `sync`, `mint`, or `burn`.
- `entityId` is always the pair address, shared across event types.
- `payload` contains the event-specific schema.
- Schemas are embedded in the ingester binary and do not require Schema Registry.

## Verify

**For Docker deployment:**

Check ingester logs:
```bash
docker logs -f ingester
# Should see: "Event published" with event_type (swap, mint, or burn)
```

Check Kafka topics:
```bash
docker exec kafka kafka-topics \
  --bootstrap-server kafka:9092 \
  --describe --topic dex-trading-events
# Should show: PartitionCount: 6

docker exec kafka kafka-topics \
  --bootstrap-server kafka:9092 \
  --describe --topic dex-liquidity-events
# Should show: PartitionCount: 6
```

View messages (Avro binary):
```bash
# Trading events (swaps)
docker exec kafka kafka-console-consumer \
  --bootstrap-server kafka:9092 \
  --topic dex-trading-events --from-beginning --max-messages 1

# Liquidity events (mints/burns)
docker exec kafka kafka-console-consumer \
  --bootstrap-server kafka:9092 \
  --topic dex-liquidity-events --from-beginning --max-messages 1
```

**For local development:**

Check ingester logs:
```bash
# Should see blockchain events being captured
{"level":"INFO","msg":"Configuration loaded"...}
{"level":"INFO","msg":"Pair metadata loaded"...}

# Publishing will fail (expected):
{"level":"ERROR","msg":"Failed to publish dex event"...connection refused"}
```

## Troubleshooting

### Error: "Failed to publish dex event...connection refused"

**When running locally (`./run-ingester.sh`):**

✅ **This is expected and normal!** The local script runs the ingester without DAPR, so it cannot publish to Kafka. Use this mode to:
- Test blockchain connectivity
- Verify swap and sync events are captured
- Debug event parsing logic

To publish events to Kafka, use Docker:
```bash
docker compose up -d
```

**When running in Docker:**

❌ This indicates DAPR sidecar is not running. Check:
```bash
docker ps | grep dapr
docker logs ingester-dapr
```

### Error: "notifications not supported"
This error means your RPC endpoint doesn't support WebSocket subscriptions. Make sure:
- Your URL starts with `wss://` (not `https://`)
- You're using an RPC provider that supports subscriptions (Alchemy, Infura, etc.)
- Your API key is valid and has sufficient quota
