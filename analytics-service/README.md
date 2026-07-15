# Analytics Service

Kotlin/Ktor service that receives aggregated DEX analytics via Dapr subscriptions, computes pool health scores, and serves time-series query endpoints. Stores data in Redis sorted sets.

## Requirements

- JDK 21
- Redis 7+

## Run

```bash
REDIS_HOST=localhost REDIS_PORT=6379 APP_PORT=8080 ./gradlew run
```

## Endpoints

### Dapr Subscription

Topics are subscribed **declaratively** via `dapr/subscriptions/*.yaml` (no programmatic `/dapr/subscribe` manifest). Dapr pushes events to these routes:

| Method | Path | Purpose |
|---|---|---|
| POST | `/events/trading-analytics` | Receive trading windows |
| POST | `/events/liquidity-analytics` | Receive liquidity windows |
| POST | `/events/pattern-analytics` | Receive MEV alerts |
| POST | `/events/market-trends` | Receive market trends |

### Trading Analytics
| Method | Path | Purpose |
|---|---|---|
| GET | `/pairs/{pair}/twap` | TWAP series |
| GET | `/pairs/{pair}/ohlc` | OHLC candlestick data |
| GET | `/pairs/{pair}/volume` | Volume and swap count |
| GET | `/pairs/{pair}/trading` | Full trading windows |
| GET | `/pairs/{pair}/latest` | Most recent trading window |

### Liquidity Analytics
| Method | Path | Purpose |
|---|---|---|
| GET | `/pairs/{pair}/liquidity` | Liquidity windows |
| GET | `/pairs/{pair}/liquidity/latest` | Most recent liquidity window |
| GET | `/pairs/{pair}/liquidity/flows` | LP token flow summary |

### Pool Health
| Method | Path | Purpose |
|---|---|---|
| GET | `/pools/{pair}/health` | Composite health score |
| GET | `/pools/{pair}/alerts` | MEV alerts for pair |
| GET | `/pools/{pair}/trends` | Market trends for pair |
| GET | `/pools/leaderboard` | All pools ranked by health |

### Summary
| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Health check |
| GET | `/analytics/summary` | Tracked pair and window counts |
| GET | `/analytics/pairs` | All pairs with latest metrics |

### Dashboard
| Method | Path | Purpose |
|---|---|---|
| GET | `/dashboard` | Real-time HTML dashboard |
| WS | `/ws/analytics` | Live analytics stream (send `{"channels":[...]}` to filter) |

All time-range endpoints accept `?from=<ms>&to=<ms>` query parameters (default: last hour).

## Pool Health Scoring

The `/pools/{pair}/health` endpoint computes a weighted composite score (0–1):

- **Trading score** (35%): volume, activity frequency, trader diversity
- **Liquidity score** (35%): LP event activity
- **Safety score** (30%): inverse of recent MEV alert count

## Configuration

| Variable | Default | Description |
|---|---|---|
| `APP_PORT` | `8080` | HTTP server port |
| `REDIS_HOST` | `localhost` | Redis host |
| `REDIS_PORT` | `6379` | Redis port |
| `REDIS_PASSWORD` | — | Redis password (optional) |
| `TRADING_RETENTION_HOURS` | `168` | Trading data retention (7 days) |
| `LIQUIDITY_RETENTION_HOURS` | `720` | Liquidity data retention (30 days) |

## Test

```bash
./gradlew test   # 30 tests
```
