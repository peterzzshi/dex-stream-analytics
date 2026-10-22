package com.web3analytics;

import com.web3analytics.config.FlinkConfig;
import com.web3analytics.errors.DecodingError;
import com.web3analytics.functions.LiquidityWindowFunction;
import com.web3analytics.functions.MarketTrendWindowFunction;
import com.web3analytics.functions.MevDetectionFunction;
import com.web3analytics.functions.SwapAggregator;
import com.web3analytics.functions.SwapAnalyticsWindowFunction;
import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.LiquidityAnalytics;
import com.web3analytics.models.MarketTrend;
import com.web3analytics.models.MevAlert;
import com.web3analytics.models.SwapEvent;
import com.web3analytics.serde.ByteArrayDeserializationSchema;
import com.web3analytics.serde.JsonSerializationSchema;
import com.web3analytics.serde.LiquidityEventDeserializer;
import com.web3analytics.serde.SafeDecodeProcessFunction;
import com.web3analytics.serde.SwapEventDeserializer;
import java.io.Serial;
import java.time.Duration;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.apache.flink.connector.base.DeliveryGuarantee;
import org.apache.flink.connector.kafka.sink.KafkaRecordSerializationSchema;
import org.apache.flink.connector.kafka.sink.KafkaSink;
import org.apache.flink.connector.kafka.source.KafkaSource;
import org.apache.flink.connector.kafka.source.enumerator.initializer.OffsetsInitializer;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.datastream.SingleOutputStreamOperator;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.windowing.assigners.EventTimeSessionWindows;
import org.apache.flink.streaming.api.windowing.assigners.SlidingEventTimeWindows;
import org.apache.flink.streaming.api.windowing.assigners.TumblingEventTimeWindows;
import org.apache.flink.util.OutputTag;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Multi-window Flink processor demonstrating four window strategies:
 *
 * <ul>
 *   <li><b>Tumbling</b>: SwapEvent → trading analytics (TWAP, OHLC, volume)</li>
 *   <li><b>Tumbling</b>: Mint/Burn/Transfer → liquidity analytics</li>
 *   <li><b>Session</b>: All events → MEV detection (sandwich attacks, JIT liquidity)</li>
 *   <li><b>Sliding</b>: SwapEvent → market trends</li>
 * </ul>
 *
 * Decode and watermark assignment happen once per source; window pipelines fan out
 * from the shared watermarked streams.
 * Uses native Kafka connector (not DAPR) for exactly-once semantics.
 */
public class StreamProcessor {

    private static final Logger LOG = LoggerFactory.getLogger(StreamProcessor.class);

  public static void main(final String[] args) throws Exception {
    final FlinkConfig config = FlinkConfig.fromEnv();
        LOG.info("Starting DEX Analytics Stream Processor");
    LOG.info("Trading: {} -> {} (tumbling={}m)", config.topicTradingEvents(),
        config.topicTradingAnalytics(), config.tradingWindowMinutes());
    LOG.info("Liquidity: {} -> {} (tumbling={}m)", config.topicLiquidityEvents(),
        config.topicLiquidityAnalytics(), config.liquidityWindowMinutes());
    LOG.info("MEV detection: session gap={}s -> {}", config.sessionGapSeconds(),
        config.topicPatternAnalytics());
    LOG.info("Market trends: sliding={}m/{}m -> {}", config.trendWindowMinutes(),
        config.trendSlideMinutes(),
                config.topicMarketTrends());

    final StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.setParallelism(config.parallelism());
        env.enableCheckpointing(config.checkpointMs());

        // Decode once, fan out to all pipelines
    final DataStream<SwapEvent> tradingStream = buildWatermarkedTradingStream(env, config);
    final DataStream<DexEvent> liquidityStream = buildWatermarkedLiquidityStream(env, config);

    // 1. Tumbling window: Trading analytics
    buildTradingWindowPipeline(tradingStream, config);

        // 2. Tumbling window: Liquidity analytics
        buildLiquidityPipeline(liquidityStream, config);

        // 3. Session windows: MEV detection (union of both streams)
        buildMevDetectionStream(tradingStream, liquidityStream, config);

        // 4. Sliding windows: Market trends
        buildMarketTrendStream(tradingStream, config);

        env.execute("DEX Analytics Processor");
    }

    // ── Source decode + watermark (shared) ────────────────────────────

    private static DataStream<SwapEvent> buildWatermarkedTradingStream(
        final StreamExecutionEnvironment env, final FlinkConfig config
    ) {
      final KafkaSource<byte[]> source = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicTradingEvents())
                .setGroupId(config.consumerGroup() + "-trading")
                .setStartingOffsets(OffsetsInitializer.earliest())
            .setValueOnlyDeserializer(new ByteArrayDeserializationSchema())
                .build();

      final OutputTag<DecodingError> decodeErrors = new OutputTag<>("trading-decode-errors") {
        @Serial
        private static final long serialVersionUID = 7775045071686846421L;
      };
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();

      final SingleOutputStreamOperator<SwapEvent> decoded = env
                .fromSource(source, WatermarkStrategy.noWatermarks(), "trading-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicTradingEvents(),
                        deserializer::deserialize,
                        decodeErrors
                ))
                .returns(TypeInformation.of(SwapEvent.class))
                .name("decode-trading-events");

        decoded.getSideOutput(decodeErrors).print("trading-decode-errors");

      final WatermarkStrategy<SwapEvent> watermarks = WatermarkStrategy
                .<SwapEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        return decoded.assignTimestampsAndWatermarks(watermarks);
    }

    private static DataStream<DexEvent> buildWatermarkedLiquidityStream(
        final StreamExecutionEnvironment env, final FlinkConfig config
    ) {
      final KafkaSource<byte[]> source = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicLiquidityEvents())
                .setGroupId(config.consumerGroup() + "-liquidity")
                .setStartingOffsets(OffsetsInitializer.earliest())
            .setValueOnlyDeserializer(new ByteArrayDeserializationSchema())
                .build();

      final OutputTag<DecodingError> decodeErrors = new OutputTag<>("liquidity-decode-errors") {
        @Serial
        private static final long serialVersionUID = 2377963851791726070L;
      };
      final LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();

      final SingleOutputStreamOperator<DexEvent> decoded = env
                .fromSource(source, WatermarkStrategy.noWatermarks(), "liquidity-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicLiquidityEvents(),
                        deserializer::deserialize,
                        decodeErrors
                ))
                .returns(TypeInformation.of(DexEvent.class))
                .name("decode-liquidity-events");

        decoded.getSideOutput(decodeErrors).print("liquidity-decode-errors");

      final WatermarkStrategy<DexEvent> watermarks = WatermarkStrategy
                .<DexEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        return decoded.assignTimestampsAndWatermarks(watermarks);
    }

  // ── Pipeline 1: Trading Analytics (tumbling window) ──────────────

  private static void buildTradingWindowPipeline(final DataStream<SwapEvent> watermarked,
      final FlinkConfig config) {
    final DataStream<AggregatedAnalytics> analytics = watermarked
                .keyBy(event -> event.chain() + ":" + event.pairAddress())
        .window(
            TumblingEventTimeWindows.of(Duration.ofMinutes(config.tradingWindowMinutes())))
                .aggregate(new SwapAggregator(), new SwapAnalyticsWindowFunction())
        .name("trading-analytics");

    analytics.print("trading-analytics");
    analytics.sinkTo(
        buildKafkaSink(config.kafkaBootstrap(), config.topicTradingAnalytics(), "trading"));
    }

    // ── Pipeline 2: Liquidity Analytics (tumbling window) ────────────

  private static void buildLiquidityPipeline(final DataStream<DexEvent> liquidityStream,
      final FlinkConfig config) {
    final DataStream<LiquidityAnalytics> analytics = liquidityStream
                .keyBy(event -> event.chain() + ":" + event.pairAddress())
        .window(
            TumblingEventTimeWindows.of(Duration.ofMinutes(config.liquidityWindowMinutes())))
                .process(new LiquidityWindowFunction());

        analytics.print("liquidity-analytics");
        analytics.sinkTo(buildKafkaSink(config.kafkaBootstrap(), config.topicLiquidityAnalytics(), "liquidity"));
    }

    // ── Pipeline 3: MEV Detection (session windows) ──────────────────

    private static void buildMevDetectionStream(
        final DataStream<SwapEvent> tradingStream,
        final DataStream<DexEvent> liquidityStream,
        final FlinkConfig config
    ) {
      // Cast SwapEvent stream to DexEvent for union compatibility
      final DataStream<DexEvent> tradingAsEvents = tradingStream.map(swap -> swap);
      final DataStream<DexEvent> allEvents = tradingAsEvents.union(liquidityStream);

      final DataStream<MevAlert> alerts = allEvents
                .keyBy(event -> event.chain() + ":" + event.pairAddress())
          .window(EventTimeSessionWindows.withGap(Duration.ofSeconds(config.sessionGapSeconds())))
                .process(new MevDetectionFunction())
                .name("mev-detection");

        alerts.print("mev-alerts");
        alerts.sinkTo(buildKafkaSink(config.kafkaBootstrap(), config.topicPatternAnalytics(), "mev"));
    }

    // ── Pipeline 4: Market Trends (sliding windows) ──────────────────

  private static void buildMarketTrendStream(final DataStream<SwapEvent> tradingStream,
      final FlinkConfig config) {
    final DataStream<MarketTrend> trends = tradingStream
                .keyBy(event -> event.chain() + ":" + event.pairAddress())
        .window(SlidingEventTimeWindows.of(
            Duration.ofMinutes(config.trendWindowMinutes()),
            Duration.ofMinutes(config.trendSlideMinutes())))
                .aggregate(new SwapAggregator(), new MarketTrendWindowFunction())
                .name("market-trends");

        trends.print("market-trends");
        trends.sinkTo(buildKafkaSink(config.kafkaBootstrap(), config.topicMarketTrends(), "trends"));
    }

    // ── Shared sink builder ──────────────────────────────────────────

  private static <T> KafkaSink<T> buildKafkaSink(final String kafkaBootstrap, final String topic,
      final String sinkName) {
        return KafkaSink.<T>builder()
                .setBootstrapServers(kafkaBootstrap)
                .setDeliveryGuarantee(DeliveryGuarantee.EXACTLY_ONCE)
                .setTransactionalIdPrefix("dex-analytics-" + sinkName)
                .setProperty("transaction.timeout.ms", "600000")
                .setRecordSerializer(
                        KafkaRecordSerializationSchema.<T>builder()
                                .setTopic(topic)
                                .setValueSerializationSchema(new JsonSerializationSchema<>())
                                .build()
                )
                .build();
    }
}
