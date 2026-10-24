# Analytics Service

Kotlin/Ktor REST API consuming Flink-aggregated analytics from Kafka (via Dapr subscriptions),
computing pool health scores, and serving time-series query endpoints. Stores data in Redis
sorted sets.

## Requirements

- JDK 21
- Redis 7+
- Kafka broker (via Dapr)

## Build & Test

```bash
./gradlew test
./gradlew build
```

## Run

Standalone (requires reachable Redis):
```bash
REDIS_HOST=localhost REDIS_PORT=6379 APP_PORT=8080 ./gradlew run
```

Full stack (recommended):

```bash
docker compose up -d --build   # from repo root
```

Visit http://localhost:8080/dashboard for live analytics.

## Configuration

| Variable                    | Default     | Description                        |
|-----------------------------|-------------|------------------------------------|
| `APP_PORT`                  | `8080`      | HTTP server port                   |
| `REDIS_HOST`                | `localhost` | Redis host                         |
| `REDIS_PORT`                | `6379`      | Redis port                         |
| `REDIS_PASSWORD`            | —           | Redis password (optional)          |
| `TRADING_RETENTION_HOURS`   | `168`       | Trading data retention (7 days)    |
| `LIQUIDITY_RETENTION_HOURS` | `720`       | Liquidity data retention (30 days) |
| `MEV_RETENTION_HOURS`       | `168`       | MEV alert retention (7 days)       |
| `TREND_RETENTION_HOURS`     | `168`       | Market trend retention (7 days)    |

Kafka topics and Dapr subscription routes are wired in `dapr/subscriptions/*.yaml` and match the
aggregator's output topics — see root [ARCHITECTURE.md](../ARCHITECTURE.md#event-to-analytics-mapping)
for the full contract.

## Design Notes

- **Dapr subscriptions are declarative** — no programmatic `/dapr/subscribe` endpoint; the sidecar
  reads `dapr/subscriptions/analytics-subscriptions.yaml` and pushes CloudEvents to `/events/*`
  routes.
- **Pool health scoring** — composite metric (35 % trading activity + 35 % liquidity activity +
  30 % safety/inverse of MEV alerts) computed at query time, not persisted.
- **Redis key structure** — `analytics:trading:{pair}`, `analytics:liquidity:{pair}`,
  `analytics:mev:{pair}`, `analytics:trends:{pair}`, all scored by `windowStart` or `detectedAt`
  for time-range queries. When multi-chain lands, a chain prefix joins the key.
