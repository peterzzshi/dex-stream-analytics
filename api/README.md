# API (Kotlin Skeleton)

Kotlin sink/API skeleton for aggregated DEX analytics using DAPR-compatible HTTP endpoints.

## Requirements

- JDK 21
- Gradle 8+

## Run

```bash
cd api
APP_PORT=8080 gradle run
```

## Endpoints

- `GET /health`
- `POST /events/analytics` (CloudEvent envelope with analytics payload)
- `GET /pairs/{pair}/twap`
- `GET /pairs/{pair}/volume`
- `GET /analytics/summary`
- `GET /dapr/subscribe`

## Configuration

- `APP_PORT`: HTTP port for the API server (default: `8080`)
