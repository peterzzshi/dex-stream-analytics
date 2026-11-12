# Aggregator

Apache Flink 2 job (Java 21) consuming bridge events from Kafka and producing five alert streams.

`com.web3analytics.BridgeStreamProcessor` builds five pipelines over the two ingest topics and sinks
each one to its own Kafka topic. See root [ARCHITECTURE.md](../ARCHITECTURE.md#event-to-alert-mapping)
for the event-to-alert contract.

## Requirements

- Java 21
- Maven
- Kafka broker reachable from the Flink runtime

Every value the job reads is a required env var with no fallback — a missing or malformed one fails
the job at launch with the variable name in the message. The full list lives in
[`BridgeStreamProcessorConfig.fromEnv`](src/main/java/com/web3analytics/config/BridgeStreamProcessorConfig.java);
concrete values for the default stack are in `docker-compose.yml`.

## Build & Test

```bash
mvn test
mvn -DskipTests package
```

## Run

The compose stack runs the job in standalone application mode:

```bash
docker compose up -d flink-jobmanager flink-taskmanager
# jobmanager command: standalone-job --job-classname com.web3analytics.BridgeStreamProcessor
```

Inside the Flink image with the jar on the classpath, the equivalent is:

```bash
flink run -c com.web3analytics.BridgeStreamProcessor \
  /opt/flink/usrlib/aggregator.jar
```

`flink run target/aggregator-1.0-SNAPSHOT.jar` alone will not work: the shaded jar declares a
nonexistent main class (`com.web3analytics.StreamProcessor`), so the class must be named explicitly.
Tracked in [`../GAPS.md`](../GAPS.md).

Flink UI: <http://localhost:8081>. One job, five sink vertices.

## Configuration

| Variable | Purpose |
|---|---|
| `KAFKA_BOOTSTRAP_SERVERS` | broker list |
| `BRIDGE_TRANSFER_TOPIC`, `BRIDGE_SUPPLY_TOPIC` | the two ingest topics |
| `WATERMARK_DELAY_SECONDS` | bounded out-of-orderness |
| `TRANSFER_WINDOW_SECONDS`, `SUPPLY_WINDOW_SECONDS`, `FLOW_WINDOW_SECONDS` | tumbling windows |
| `EXPLOIT_SESSION_GAP_SECONDS` | event-time session gap |
| `REENTRANCY_WINDOW_SECONDS` | tumbling window keyed by transaction |
| `RESTART_MAX_ATTEMPTS`, `RESTART_DELAY_SECONDS` | bounded restart strategy |
| `CHECKPOINT_INTERVAL_SECONDS` | checkpoint period |

Alert topic names are constants in `AlertTopics`, not env vars.

## Sink Semantics

Avro binary via `AvroSerializationSchema.forSpecific`, delivery guarantee `AT_LEAST_ONCE`. A restart
without a checkpoint replays the topic from earliest, so duplicates are expected and the alerting
service deduplicates on `(alertType, bridgeAddress, windowStart, windowEnd)`.
