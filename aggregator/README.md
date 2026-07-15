# Aggregator

Apache Flink 2 job (Java 21) that consumes DEX events from Kafka and produces four analytics streams using different window types.

## Pipelines

| Pipeline | Input | Window | Output Topic | Key Operator |
|---|---|---|---|---|
| Trading Analytics | `dex-trading-events` | Tumbling (5 min) | `dex-trading-analytics` | `SwapAggregator` + `SwapAnalyticsWindowFunction` |
| Liquidity Analytics | `dex-liquidity-events` | Tumbling (1 h) | `dex-liquidity-analytics` | `LiquidityWindowFunction` |
| MEV Detection | Both topics (union) | Session (3 s gap) | `dex-pattern-analytics` | `MevDetectionFunction` |
| Market Trends | `dex-trading-events` | Sliding (30 min / 5 min) | `dex-market-trends` | `SwapAggregator` + `MarketTrendWindowFunction` |

Events are decoded and watermarked once per source topic, then fanned out to the four pipelines.

## Contract with Ingester

- Kafka values are Dapr CloudEvents envelopes containing Avro-encoded payloads.
- Deserializers validate the CloudEvent `type` field before Avro decode.
- Payload is read from `data_base64` (primary) or textual `data` (Dapr ISO-8859-1 fallback).
- Invalid envelopes and unsupported types are routed to side outputs (`trading-decode-errors`, `liquidity-decode-errors`).

CloudEvent type routing:

| Type | Schema |
|---|---|
| `com.dex.events.swap` | SwapEvent |
| `com.dex.events.mint` | MintEvent |
| `com.dex.events.burn` | BurnEvent |
| `com.dex.events.transfer` | TransferEvent |

## Requirements

- Java 21
- Maven
- Kafka broker reachable from Flink runtime

## Configuration

| Variable | Required | Description |
|---|---|---|
| `KAFKA_BOOTSTRAP_SERVERS` | yes | Kafka brokers |
| `TOPIC_TRADING_EVENTS` | yes | Input trading topic |
| `TOPIC_LIQUIDITY_EVENTS` | yes | Input liquidity topic |
| `TOPIC_TRADING_ANALYTICS` | yes | Output trading topic base name |
| `TOPIC_LIQUIDITY_ANALYTICS` | yes | Output liquidity topic |
| `TOPIC_PATTERN_ANALYTICS` | no | Output MEV topic (default `dex-pattern-analytics`) |
| `TOPIC_MARKET_TRENDS` | no | Output trends topic (default `dex-market-trends`) |
| `FLINK_CONSUMER_GROUP` | yes | Kafka consumer group |
| `FLINK_PARALLELISM` | yes | Job parallelism |
| `FLINK_CHECKPOINT_MS` | yes | Checkpoint interval |
| `TRADING_WINDOW_MINUTES` | no | Comma-separated window sizes (default `5`) |
| `LIQUIDITY_WINDOW_MINUTES` | no | Liquidity window size (default `60`) |
| `SESSION_GAP_SECONDS` | no | MEV session gap (default `3`) |
| `TREND_WINDOW_MINUTES` | no | Trend window size (default `30`) |
| `TREND_SLIDE_MINUTES` | no | Trend slide interval (default `5`) |

## Build & Test

```bash
mvn test          # 55 tests
mvn -DskipTests package
```

## Run

```bash
flink run target/aggregator-1.0-SNAPSHOT.jar
```
