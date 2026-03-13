package com.web3analytics.config;

/**
 * Flink configuration for dual-topic architecture.
 * Consumes from two input topics (trading and liquidity events).
 */
public record FlinkConfig(
        String kafkaBootstrap,
        String topicTradingEvents,
        String topicLiquidityEvents,
        String topicTradingAnalytics,
        String topicLiquidityAnalytics,
        String consumerGroup,
        int parallelism,
        long checkpointMs
) {
    public static FlinkConfig fromEnv() {
        return new FlinkConfig(
                env("KAFKA_BOOTSTRAP_SERVERS", env("KAFKA_BOOTSTRAP", "localhost:9092")),
                env("TOPIC_TRADING_EVENTS", "dex-trading-events"),
                env("TOPIC_LIQUIDITY_EVENTS", "dex-liquidity-events"),
                env("TOPIC_TRADING_ANALYTICS", "dex-trading-analytics"),
                env("TOPIC_LIQUIDITY_ANALYTICS", "dex-liquidity-analytics"),
                env("FLINK_CONSUMER_GROUP", "dex-processor"),
                Integer.parseInt(env("FLINK_PARALLELISM", "2")),
                Long.parseLong(env("FLINK_CHECKPOINT_MS", "10000"))
        );
    }

    private static String env(String key, String def) {
        String v = System.getenv(key);
        return v == null || v.isBlank() ? def : v;
    }
}
