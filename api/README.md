# API

REST API service for aggregated DEX analytics.

## Requirements

- Go 1.21

## Run

```bash
cd api
APP_PORT="8080" \
ENVIRONMENT="development" \
LOG_LEVEL="info" \
go run ./cmd/api
```

## Configuration

- `APP_PORT`: HTTP port for the API server (default: `8080`)
- `ENVIRONMENT`: `development` or `production` (default: `development`)
- `LOG_LEVEL`: `debug`, `info`, `warn`, or `error` (default: `info`)
