# Aggregator

Apache Flink 2 job (Java 21) consuming bridge events from Kafka and producing five analytics streams.

## Requirements

- Java 21
- Maven
- Kafka broker reachable from Flink runtime

Every value the job reads is a required env var with no fallback — a missing or malformed one
fails the job at launch. See
[`BridgeStreamProcessorConfig.fromEnv`](src/main/java/com/web3analytics/config/BridgeStreamProcessorConfig.java)
for the full list, or root [ARCHITECTURE.md](../ARCHITECTURE.md#event-to-analytics-mapping) for the
topic/pipeline contract. Values for the default stack are in `docker-compose.yml`.

## Build & Test

```bash
mvn test
mvn -DskipTests package
```

## Run

```bash
flink run target/aggregator-1.0-SNAPSHOT.jar
```
