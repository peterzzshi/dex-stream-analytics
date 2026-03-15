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
import com.web3analytics.serde.AvroSerializationSchema;
import com.web3analytics.serde.ByteArrayPassthroughDeserializer;
import com.web3analytics.serde.LiquidityEventDeserializer;
import com.web3analytics.serde.SafeDecodeProcessFunction;
import com.web3analytics.serde.SwapEventDeserializer;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
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
 * Multi-source Flink processor for dual-topic DEX analytics.
 * 
 * Architecture:
 * - Trading stream: SwapEvent (dex-trading-events) → 5-min windows → dex-trading-analytics
 * - Liquidity stream: Mint/Burn/Transfer events (dex-liquidity-events) → 1-hour windows → dex-liquidity-analytics
 * 
 * Uses native Kafka connector (not DAPR) for exactly-once semantics and checkpointing.
 */
public class StreamProcessor {

    private static final Logger LOG = LoggerFactory.getLogger(StreamProcessor.class);

    public static void main(String[] args) throws Exception {
        FlinkConfig config = FlinkConfig.fromEnv();
        LOG.info("Starting DEX Analytics Stream Processor with dual-topic architecture");
        LOG.info("Trading events: {}", config.topicTradingEvents());
        LOG.info("Liquidity events: {}", config.topicLiquidityEvents());
        LOG.info("Trading output: {}", config.topicTradingAnalytics());
        LOG.info("Liquidity output: {}", config.topicLiquidityAnalytics());

        StreamExecutionEnvironment env = StreamExecutionEnvironment.getExecutionEnvironment();
        env.setParallelism(config.parallelism());
        
        // TODO: Enable checkpointing in production
        // env.enableCheckpointing(config.checkpointMs());

        // ========== Trading Stream ==========
        // Demonstrates incremental `aggregate` followed by `ProcessWindowFunction`.
        // Read raw bytes so decode failures can be routed to side output instead of failing the job.
        KafkaSource<byte[]> tradingSource = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicTradingEvents())
                .setGroupId(config.consumerGroup() + "-trading")
                .setStartingOffsets(OffsetsInitializer.earliest())
                .setValueOnlyDeserializer(new ByteArrayPassthroughDeserializer())
                .build();

        WatermarkStrategy<SwapEvent> swapWatermarks = WatermarkStrategy
                .<SwapEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis()); // Using DexEvent default method

        OutputTag<DecodingError> tradingDecodeErrors = new OutputTag<>("trading-decode-errors") {};
        SwapEventDeserializer swapDeserializer = new SwapEventDeserializer();

        SingleOutputStreamOperator<SwapEvent> tradingEvents = env
                .fromSource(tradingSource, WatermarkStrategy.noWatermarks(), "trading-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicTradingEvents(),
                        swapDeserializer::deserialize,
                        tradingDecodeErrors
                ))
                .name("decode-trading-events");

        tradingEvents.getSideOutput(tradingDecodeErrors).print("trading-decode-errors");

        DataStream<AggregatedAnalytics> tradingAnalytics = tradingEvents
                .assignTimestampsAndWatermarks(swapWatermarks)
                .keyBy(SwapEvent::pairAddress)
                .window(TumblingEventTimeWindows.of(Duration.ofMinutes(5)))
                .aggregate(new SwapAggregator(), new SwapAnalyticsWindowFunction());

        // Print for observability
        tradingAnalytics.print("trading-analytics");

        // Sink to output topic
        KafkaSink<AggregatedAnalytics> tradingSink = KafkaSink.<AggregatedAnalytics>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setRecordSerializer(
                        KafkaRecordSerializationSchema.<AggregatedAnalytics>builder()
                                .setTopic(config.topicTradingAnalytics())
                                .setValueSerializationSchema(new AvroSerializationSchema<AggregatedAnalytics>())
                                .build()
                )
                .build();

        tradingAnalytics.sinkTo(tradingSink);

        // ========== Liquidity Stream ==========
        // Demonstrates full-window `ProcessWindowFunction` without incremental aggregate.
        // Liquidity source is heterogeneous and routed by CloudEvent type before Avro deserialization.
        KafkaSource<byte[]> liquiditySource = KafkaSource.<byte[]>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setTopics(config.topicLiquidityEvents())
                .setGroupId(config.consumerGroup() + "-liquidity")
                .setStartingOffsets(OffsetsInitializer.earliest())
                .setValueOnlyDeserializer(new ByteArrayPassthroughDeserializer())
                .build();

        WatermarkStrategy<DexEvent> liquidityWatermarks = WatermarkStrategy
                .<DexEvent>forBoundedOutOfOrderness(Duration.ofSeconds(60))
                .withTimestampAssigner((event, ts) -> event.getEventTimeMillis());

        OutputTag<DecodingError> liquidityDecodeErrors = new OutputTag<>("liquidity-decode-errors") {};
        LiquidityEventDeserializer liquidityDeserializer = new LiquidityEventDeserializer();

        SingleOutputStreamOperator<DexEvent> liquidityEvents = env
                .fromSource(liquiditySource, WatermarkStrategy.noWatermarks(), "liquidity-events-raw")
                .process(new SafeDecodeProcessFunction<>(
                        config.topicLiquidityEvents(),
                        liquidityDeserializer::deserialize,
                        liquidityDecodeErrors
                ))
                .name("decode-liquidity-events");

        liquidityEvents.getSideOutput(liquidityDecodeErrors).print("liquidity-decode-errors");

        DataStream<LiquidityAnalytics> liquidityAnalytics = liquidityEvents
                .assignTimestampsAndWatermarks(liquidityWatermarks)
                .keyBy(DexEvent::pairAddress)
                .window(TumblingEventTimeWindows.of(Duration.ofHours(1)))
                .process(new LiquidityWindowFunction());

        liquidityAnalytics.print("liquidity-analytics");

        KafkaSink<LiquidityAnalytics> liquiditySink = KafkaSink.<LiquidityAnalytics>builder()
                .setBootstrapServers(config.kafkaBootstrap())
                .setRecordSerializer(
                        KafkaRecordSerializationSchema.<LiquidityAnalytics>builder()
                                .setTopic(config.topicLiquidityAnalytics())
                                .setValueSerializationSchema(new AvroSerializationSchema<LiquidityAnalytics>())
                                .build()
                )
                .build();

        liquidityAnalytics.sinkTo(liquiditySink);

        env.execute("DEX Analytics Processor");
    }
}
