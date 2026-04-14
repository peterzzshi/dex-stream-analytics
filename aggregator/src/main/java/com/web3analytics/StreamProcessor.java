package com.web3analytics;

import com.web3analytics.config.FlinkConfig;
import com.web3analytics.functions.LiquidityWindowFunction;
import com.web3analytics.functions.SwapAnalyticsWindowFunction;
import com.web3analytics.functions.SwapAggregator;
import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.DecodingError;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.LiquidityAnalytics;
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
import org.apache.flink.connector.kafka.source.KafkaSource;
import org.apache.flink.connector.kafka.source.enumerator.initializer.OffsetsInitializer;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.datastream.SingleOutputStreamOperator;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.windowing.assigners.TumblingEventTimeWindows;
import org.apache.flink.util.OutputTag;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.time.Duration;

/**
 * Dual-topic Flink processor:
 * SwapEvent -> 5-min trading windows -> dex-trading-analytics
 * Mint/Burn/Transfer -> 1-hour liquidity windows -> dex-liquidity-analytics
 *
 * Uses native Kafka connector (not DAPR) for exactly-once semantics.
 */
public class StreamProcessor {

    private static final Logger LOG = LoggerFactory.getLogger(StreamProcessor.class);

    public static void main(String[] args) throws Exception {
        FlinkConfig config = FlinkConfig.fromEnv();
        LOG.info("Starting DEX Analytics Stream Processor");
        LOG.info("Trading: {} -> {}", config.topicTradingEvents(), config.topicTradingAnalytics());
        LOG.info("Liquidity: {} -> {}", config.topicLiquidityEvents(), config.topicLiquidityAnalytics());

        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.setParallelism(config.parallelism());
        env.enableCheckpointing(config.checkpointMs());

        buildTradingStream(env, config);
        buildLiquidityStream(env, config);

        env.execute("DEX Analytics Processor");
    }

    private static void buildTradingStream(StreamExecutionEnvironment env, FlinkConfig config) {
        KafkaSource<byte[]> source = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicTradingEvents())
                .setGroupId(config.consumerGroup() + "-trading")
                .setStartingOffsets(OffsetsInitializer.earliest())
                .setValueOnlyDeserializer(new ByteArrayPassthroughDeserializer())
                .build();

        OutputTag<DecodingError> decodeErrors = new OutputTag<>("trading-decode-errors") {};
        SwapEventDeserializer deserializer = new SwapEventDeserializer();

        SingleOutputStreamOperator<SwapEvent> events = env
                .fromSource(source, WatermarkStrategy.noWatermarks(), "trading-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicTradingEvents(),
                        deserializer::deserialize,
                        decodeErrors
                ))
                .returns(TypeInformation.of(SwapEvent.class))
                .name("decode-trading-events");

        events.getSideOutput(decodeErrors).print("trading-decode-errors");

        WatermarkStrategy<SwapEvent> watermarks = WatermarkStrategy
                .<SwapEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        DataStream<AggregatedAnalytics> analytics = events
                .assignTimestampsAndWatermarks(watermarks)
                .keyBy(SwapEvent::pairAddress)
                .window(TumblingEventTimeWindows.of(Duration.ofMinutes(5)))
                .aggregate(new SwapAggregator(), new SwapAnalyticsWindowFunction());

        analytics.print("trading-analytics");

        analytics.sinkTo(KafkaSink.<AggregatedAnalytics>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setRecordSerializer(
                        KafkaRecordSerializationSchema.<AggregatedAnalytics>builder()
                                .setTopic(config.topicTradingAnalytics())
                                .setValueSerializationSchema(new JsonSerializationSchema<>())
                                .build()
                )
                .build());
    }

    private static void buildLiquidityStream(StreamExecutionEnvironment env, FlinkConfig config) {
        KafkaSource<byte[]> source = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicLiquidityEvents())
                .setGroupId(config.consumerGroup() + "-liquidity")
                .setStartingOffsets(OffsetsInitializer.earliest())
                .setValueOnlyDeserializer(new ByteArrayPassthroughDeserializer())
                .build();

        OutputTag<DecodingError> decodeErrors = new OutputTag<>("liquidity-decode-errors") {};
        LiquidityEventDeserializer deserializer = new LiquidityEventDeserializer();

        SingleOutputStreamOperator<DexEvent> events = env
                .fromSource(source, WatermarkStrategy.noWatermarks(), "liquidity-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicLiquidityEvents(),
                        deserializer::deserialize,
                        decodeErrors
                ))
                .returns(TypeInformation.of(DexEvent.class))
                .name("decode-liquidity-events");

        events.getSideOutput(decodeErrors).print("liquidity-decode-errors");

        WatermarkStrategy<DexEvent> watermarks = WatermarkStrategy
                .<DexEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        DataStream<LiquidityAnalytics> analytics = events
                .assignTimestampsAndWatermarks(watermarks)
                .keyBy(DexEvent::pairAddress)
                .window(TumblingEventTimeWindows.of(Duration.ofHours(1)))
                .process(new LiquidityWindowFunction());

        analytics.print("liquidity-analytics");

        analytics.sinkTo(KafkaSink.<LiquidityAnalytics>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setRecordSerializer(
                        KafkaRecordSerializationSchema.<LiquidityAnalytics>builder()
                                .setTopic(config.topicLiquidityAnalytics())
                                .setValueSerializationSchema(new JsonSerializationSchema<>())
                                .build()
                )
                .build());
    }
}
