# Alerting Service

Kotlin/Ktor real-time alerting service consuming bridge security threat signals from Flink via Dapr/Kafka subscription. Maintains recent alert cache in Redis and broadcasts to WebSocket clients with severity-based filtering.

## Requirements

- JDK 21
- Redis 7+
- Dapr sidecar with Kafka component

## Build & Test

```bash
./gradlew test
./gradlew build
```

## Run

Standalone (requires reachable Redis and Dapr sidecar):
```bash
REDIS_HOST=localhost REDIS_PORT=6379 APP_PORT=8080 ./gradlew run
```

Full stack (recommended):

```bash
docker compose up -d --build   # from repo root
```

Connect to WebSocket at `ws://localhost:8080/ws/alerts` for real-time bridge security alerts.

## Configuration

| Variable                    | Default     | Description                        |
|-----------------------------|-------------|------------------------------------|
| `APP_PORT`                  | `8080`      | HTTP server port                   |
| `REDIS_HOST`                | `localhost` | Redis host                         |
| `REDIS_PORT`                | `6379`      | Redis port                         |
| `REDIS_PASSWORD`            | —           | Redis password (optional)          |

## API

### WebSocket
- `GET /ws/alerts` - Real-time alert broadcast
  - Query params: `bridgeAddress`, `chain`, `minSeverity` (all optional)

### REST
- `GET /health` - Health check
- `GET /alerts/recent?bridgeAddress={addr}&limit={n}` - Recent alerts (default 100)

### Dapr Subscriptions
Service registers subscriptions for Flink output topics via `/dapr/subscribe`:
- `bridge-alerts-transfer-volume`
- `bridge-alerts-supply-tracking`
- `bridge-alerts-cross-chain-flow`
- `bridge-alerts-exploit-detection`
- `bridge-alerts-reentrancy`

See [ARCHITECTURE.md](../ARCHITECTURE.md) for complete data flow.

## Design Notes

- **Dapr subscription pattern** - Service receives POST to `/events/{alert-type}` from Dapr sidecar when new alerts arrive
- **Redis as cache only** - Stores recent 100 alerts per bridge (not a time-series store)
- **WebSocket fan-out** - Kotlin coroutines handle concurrent connections with structured concurrency
- **Alert deduplication** - Same alert pattern within 5-minute window is suppressed
