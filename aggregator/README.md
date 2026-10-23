# Aggregator

Apache Flink 2 job (Java 21) consuming DEX events from Kafka and producing four analytics streams.

## Requirements

- Java 21
- Maven
- Kafka broker reachable from Flink runtime

Kafka connection, topics, and job tuning are all env vars; see
[`FlinkConfig.fromEnv`](src/main/java/com/web3analytics/config/FlinkConfig.java) for the
full list, or root [ARCHITECTURE.md](../ARCHITECTURE.md#event-to-analytics-mapping) for the
topic/pipeline contract. Defaults for the default stack are in `docker-compose.yml`.

## Build & Test

```bash
mvn test
mvn -DskipTests package
```

## Run

```bash
flink run target/aggregator-1.0-SNAPSHOT.jar
```
