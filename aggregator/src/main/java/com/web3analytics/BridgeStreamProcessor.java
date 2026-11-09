package com.web3analytics;

import com.web3analytics.bridge.BridgeBurnEvent;
import com.web3analytics.bridge.BridgeLockEvent;
import com.web3analytics.bridge.BridgeMintEvent;
import com.web3analytics.bridge.BridgeUnlockEvent;
import com.web3analytics.functions.bridge.CrossChainFlowAggregateFunction;
import com.web3analytics.functions.bridge.ExploitDetectionAggregateFunction;
import com.web3analytics.functions.bridge.SupplyTrackingAggregateFunction;
import com.web3analytics.functions.bridge.TransferVolumeAggregateFunction;
import com.web3analytics.serde.BridgeBurnEventDeserializer;
import com.web3analytics.serde.BridgeLockEventDeserializer;
import com.web3analytics.serde.BridgeMintEventDeserializer;
import com.web3analytics.serde.BridgeUnlockEventDeserializer;
import com.web3analytics.sinks.RestApiSink;
import java.time.Duration;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.api.java.tuple.Tuple2;
import org.apache.flink.api.java.tuple.Tuple4;
import org.apache.flink.connector.kafka.source.KafkaSource;
import org.apache.flink.connector.kafka.source.enumerator.initializer.OffsetsInitializer;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.windowing.assigners.EventTimeSessionWindows;
import org.apache.flink.streaming.api.windowing.assigners.TumblingEventTimeWindows;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class BridgeStreamProcessor {

  private static final Logger LOG = LoggerFactory.getLogger(BridgeStreamProcessor.class);

  private static final String KAFKA_BOOTSTRAP_SERVERS = getEnvOrFail("KAFKA_BOOTSTRAP_SERVERS");
  private static final String BRIDGE_TRANSFER_TOPIC = getEnvOrFail("BRIDGE_TRANSFER_TOPIC");
  private static final String BRIDGE_SUPPLY_TOPIC = getEnvOrFail("BRIDGE_SUPPLY_TOPIC");
  private static final String TRANSFER_VOLUME_ENDPOINT = getEnvOrFail("TRANSFER_VOLUME_ENDPOINT");
  private static final String SUPPLY_TRACKING_ENDPOINT = getEnvOrFail("SUPPLY_TRACKING_ENDPOINT");
  private static final String CROSS_CHAIN_FLOW_ENDPOINT = getEnvOrFail("CROSS_CHAIN_FLOW_ENDPOINT");
  private static final String EXPLOIT_DETECTION_ENDPOINT = getEnvOrFail(
      "EXPLOIT_DETECTION_ENDPOINT");

  private static final int WATERMARK_DELAY_SECONDS = getEnvAsInt("WATERMARK_DELAY_SECONDS", 5);
  private static final int TRANSFER_WINDOW_SECONDS = getEnvAsInt("TRANSFER_WINDOW_SECONDS", 300);
  private static final int SUPPLY_WINDOW_SECONDS = getEnvAsInt("SUPPLY_WINDOW_SECONDS", 300);
  private static final int FLOW_WINDOW_SECONDS = getEnvAsInt("FLOW_WINDOW_SECONDS", 300);
  private static final int EXPLOIT_SESSION_GAP_SECONDS = getEnvAsInt("EXPLOIT_SESSION_GAP_SECONDS",
      60);

  private static String getEnvOrFail(final String key) {
    final String value = System.getenv(key);
    if (value == null || value.isBlank()) {
      throw new IllegalStateException("Required environment variable not set: " + key);
    }
    return value;
  }

  private static int getEnvAsInt(final String key, final int defaultValue) {
    final String value = System.getenv(key);
    if (value == null || value.isBlank()) {
      return defaultValue;
    }
    try {
      return Integer.parseInt(value);
    } catch (final NumberFormatException e) {
      throw new IllegalStateException(
          "Invalid integer for environment variable " + key + ": " + value, e);
    }
  }

  public static void main(final String[] args) throws Exception {
    final StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();

    final DataStream<BridgeLockEvent> lockStream = createLockEventStream(env);
    final DataStream<BridgeUnlockEvent> unlockStream = createUnlockEventStream(env);
    final DataStream<BridgeMintEvent> mintStream = createMintEventStream(env);
    final DataStream<BridgeBurnEvent> burnStream = createBurnEventStream(env);

    buildTransferVolumePipeline(lockStream, unlockStream);
    buildSupplyTrackingPipeline(mintStream, burnStream);
    buildCrossChainFlowPipeline(lockStream, unlockStream, mintStream, burnStream);
    buildExploitDetectionPipeline(lockStream, unlockStream, mintStream, burnStream);

    LOG.info("Starting Bridge Stream Processor with 4 pipelines");
    env.execute("Bridge Security Monitor");
  }

  private static DataStream<BridgeLockEvent> createLockEventStream(
      final StreamExecutionEnvironment env) {
    final KafkaSource<BridgeLockEvent> source = KafkaSource.<BridgeLockEvent>builder()
        .setBootstrapServers(KAFKA_BOOTSTRAP_SERVERS)
        .setTopics(BRIDGE_TRANSFER_TOPIC)
        .setGroupId("bridge-lock-consumer")
        .setStartingOffsets(OffsetsInitializer.earliest())
        .setValueOnlyDeserializer(new BridgeLockEventDeserializer())
        .build();

    final WatermarkStrategy<BridgeLockEvent> watermarkStrategy = WatermarkStrategy
        .<BridgeLockEvent>forBoundedOutOfOrderness(Duration.ofSeconds(WATERMARK_DELAY_SECONDS))
        .withTimestampAssigner((event, timestamp) -> event.getBlockTimestamp());

    return env.fromSource(source, watermarkStrategy, "Bridge Lock Events");
  }

  private static DataStream<BridgeUnlockEvent> createUnlockEventStream(
      final StreamExecutionEnvironment env) {
    final KafkaSource<BridgeUnlockEvent> source = KafkaSource.<BridgeUnlockEvent>builder()
        .setBootstrapServers(KAFKA_BOOTSTRAP_SERVERS)
        .setTopics(BRIDGE_TRANSFER_TOPIC)
        .setGroupId("bridge-unlock-consumer")
        .setStartingOffsets(OffsetsInitializer.earliest())
        .setValueOnlyDeserializer(new BridgeUnlockEventDeserializer())
        .build();

    final WatermarkStrategy<BridgeUnlockEvent> watermarkStrategy = WatermarkStrategy
        .<BridgeUnlockEvent>forBoundedOutOfOrderness(Duration.ofSeconds(WATERMARK_DELAY_SECONDS))
        .withTimestampAssigner((event, timestamp) -> event.getBlockTimestamp());

    return env.fromSource(source, watermarkStrategy, "Bridge Unlock Events");
  }

  private static DataStream<BridgeMintEvent> createMintEventStream(
      final StreamExecutionEnvironment env) {
    final KafkaSource<BridgeMintEvent> source = KafkaSource.<BridgeMintEvent>builder()
        .setBootstrapServers(KAFKA_BOOTSTRAP_SERVERS)
        .setTopics(BRIDGE_SUPPLY_TOPIC)
        .setGroupId("bridge-mint-consumer")
        .setStartingOffsets(OffsetsInitializer.earliest())
        .setValueOnlyDeserializer(new BridgeMintEventDeserializer())
        .build();

    final WatermarkStrategy<BridgeMintEvent> watermarkStrategy = WatermarkStrategy
        .<BridgeMintEvent>forBoundedOutOfOrderness(Duration.ofSeconds(WATERMARK_DELAY_SECONDS))
        .withTimestampAssigner((event, timestamp) -> event.getBlockTimestamp());

    return env.fromSource(source, watermarkStrategy, "Bridge Mint Events");
  }

  private static DataStream<BridgeBurnEvent> createBurnEventStream(
      final StreamExecutionEnvironment env) {
    final KafkaSource<BridgeBurnEvent> source = KafkaSource.<BridgeBurnEvent>builder()
        .setBootstrapServers(KAFKA_BOOTSTRAP_SERVERS)
        .setTopics(BRIDGE_SUPPLY_TOPIC)
        .setGroupId("bridge-burn-consumer")
        .setStartingOffsets(OffsetsInitializer.earliest())
        .setValueOnlyDeserializer(new BridgeBurnEventDeserializer())
        .build();

    final WatermarkStrategy<BridgeBurnEvent> watermarkStrategy = WatermarkStrategy
        .<BridgeBurnEvent>forBoundedOutOfOrderness(Duration.ofSeconds(WATERMARK_DELAY_SECONDS))
        .withTimestampAssigner((event, timestamp) -> event.getBlockTimestamp());

    return env.fromSource(source, watermarkStrategy, "Bridge Burn Events");
  }

  private static void buildTransferVolumePipeline(
      final DataStream<BridgeLockEvent> lockStream,
      final DataStream<BridgeUnlockEvent> unlockStream) {
    final DataStream<Tuple2<BridgeLockEvent, BridgeUnlockEvent>> transferUnion = lockStream
        .map(event -> Tuple2.of(event, (BridgeUnlockEvent) null))
        .union(unlockStream.map(event -> Tuple2.of(null, event)));

    transferUnion
        .keyBy(tuple -> tuple.f0 != null
            ? tuple.f0.getBridgeAddress() + ":" + tuple.f0.getChain()
            : tuple.f1.getBridgeAddress() + ":" + tuple.f1.getChain())
        .window(TumblingEventTimeWindows.of(Duration.ofSeconds(TRANSFER_WINDOW_SECONDS)))
        .aggregate(new TransferVolumeAggregateFunction())
        .sinkTo(new RestApiSink<>(TRANSFER_VOLUME_ENDPOINT))
        .name("Transfer Volume REST Sink");
  }

  private static void buildSupplyTrackingPipeline(
      final DataStream<BridgeMintEvent> mintStream,
      final DataStream<BridgeBurnEvent> burnStream) {
    final DataStream<Tuple2<BridgeMintEvent, BridgeBurnEvent>> supplyUnion = mintStream
        .map(event -> Tuple2.of(event, (BridgeBurnEvent) null))
        .union(burnStream.map(event -> Tuple2.of(null, event)));

    supplyUnion
        .keyBy(tuple -> tuple.f0 != null
            ? tuple.f0.getBridgeAddress() + ":" + tuple.f0.getChain()
            : tuple.f1.getBridgeAddress() + ":" + tuple.f1.getChain())
        .window(TumblingEventTimeWindows.of(Duration.ofSeconds(SUPPLY_WINDOW_SECONDS)))
        .aggregate(new SupplyTrackingAggregateFunction())
        .sinkTo(new RestApiSink<>(SUPPLY_TRACKING_ENDPOINT))
        .name("Supply Tracking REST Sink");
  }

  private static void buildCrossChainFlowPipeline(
      final DataStream<BridgeLockEvent> lockStream,
      final DataStream<BridgeUnlockEvent> unlockStream,
      final DataStream<BridgeMintEvent> mintStream,
      final DataStream<BridgeBurnEvent> burnStream) {
    final DataStream<Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent>> flowUnion =
        createTuple4Union(lockStream, unlockStream, mintStream, burnStream);

    flowUnion
        .keyBy(BridgeStreamProcessor::extractBridgeAddress)
        .window(TumblingEventTimeWindows.of(Duration.ofSeconds(FLOW_WINDOW_SECONDS)))
        .aggregate(new CrossChainFlowAggregateFunction())
        .sinkTo(new RestApiSink<>(CROSS_CHAIN_FLOW_ENDPOINT))
        .name("Cross-Chain Flow REST Sink");
  }

  private static void buildExploitDetectionPipeline(
      final DataStream<BridgeLockEvent> lockStream,
      final DataStream<BridgeUnlockEvent> unlockStream,
      final DataStream<BridgeMintEvent> mintStream,
      final DataStream<BridgeBurnEvent> burnStream) {
    final DataStream<Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent>> flowUnion =
        createTuple4Union(lockStream, unlockStream, mintStream, burnStream);

    flowUnion
        .keyBy(BridgeStreamProcessor::extractBridgeAddressWithChain)
        .window(EventTimeSessionWindows.withGap(Duration.ofSeconds(EXPLOIT_SESSION_GAP_SECONDS)))
        .aggregate(new ExploitDetectionAggregateFunction())
        .filter(acc -> !acc.suspiciousPatterns.isEmpty())
        .sinkTo(new RestApiSink<>(EXPLOIT_DETECTION_ENDPOINT))
        .name("Exploit Detection REST Sink");
  }

  private static DataStream<Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent>> createTuple4Union(
      final DataStream<BridgeLockEvent> lockStream,
      final DataStream<BridgeUnlockEvent> unlockStream,
      final DataStream<BridgeMintEvent> mintStream,
      final DataStream<BridgeBurnEvent> burnStream) {
    return lockStream
        .map(event -> Tuple4.of(event, (BridgeUnlockEvent) null, (BridgeMintEvent) null,
            (BridgeBurnEvent) null))
        .union(unlockStream.map(event -> Tuple4.of(null, event,
            null, null)))
        .union(mintStream.map(event -> Tuple4.of(null,
            null, event, null)))
        .union(burnStream.map(event -> Tuple4.of(null,
            null, null, event)));
  }

  private static String extractBridgeAddress(
      final Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent> tuple) {
    if (tuple.f0 != null) {
      return tuple.f0.getBridgeAddress();
    }
    if (tuple.f1 != null) {
      return tuple.f1.getBridgeAddress();
    }
    if (tuple.f2 != null) {
      return tuple.f2.getBridgeAddress();
    }
    return tuple.f3.getBridgeAddress();
  }

  private static String extractBridgeAddressWithChain(
      final Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent> tuple) {
    if (tuple.f0 != null) {
      return tuple.f0.getBridgeAddress() + ":" + tuple.f0.getChain();
    }
    if (tuple.f1 != null) {
      return tuple.f1.getBridgeAddress() + ":" + tuple.f1.getChain();
    }
    if (tuple.f2 != null) {
      return tuple.f2.getBridgeAddress() + ":" + tuple.f2.getChain();
    }
    return tuple.f3.getBridgeAddress() + ":" + tuple.f3.getChain();
  }
}
