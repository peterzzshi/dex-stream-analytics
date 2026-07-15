package com.web3analytics;

import com.web3analytics.config.FlinkConfig;
import com.web3analytics.config.FlinkConfig.WindowConfig;
import com.web3analytics.functions.LiquidityWindowFunction;
import com.web3analytics.functions.MarketTrendWindowFunction;
import com.web3analytics.functions.MevDetectionFunction;
import com.web3analytics.functions.SwapAnalyticsWindowFunction;
import com.web3analytics.functions.SwapAggregator;
import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.DecodingError;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.LiquidityAnalytics;
import com.web3analytics.models.MarketTrend;
import com.web3analytics.models.MevAlert;
import com.web3analytics.models.SwapEvent;
import com.web3analytics.serde.JsonSerializationSchema;
import com.web3analytics.serde.ByteArrayPassthroughDeserializer;
import com.web3analytics.serde.LiquidityEventDeserializer;
import com.web3analytics.serde.SafeDecodeProcessFunction;
import com.web3analytics.serde.SwapEventDeserializer;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.apache.flink.connector.kafka.sink.KafkaRecordSerializationSchema;
import org.apache.flink.connector.kafka.sink.KafkaSink;
import org.apache.flink.connector.base.DeliveryGuarantee;
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

import java.time.Duration;

/**
 * Multi-window Flink processor demonstrating four window strategies:
 *
 * <ul>
 *   <li><b>Tumbling</b>: SwapEvent → 5-min trading analytics (TWAP, OHLC, volume)</li>
 *   <li><b>Tumbling</b>: Mint/Burn/Transfer → 1-hour liquidity analytics</li>
 *   <li><b>Session</b>: All events → MEV detection (sandwich attacks, JIT liquidity)</li>
 *   <li><b>Sliding</b>: SwapEvent → 30-min/5-min market trends</li>
 * </ul>
 *
 * Decode and watermark assignment happen once per source; window pipelines fan out
 * from the shared watermarked streams.
 *
 * Uses native Kafka connector (not DAPR) for exactly-once semantics.
 */
public class StreamProcessor {

    private static final Logger LOG = LoggerFactory.getLogger(StreamProcessor.class);

    public static void main(String[] args) throws Exception {
        FlinkConfig config = FlinkConfig.fromEnv();
        LOG.info("Starting DEX Analytics Stream Processor");
        LOG.info("Trading: {} -> {} window(s)", config.topicTradingEvents(), config.tradingWindows().size());
        for (WindowConfig w : config.tradingWindows()) {
            LOG.info("  Tumbling window: {} -> {}", w.duration(), w.outputTopic());
        }
        LOG.info("Liquidity: {} -> {} (tumbling={})", config.topicLiquidityEvents(),
                config.topicLiquidityAnalytics(), config.liquidityWindow());
        LOG.info("MEV detection: session gap={} -> {}", config.sessionGap(), config.topicPatternAnalytics());
        LOG.info("Market trends: sliding={}/{} -> {}", config.trendWindow(), config.trendSlide(),
                config.topicMarketTrends());

        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.setParallelism(config.parallelism());
        env.enableCheckpointing(config.checkpointMs());

        // Decode once, fan out to all pipelines
        DataStream<SwapEvent> tradingStream = buildWatermarkedTradingStream(env, config);
        DataStream<DexEvent> liquidityStream = buildWatermarkedLiquidityStream(env, config);

        // 1. Tumbling windows: Trading analytics
        for (WindowConfig window : config.tradingWindows()) {
            buildTradingWindowPipeline(tradingStream, window, config.kafkaBootstrap());
        }

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
            StreamExecutionEnvironment env, FlinkConfig config
    ) {
        KafkaSource<byte[]> source = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicTradingEvents())
                .setGroupId(config.consumerGroup() + "-trading")
                .setStartingOffsets(OffsetsInitializer.earliest())
                .setValueOnlyDeserializer(new ByteArrayPassthroughDeserializer())
                .build();

        OutputTag<DecodingError> decodeErrors = new OutputTag<>("trading-decode-errors") {};
        SwapEventDeserializer deserializer = new SwapEventDeserializer();

        SingleOutputStreamOperator<SwapEvent> decoded = env
                .fromSource(source, WatermarkStrategy.noWatermarks(), "trading-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicTradingEvents(),
                        deserializer::deserialize,
                        decodeErrors
                ))
                .returns(TypeInformation.of(SwapEvent.class))
                .name("decode-trading-events");

        decoded.getSideOutput(decodeErrors).print("trading-decode-errors");

        WatermarkStrategy<SwapEvent> watermarks = WatermarkStrategy
                .<SwapEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        return decoded.assignTimestampsAndWatermarks(watermarks);
    }

    private static DataStream<DexEvent> buildWatermarkedLiquidityStream(
            StreamExecutionEnvironment env, FlinkConfig config
    ) {
        KafkaSource<byte[]> source = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicLiquidityEvents())
                .setGroupId(config.consumerGroup() + "-liquidity")
                .setStartingOffsets(OffsetsInitializer.earliest())
                .setValueOnlyDeserializer(new ByteArrayPassthroughDeserializer())
                .build();

        OutputTag<DecodingError> decodeErrors = new OutputTag<>("liquidity-decode-errors") {};
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();

        SingleOutputStreamOperator<DexEvent> decoded = env
                .fromSource(source, WatermarkStrategy.noWatermarks(), "liquidity-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicLiquidityEvents(),
                        deserializer::deserialize,
                        decodeErrors
                ))
                .returns(TypeInformation.of(DexEvent.class))
                .name("decode-liquidity-events");

        decoded.getSideOutput(decodeErrors).print("liquidity-decode-errors");

        WatermarkStrategy<DexEvent> watermarks = WatermarkStrategy
                .<DexEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        return decoded.assignTimestampsAndWatermarks(watermarks);
    }

    // ── Pipeline 1: Trading Analytics (tumbling windows) ─────────────

    private static void buildTradingWindowPipeline(
            DataStream<SwapEvent> watermarked,
            WindowConfig window,
            String kafkaBootstrap
    ) {
        long minutes = window.duration().toMinutes();
        String name = "trading-analytics-" + minutes + "m";

        DataStream<AggregatedAnalytics> analytics = watermarked
                .keyBy(SwapEvent::pairAddress)
                .window(TumblingEventTimeWindows.of(window.duration()))
                .aggregate(new SwapAggregator(), new SwapAnalyticsWindowFunction())
                .name(name);

        analytics.print(name);
        analytics.sinkTo(buildKafkaSink(kafkaBootstrap, window.outputTopic(), name));
    }

    // ── Pipeline 2: Liquidity Analytics (tumbling window) ────────────

    private static void buildLiquidityPipeline(DataStream<DexEvent> liquidityStream, FlinkConfig config) {
        DataStream<LiquidityAnalytics> analytics = liquidityStream
                .keyBy(DexEvent::pairAddress)
                .window(TumblingEventTimeWindows.of(config.liquidityWindow()))
                .process(new LiquidityWindowFunction());

        analytics.print("liquidity-analytics");
        analytics.sinkTo(buildKafkaSink(config.kafkaBootstrap(), config.topicLiquidityAnalytics(), "liquidity"));
    }

    // ── Pipeline 3: MEV Detection (session windows) ──────────────────

    private static void buildMevDetectionStream(
            DataStream<SwapEvent> tradingStream,
            DataStream<DexEvent> liquidityStream,
            FlinkConfig config
    ) {
        // Widen SwapEvent to DexEvent for union
        DataStream<DexEvent> swapsAsDex = tradingStream
                .map(event -> (DexEvent) event)
                .returns(TypeInformation.of(DexEvent.class));

        DataStream<DexEvent> allEvents = swapsAsDex.union(liquidityStream);

        DataStream<MevAlert> alerts = allEvents
                .keyBy(DexEvent::pairAddress)
                .window(EventTimeSessionWindows.withGap(config.sessionGap()))
                .process(new MevDetectionFunction())
                .name("mev-detection");

        alerts.print("mev-alerts");
        alerts.sinkTo(buildKafkaSink(config.kafkaBootstrap(), config.topicPatternAnalytics(), "mev"));
    }

    // ── Pipeline 4: Market Trends (sliding windows) ──────────────────

    private static void buildMarketTrendStream(DataStream<SwapEvent> tradingStream, FlinkConfig config) {
        DataStream<MarketTrend> trends = tradingStream
                .keyBy(SwapEvent::pairAddress)
                .window(SlidingEventTimeWindows.of(config.trendWindow(), config.trendSlide()))
                .aggregate(new SwapAggregator(), new MarketTrendWindowFunction())
                .name("market-trends");

        trends.print("market-trends");
        trends.sinkTo(buildKafkaSink(config.kafkaBootstrap(), config.topicMarketTrends(), "trends"));
    }

    // ── Shared sink builder ──────────────────────────────────────────

    private static <T> KafkaSink<T> buildKafkaSink(String kafkaBootstrap, String topic, String sinkName) {
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
