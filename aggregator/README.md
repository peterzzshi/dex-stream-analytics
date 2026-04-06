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
  - Current decode support: `MintEvent`, `BurnEvent`, `TransferEvent` (transfer currently ingested for correlation phases)

## Contract with Ingester

- Ingester publishes through Dapr pub/sub, so Kafka values are CloudEvents envelopes.
- Liquidity deserialization pattern-matches CloudEvent `type` before Avro deserialization.
- This aggregator unwraps CloudEvents and decodes Avro payload from `data_base64` or `data`.
- Raw Avro Kafka values are not accepted.
- Avro schemas are loaded from `src/main/resources/avro`.

Liquidity event type routing:
- `com.dex.events.mint` -> `MintEvent`
- `com.dex.events.burn` -> `BurnEvent`
- `com.dex.events.transfer` -> `TransferEvent` (currently ingested for correlation phases)

## Requirements

- Java 21
- Maven
- Kafka broker reachable from Flink runtime

## Configuration

All environment variables below are required:

- `KAFKA_BOOTSTRAP_SERVERS`
- `TOPIC_TRADING_EVENTS`
- `TOPIC_LIQUIDITY_EVENTS`
- `TOPIC_TRADING_ANALYTICS`
- `TOPIC_LIQUIDITY_ANALYTICS`
- `FLINK_CONSUMER_GROUP`
- `FLINK_PARALLELISM`
- `FLINK_CHECKPOINT_MS`

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
