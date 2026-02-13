# Aggregator

Apache Flink job that aggregates swap events into analytics windows and writes the results back to Kafka.

## Requirements

- Java 17
- Maven
- Flink 2.x
- Kafka and Schema Registry endpoints

## Build

```bash
cd aggregator
mvn -DskipTests package
```

## Run

```bash
flink run target/aggregator-1.0-SNAPSHOT.jar
```

## Configuration

- `KAFKA_BOOTSTRAP`: Kafka bootstrap servers (default: `kafka:9092`)
- `TOPIC_DEX_EVENTS`: Input topic for swap events (default: `dex-events`)
- `TOPIC_DEX_ANALYTICS`: Output topic for aggregated analytics (default: `dex-analytics`)
- `FLINK_CONSUMER_GROUP`: Kafka consumer group (default: `dex-processor`)
- `FLINK_PARALLELISM`: Job parallelism (default: `2`)
- `FLINK_CHECKPOINT_MS`: Checkpoint interval in milliseconds (default: `10000`)
- `SCHEMA_REGISTRY_URL`: Schema Registry endpoint (default: `http://schema-registry:8081`)
