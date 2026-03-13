# Aggregator

Apache Flink job that consumes DEX events from Kafka and produces trading + liquidity analytics.

## Current Scope

- Trading flow:
  - Input: `TOPIC_TRADING_EVENTS` (`dex-trading-events`)
  - Output: `TOPIC_TRADING_ANALYTICS` (`dex-trading-analytics`)
  - Window: 5-minute tumbling event-time
  - Operators: `aggregate` + `ProcessWindowFunction`
  - Metrics: TWAP, OHLC, volume, trader activity, gas stats, arbitrage count
- Liquidity flow:
  - Input: `TOPIC_LIQUIDITY_EVENTS` (`dex-liquidity-events`)
  - Output: `TOPIC_LIQUIDITY_ANALYTICS` (`dex-liquidity-analytics`)
  - Window: 1-hour tumbling event-time
  - Operators: `ProcessWindowFunction` (full-window scan)
  - Metrics: mint/burn counts, gross flows, net liquidity change, LP churn

## Contract with Ingester

- Ingester publishes through Dapr pub/sub, so Kafka values are CloudEvents envelopes.
- This aggregator now unwraps CloudEvents and decodes Avro payload from `data_base64` or `data`.
- Raw Avro Kafka values are not accepted.
- Avro schemas are loaded from `src/main/resources/avro`.

## Requirements

- Java 21
- Maven
- Kafka broker reachable from Flink runtime

## Configuration

- `KAFKA_BOOTSTRAP_SERVERS` (fallback: `KAFKA_BOOTSTRAP`, default `localhost:9092`)
- `TOPIC_TRADING_EVENTS` (default `dex-trading-events`)
- `TOPIC_LIQUIDITY_EVENTS` (default `dex-liquidity-events`)
- `TOPIC_TRADING_ANALYTICS` (default `dex-trading-analytics`)
- `TOPIC_LIQUIDITY_ANALYTICS` (default `dex-liquidity-analytics`)
- `FLINK_CONSUMER_GROUP` (default `dex-processor`)
- `FLINK_PARALLELISM` (default `2`)
- `FLINK_CHECKPOINT_MS` (default `10000`)

## Build and Test

```bash
cd aggregator
mvn test
mvn -DskipTests package
```

## Run

```bash
flink run target/aggregator-1.0-SNAPSHOT.jar
```

## Notes

- Decode failures are routed to side outputs and printed (`trading-decode-errors`, `liquidity-decode-errors`).
- Flink checkpointing is currently disabled in `StreamProcessor`.
