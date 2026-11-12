# Alerting Service

Kotlin/Ktor service that consumes bridge security alerts from Flink via Dapr, caches recent alerts in
Redis, and broadcasts them to WebSocket clients.

## Requirements

- JDK 21
- Redis 7+
- Dapr sidecar with the `kafka-pubsub` component (for subscribed mode)

## Build & Test

```bash
./gradlew test
./gradlew build
```

Avro alert classes are generated from `../schemas` at build time (Gradle plugin
`com.github.davidmc24.gradle.plugin.avro`); the generated sources land in
`build/generated-main-avro-java`.

## Run

```bash
REDIS_HOST=localhost REDIS_PORT=6379 ./gradlew run
```

The server binds `0.0.0.0:8080` unconditionally — `APP_PORT` is not read by the server. If Redis is
unreachable the process fails at startup, not on first request.

Full stack: `docker compose up -d --build` from the repo root (see `GAPS.md` for the currently broken
image build).

## Configuration

| Variable | Default | Description |
|---|---|---|
| `REDIS_HOST` | `localhost` | Redis host |
| `REDIS_PORT` | `6379` | Redis port |
| `ALERT_RETENTION_LIMIT` | `100` | Alerts kept per bridge+chain |
| `WS_KEEPALIVE_SECONDS` | `30` | WebSocket ping period |

`SCHEMAS_PATH` is read into `AppConfig` but unused — Avro decoding uses compiled generated classes,
not `.avsc` files at runtime.

## API

### WebSocket

- `GET /ws/alerts` — live alert stream, one JSON object per text frame
  - `bridgeAddress` — exact match
  - `chain` — CAIP-2 chain id; Cross-Chain Flow alerts never match (they carry no chain)
  - `severity` — minimum severity: `LOW`, `MEDIUM`, `HIGH`, `CRITICAL`
  - `alertTypes` — comma-separated, e.g. `EXPLOIT_DETECTION,REENTRANCY`

### REST

- `GET /health` — pings Redis, reports WebSocket connection count
- `GET /alerts/{bridgeAddress}/{chain}?limit=100` — recent alerts for one bridge+chain, newest first

### Dapr

- `POST /dapr/subscribe` — returns the five subscriptions (Dapr also loads the declarative
  subscriptions from `../dapr/subscriptions/alerting-subscriptions.yaml`)
- `POST /events/{transfer-volume,supply-tracking,cross-chain-flow,exploit-detection,reentrancy}` —
  CloudEvent handler per topic

## Message Format

Dapr delivers a CloudEvent whose `data_base64` holds the Avro-binary alert. The service decodes it with
a `SpecificDatumReader` against the generated class. A missing or undecodable `data_base64` is logged
and dropped; the handler still returns 200 so the sidecar does not redeliver a poison message
indefinitely.

## Storage

Redis sorted set per `bridge:alerts:{bridgeAddress}:{chain}`, scored by `windowEnd` epoch-millis,
capped at `ALERT_RETENTION_LIMIT`. `ZADD ... GT` prevents a replayed older window from displacing a
newer one.

See [ARCHITECTURE.md](../ARCHITECTURE.md) for the full data flow.
