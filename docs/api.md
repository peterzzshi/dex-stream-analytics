# API Documentation

The Analytics API exposes RESTful endpoints for accessing aggregated DEX analytics data.

## Base URL

```
http://localhost:8080
```

## Authentication

Currently, no authentication is required (development mode). In production, implement API keys or OAuth.

## Endpoints

### Health Check

Check if the API is running.

**Endpoint:** `GET /health`

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2026-02-10T12:00:00Z",
  "version": "1.0.0"
}
```

**Status Codes:**
- `200 OK`: Service is healthy
- `503 Service Unavailable`: Service is unhealthy

---

### Get Analytics Summary

Get overall analytics across all monitored pairs.

**Endpoint:** `GET /analytics/summary`

**Response:**
```json
{
  "totalPairs": 1,
  "totalWindows": 150,
  "latestWindowEnd": 1707566700,
  "pairs": [
    {
      "pairAddress": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827",
      "token0Symbol": "WMATIC",
      "token1Symbol": "USDC",
      "latestTwap": 0.85,
      "windowCount": 150
    }
  ]
}
```

**Status Codes:**
- `200 OK`: Success
- `500 Internal Server Error`: Server error

---

### Get Pair TWAP

Get the latest Time-Weighted Average Price for a specific pair.

**Endpoint:** `GET /pairs/{pairAddress}/twap`

**Path Parameters:**
- `pairAddress` (string, required): DEX pair contract address

**Query Parameters:**
- `limit` (integer, optional): Number of windows to return (default: 1)

**Example:**
```bash
curl http://localhost:8080/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/twap?limit=10
```

**Response:**
```json
{
  "pairAddress": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827",
  "token0Symbol": "WMATIC",
  "token1Symbol": "USDC",
  "windows": [
    {
      "windowId": "2026-02-10T12:00:00Z",
      "windowStart": 1707566400,
      "windowEnd": 1707566700,
      "twap": 0.85123,
      "openPrice": 0.85000,
      "closePrice": 0.85200,
      "highPrice": 0.85500,
      "lowPrice": 0.84800,
      "swapCount": 42
    }
  ]
}
```

**Status Codes:**
- `200 OK`: Success
- `404 Not Found`: Pair not found
- `400 Bad Request`: Invalid parameters

---

### Get Pair Volume

Get volume statistics for a specific pair.

**Endpoint:** `GET /pairs/{pairAddress}/volume`

**Path Parameters:**
- `pairAddress` (string, required): DEX pair contract address

**Query Parameters:**
- `limit` (integer, optional): Number of windows to return (default: 10)

**Example:**
```bash
curl http://localhost:8080/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/volume?limit=5
```

**Response:**
```json
{
  "pairAddress": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827",
  "token0Symbol": "WMATIC",
  "token1Symbol": "USDC",
  "windows": [
    {
      "windowId": "2026-02-10T12:00:00Z",
      "windowStart": 1707566400,
      "windowEnd": 1707566700,
      "totalVolume0": "12500000000000000000000",
      "totalVolume1": "10625000000",
      "volumeUSD": 21250.50,
      "swapCount": 42,
      "uniqueTraders": 28
    }
  ]
}
```

**Status Codes:**
- `200 OK`: Success
- `404 Not Found`: Pair not found

---

### Get Pair Analytics (Full)

Get complete analytics for a specific pair.

**Endpoint:** `GET /pairs/{pairAddress}/analytics`

**Path Parameters:**
- `pairAddress` (string, required): DEX pair contract address

**Query Parameters:**
- `limit` (integer, optional): Number of windows to return (default: 10)
- `startTime` (integer, optional): Unix timestamp for start time
- `endTime` (integer, optional): Unix timestamp for end time

**Example:**
```bash
curl "http://localhost:8080/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/analytics?limit=5"
```

**Response:**
```json
{
  "pairAddress": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827",
  "token0Symbol": "WMATIC",
  "token1Symbol": "USDC",
  "windows": [
    {
      "windowId": "2026-02-10T12:00:00Z",
      "windowStart": 1707566400,
      "windowEnd": 1707566700,
      "twap": 0.85123,
      "openPrice": 0.85000,
      "closePrice": 0.85200,
      "highPrice": 0.85500,
      "lowPrice": 0.84800,
      "priceVolatility": 0.00234,
      "totalVolume0": "12500000000000000000000",
      "totalVolume1": "10625000000",
      "volumeUSD": 21250.50,
      "swapCount": 42,
      "uniqueTraders": 28,
      "largestSwapValue": "500000000000000000000",
      "largestSwapAddress": "0x1234567890abcdef1234567890abcdef12345678",
      "totalGasUsed": "1250000",
      "avgGasPrice": "45000000000",
      "arbitrageCount": 5,
      "repeatedTraders": 12,
      "processedAt": 1707566705
    }
  ]
}
```

**Status Codes:**
- `200 OK`: Success
- `404 Not Found`: Pair not found
- `400 Bad Request`: Invalid parameters

---

### List All Monitored Pairs

Get list of all pairs being monitored.

**Endpoint:** `GET /pairs`

**Response:**
```json
{
  "pairs": [
    {
      "pairAddress": "0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827",
      "token0Symbol": "WMATIC",
      "token1Symbol": "USDC",
      "latestWindowEnd": 1707566700,
      "totalWindows": 150
    }
  ],
  "total": 1
}
```

**Status Codes:**
- `200 OK`: Success

---

## Data Models

### AggregatedAnalytics

| Field | Type | Description |
|-------|------|-------------|
| windowId | string | Unique identifier for window |
| windowStart | long | Window start (Unix seconds) |
| windowEnd | long | Window end (Unix seconds) |
| pairAddress | string | DEX pair address |
| token0Symbol | string | Symbol of token0 |
| token1Symbol | string | Symbol of token1 |
| twap | double | Time-Weighted Average Price |
| openPrice | double | First price in window |
| closePrice | double | Last price in window |
| highPrice | double | Highest price in window |
| lowPrice | double | Lowest price in window |
| priceVolatility | double | Standard deviation of prices |
| totalVolume0 | string | Total volume of token0 |
| totalVolume1 | string | Total volume of token1 |
| volumeUSD | double | Volume in USD (if available) |
| swapCount | int | Number of swaps |
| uniqueTraders | int | Unique trader addresses |
| largestSwapValue | string | Value of largest swap |
| largestSwapAddress | string | Address of largest swap |
| totalGasUsed | string | Total gas used |
| avgGasPrice | string | Average gas price |
| arbitrageCount | int | Detected arbitrage patterns |
| repeatedTraders | int | Traders appearing 3+ times |
| processedAt | long | Processing timestamp |

## Error Responses

All endpoints may return error responses in this format:

```json
{
  "error": "Error message description",
  "code": "ERROR_CODE",
  "timestamp": "2026-02-10T12:00:00Z"
}
```

**Common Error Codes:**
- `PAIR_NOT_FOUND`: Requested pair address not found
- `INVALID_PARAMETER`: Invalid query parameter
- `INTERNAL_ERROR`: Server error

## Rate Limiting

Currently no rate limiting in development mode. In production:
- Rate limit: 100 requests per minute per IP
- Burst: 20 requests

## Usage Examples

### cURL Examples

Get latest TWAP:
```bash
curl http://localhost:8080/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/twap
```

Get volume over last 10 windows:
```bash
curl http://localhost:8080/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/volume?limit=10
```

### Python Example

```python
import requests

BASE_URL = "http://localhost:8080"

# Get TWAP
response = requests.get(f"{BASE_URL}/pairs/0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827/twap")
data = response.json()
print(f"Latest TWAP: {data['windows'][0]['twap']}")
```

### JavaScript Example

```javascript
const BASE_URL = "http://localhost:8080";

async function getTwap(pairAddress) {
  const response = await fetch(`${BASE_URL}/pairs/${pairAddress}/twap`);
  const data = await response.json();
  return data.windows[0].twap;
}

// Usage
getTwap("0x6e7a5FAFcec6BB1e78bAE2A1F0B612012BF14827")
  .then(twap => console.log("Latest TWAP:", twap));
```

## Swagger/OpenAPI

Swagger UI is available at: http://localhost:8080/swagger/index.html

API specification (OpenAPI 3.0): http://localhost:8080/swagger/doc.json

## WebSocket Support (Future)

Real-time updates via WebSocket will be available in Phase 4:

```
ws://localhost:8080/ws/pairs/{pairAddress}/live
```

## Support

For API issues or questions, refer to:
- [Setup Guide](setup.md)
- [Architecture Documentation](architecture.md)
- Project repository issues
