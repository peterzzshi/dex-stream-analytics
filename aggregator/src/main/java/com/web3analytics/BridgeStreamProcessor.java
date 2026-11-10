package com.web3analytics;

import com.web3analytics.bridge.BridgeSupplyEvent;
import com.web3analytics.bridge.BridgeTransferEvent;
import com.web3analytics.config.BridgeStreamProcessorConfig;
import com.web3analytics.functions.CrossChainFlowAggregateFunction;
import com.web3analytics.functions.ExploitDetectionAggregateFunction;
import com.web3analytics.functions.ReentrancyDetectionAggregateFunction;
import com.web3analytics.functions.SupplyTrackingAggregateFunction;
import com.web3analytics.functions.TransferVolumeAggregateFunction;
import com.web3analytics.model.BridgeActivity;
import com.web3analytics.model.EventKind;
import com.web3analytics.model.WindowStamped;
import com.web3analytics.serde.BridgeEventDeserializer;
import com.web3analytics.sinks.RestApiSink;
import java.io.Serial;
import java.time.Duration;
import java.util.function.ToLongFunction;
import org.apache.avro.specific.SpecificRecord;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.configuration.Configuration;
import org.apache.flink.configuration.RestartStrategyOptions;
import org.apache.flink.connector.kafka.source.KafkaSource;
import org.apache.flink.connector.kafka.source.enumerator.initializer.OffsetsInitializer;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.assigners.EventTimeSessionWindows;
import org.apache.flink.streaming.api.windowing.assigners.TumblingEventTimeWindows;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class BridgeStreamProcessor {

  private static final Logger LOG = LoggerFactory.getLogger(BridgeStreamProcessor.class);

  public static void main(final String[] args) throws Exception {
    final BridgeStreamProcessorConfig config = BridgeStreamProcessorConfig.fromEnv();
    final StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();

    // A failed HTTP post must be retried, and a restart only avoids replaying the topic from the
    // beginning if offsets are checkpointed. Bounded attempts keep a permanently failing sink
    // visible as a failed job instead of an endless restart loop.
    env.enableCheckpointing(Duration.ofSeconds(config.checkpointIntervalSeconds()).toMillis());
    final Configuration restartConf = new Configuration();
    restartConf.set(RestartStrategyOptions.RESTART_STRATEGY, "fixed-delay");
    restartConf.set(RestartStrategyOptions.RESTART_STRATEGY_FIXED_DELAY_ATTEMPTS,
        config.restartMaxAttempts());
    restartConf.set(RestartStrategyOptions.RESTART_STRATEGY_FIXED_DELAY_DELAY,
        Duration.ofSeconds(config.restartDelaySeconds()));
    env.configure(restartConf);

    // One stream per topic. Lock and unlock share a topic and a record type, so they arrive on one
    // stream and are separated by the `direction` field; the same holds for mint and burn.
    final DataStream<BridgeTransferEvent> transferStream = createBridgeStream(env, config,
        config.bridgeTransferTopic(), "bridge-transfer-consumer", BridgeTransferEvent.class,
        BridgeTransferEvent::getBlockTimestamp, "Bridge Transfer Events");
    final DataStream<BridgeSupplyEvent> supplyStream = createBridgeStream(env, config,
        config.bridgeSupplyTopic(), "bridge-supply-consumer", BridgeSupplyEvent.class,
        BridgeSupplyEvent::getBlockTimestamp, "Bridge Supply Events");

    // Built once: both activity pipelines consume the same normalized stream, so the two
    // amount conversions happen per event once rather than once per consuming pipeline.
    final DataStream<BridgeActivity> activityStream =
        unionBridgeActivity(transferStream, supplyStream);

    buildTransferVolumePipeline(transferStream, config);
    buildSupplyTrackingPipeline(supplyStream, config);
    buildCrossChainFlowPipeline(activityStream, config);
    buildExploitDetectionPipeline(activityStream, config);
    buildReentrancyDetectionPipeline(transferStream, config);

    LOG.info("Starting Bridge Stream Processor with 5 pipelines");
    env.execute("Bridge Security Monitor");
  }

  private static <T extends SpecificRecord> DataStream<T> createBridgeStream(
      final StreamExecutionEnvironment env,
      final BridgeStreamProcessorConfig config,
      final String topic,
      final String groupId,
      final Class<T> eventClass,
      final ToLongFunction<T> timestampFn,
      final String sourceName) {
    final DeserializationSchema<T> deserializer = new BridgeEventDeserializer<>(eventClass);
    final KafkaSource<T> source = KafkaSource.<T>builder()
        .setBootstrapServers(config.kafkaBootstrapServers())
        .setTopics(topic)
        .setGroupId(groupId)
        .setStartingOffsets(OffsetsInitializer.earliest())
        .setValueOnlyDeserializer(deserializer)
        .build();
    final WatermarkStrategy<T> watermarkStrategy = WatermarkStrategy
        .<T>forBoundedOutOfOrderness(Duration.ofSeconds(config.watermarkDelaySeconds()))
        .withTimestampAssigner((event, timestamp) -> timestampFn.applyAsLong(event));
    return env.fromSource(source, watermarkStrategy, sourceName);
  }

  private static void buildTransferVolumePipeline(
      final DataStream<BridgeTransferEvent> transferStream,
      final BridgeStreamProcessorConfig config) {
    transferStream
        .keyBy(BridgeStreamProcessor::bridgeChainKey)
        .window(TumblingEventTimeWindows.of(Duration.ofSeconds(config.transferWindowSeconds())))
        .aggregate(new TransferVolumeAggregateFunction(), new WindowStamp<>())
        .sinkTo(new RestApiSink<>(config.transferVolumeEndpoint()))
        .name("Transfer Volume REST Sink");
  }

  private static void buildSupplyTrackingPipeline(
      final DataStream<BridgeSupplyEvent> supplyStream,
      final BridgeStreamProcessorConfig config) {
    supplyStream
        .keyBy(BridgeStreamProcessor::bridgeChainKey)
        .window(TumblingEventTimeWindows.of(Duration.ofSeconds(config.supplyWindowSeconds())))
        .aggregate(new SupplyTrackingAggregateFunction(), new WindowStamp<>())
        .sinkTo(new RestApiSink<>(config.supplyTrackingEndpoint()))
        .name("Supply Tracking REST Sink");
  }

  private static void buildCrossChainFlowPipeline(
      final DataStream<BridgeActivity> activityStream,
      final BridgeStreamProcessorConfig config) {
    activityStream
        .keyBy(BridgeActivity::bridgeAddress)
        .window(TumblingEventTimeWindows.of(Duration.ofSeconds(config.flowWindowSeconds())))
        .aggregate(new CrossChainFlowAggregateFunction(), new WindowStamp<>())
        .sinkTo(new RestApiSink<>(config.crossChainFlowEndpoint()))
        .name("Cross-Chain Flow REST Sink");
  }

  private static void buildExploitDetectionPipeline(
      final DataStream<BridgeActivity> activityStream,
      final BridgeStreamProcessorConfig config) {
    activityStream
        .keyBy(activity -> activity.bridgeAddress() + ":" + activity.chain())
        .window(EventTimeSessionWindows.withGap(
            Duration.ofSeconds(config.exploitSessionGapSeconds())))
        .aggregate(new ExploitDetectionAggregateFunction(), new WindowStamp<>())
        .filter(acc -> !acc.suspiciousPatterns.isEmpty())
        .sinkTo(new RestApiSink<>(config.exploitDetectionEndpoint()))
        .name("Exploit Detection REST Sink");
  }

  /**
   * Keyed by transaction hash rather than bridge address: reentrancy is a property of a single
   * transaction making multiple lock/unlock calls, not of bridge-level volume over a window.
   */
  private static void buildReentrancyDetectionPipeline(
      final DataStream<BridgeTransferEvent> transferStream,
      final BridgeStreamProcessorConfig config) {
    transferStream
        .keyBy(BridgeTransferEvent::getTransactionHash)
        .window(TumblingEventTimeWindows.of(
            Duration.ofSeconds(config.reentrancyWindowSeconds())))
        .aggregate(new ReentrancyDetectionAggregateFunction(), new WindowStamp<>())
        .filter(acc -> acc.lockCount > 1 || acc.unlockCount > 1)
        .sinkTo(new RestApiSink<>(config.reentrancyDetectionEndpoint()))
        .name("Reentrancy Detection REST Sink");
  }

  /**
   * Unions the two event streams into one {@code BridgeActivity}-typed stream, tagging each element
   * with its {@code EventKind} so downstream pipelines branch on an explicit discriminant instead
   * of null-checking a positional tuple slot. The two topics carry structurally different records
   * (22 and 19 fields), so the union is a real normalization, not a duplicate registration.
   */
  private static DataStream<BridgeActivity> unionBridgeActivity(
      final DataStream<BridgeTransferEvent> transferStream,
      final DataStream<BridgeSupplyEvent> supplyStream) {
    return transferStream
        .map(e -> new BridgeActivity(
            e.getDirection() == com.web3analytics.bridge.TransferDirection.lock
                ? EventKind.LOCK
                : EventKind.UNLOCK,
            e.getBridgeAddress(),
            e.getChain(),
            e.getProtocol().toString(),
            e.getAmount().unscaledValue()))
        .union(supplyStream.map(e -> new BridgeActivity(
            e.getOperation() == com.web3analytics.bridge.SupplyOperation.mint
                ? EventKind.MINT
                : EventKind.BURN,
            e.getBridgeAddress(),
            e.getChain(),
            e.getProtocol().toString(),
            e.getAmount().unscaledValue())));
  }

  private static String bridgeChainKey(final BridgeTransferEvent event) {
    return event.getBridgeAddress() + ":" + event.getChain();
  }

  private static String bridgeChainKey(final BridgeSupplyEvent event) {
    return event.getBridgeAddress() + ":" + event.getChain();
  }

  /**
   * Stamps each aggregate with the end timestamp of the window that produced it, so sinks report
   * when a result is from instead of {@code windowEnd = 0}. A window function is required because
   * {@code AggregateFunction} never sees the window itself.
   */
  private static class WindowStamp<T extends WindowStamped>
      extends ProcessWindowFunction<T, T, String, TimeWindow> {

    @Serial
    private static final long serialVersionUID = 1L;

    @Override
    public void process(
        final String key,
        final Context context,
        final Iterable<T> elements,
        final Collector<T> out) {
      final long windowEnd = context.window().getEnd();
      for (final T element : elements) {
        element.setWindowEnd(windowEnd);
        out.collect(element);
      }
    }
  }
}
