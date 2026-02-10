package com.web3analytics;

import com.web3analytics.config.FlinkConfig;
import com.web3analytics.functions.SwapAggregator;
import com.web3analytics.models.SwapEvent;
import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.serialization.AvroDeserializationSchema;
import com.web3analytics.serialization.AvroSerializationSchema;
import org.apache.flink.api.common.eventtime.WatermarkStrategy;
import org.apache.flink.connector.kafka.source.KafkaSource;
import org.apache.flink.connector.kafka.source.enumerator.initializer.OffsetsInitializer;
import org.apache.flink.connector.kafka.sink.KafkaSink;
import org.apache.flink.connector.kafka.sink.KafkaRecordSerializationSchema;
import org.apache.flink.streaming.api.datastream.DataStream;
import org.apache.flink.streaming.api.environment.StreamExecutionEnvironment;
import org.apache.flink.streaming.api.windowing.assigners.TumblingEventTimeWindows;
import org.apache.flink.streaming.api.windowing.time.Time;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.time.Duration;

/**
 * Main Flink streaming job for processing DEX swap events.
 *
 * This processor:
 * 1. Consumes swap events from Kafka
 * 2. Applies 5-minute tumbling windows
 * 3. Aggregates events to calculate TWAP, volume, and patterns
 * 4. Produces aggregated analytics to output Kafka topic
 */
public class StreamProcessor {

    private static final Logger LOG = LoggerFactory.getLogger(StreamProcessor.class);

    /**
     * Entry point for the Flink job.
     *
     * @param args Command line arguments (currently unused)
     * @throws Exception if the job fails
     */
    public static void main(String[] args) throws Exception {
        LOG.info("Starting Web3 DEX Analytics Stream Processor");

        // Load configuration
        FlinkConfig config = FlinkConfig.fromEnvironment();
        LOG.info("Configuration loaded: {}", config);

        // Create execution environment
        StreamExecutionEnvironment env = createExecutionEnvironment(config);

        // Create Kafka source for swap events
        KafkaSource<SwapEvent> source = createKafkaSource(config);

        // Create data stream from source
        DataStream<SwapEvent> swapEvents = env
            .fromSource(
                source,
                createWatermarkStrategy(),
                "DEX Swap Events Source"
            );

        // Apply windowing and aggregation
        DataStream<AggregatedAnalytics> analytics = swapEvents
            .keyBy(SwapEvent::getPairAddress)
            .window(TumblingEventTimeWindows.of(Time.minutes(5)))
            .aggregate(new SwapAggregator());

        // Create Kafka sink for analytics
        KafkaSink<AggregatedAnalytics> sink = createKafkaSink(config);

        // Write results to sink
        analytics.sinkTo(sink);

        // Execute the job
        LOG.info("Submitting Flink job...");
        env.execute("Web3 DEX Analytics Processor");
    }

    /**
     * Creates and configures the Flink execution environment.
     *
     * @param config Application configuration
     * @return Configured StreamExecutionEnvironment
     */
    private static StreamExecutionEnvironment createExecutionEnvironment(
            FlinkConfig config) {
        StreamExecutionEnvironment env = StreamExecutionEnvironment
            .getExecutionEnvironment();

        // Set parallelism
        env.setParallelism(config.getParallelism());

        // Enable checkpointing for fault tolerance
        env.enableCheckpointing(config.getCheckpointInterval());

        LOG.info("Execution environment configured with parallelism: {}",
                config.getParallelism());

        return env;
    }

    /**
     * Creates Kafka source for consuming swap events.
     *
     * @param config Application configuration
     * @return Configured KafkaSource
     */
    private static KafkaSource<SwapEvent> createKafkaSource(FlinkConfig config) {
        return KafkaSource.<SwapEvent>builder()
            .setBootstrapServers(config.getKafkaBootstrapServers())
            .setTopics(config.getInputTopic())
            .setGroupId(config.getConsumerGroup())
            .setStartingOffsets(OffsetsInitializer.earliest())
            .setValueOnlyDeserializer(
                new AvroDeserializationSchema<>(SwapEvent.class,
                    config.getSchemaRegistryUrl())
            )
            .build();
    }

    /**
     * Creates Kafka sink for producing aggregated analytics.
     *
     * @param config Application configuration
     * @return Configured KafkaSink
     */
    private static KafkaSink<AggregatedAnalytics> createKafkaSink(
            FlinkConfig config) {
        return KafkaSink.<AggregatedAnalytics>builder()
            .setBootstrapServers(config.getKafkaBootstrapServers())
            .setRecordSerializer(
                KafkaRecordSerializationSchema.builder()
                    .setTopic(config.getOutputTopic())
                    .setValueSerializationSchema(
                        new AvroSerializationSchema<>(
                            AggregatedAnalytics.class,
                            config.getSchemaRegistryUrl()
                        )
                    )
                    .build()
            )
            .build();
    }

    /**
     * Creates watermark strategy for event-time processing.
     *
     * Uses bounded out-of-orderness to handle late events
     * (allows up to 30 seconds of lateness).
     *
     * @return Configured WatermarkStrategy
     */
    private static WatermarkStrategy<SwapEvent> createWatermarkStrategy() {
        return WatermarkStrategy
            .<SwapEvent>forBoundedOutOfOrderness(Duration.ofSeconds(30))
            .withTimestampAssigner((event, timestamp) ->
                event.getBlockTimestamp() * 1000L // Convert to milliseconds
            );
    }
}