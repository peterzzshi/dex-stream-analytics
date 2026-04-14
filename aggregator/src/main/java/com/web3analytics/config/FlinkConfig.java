package com.web3analytics.config;

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
                requiredEnv("KAFKA_BOOTSTRAP_SERVERS"),
                requiredEnv("TOPIC_TRADING_EVENTS"),
                requiredEnv("TOPIC_LIQUIDITY_EVENTS"),
                requiredEnv("TOPIC_TRADING_ANALYTICS"),
                requiredEnv("TOPIC_LIQUIDITY_ANALYTICS"),
                requiredEnv("FLINK_CONSUMER_GROUP"),
                parseInt("FLINK_PARALLELISM"),
                parseLong("FLINK_CHECKPOINT_MS")
        );
    }

    private static String requiredEnv(String key) {
        String v = System.getenv(key);
        if (v == null || v.isBlank()) {
            throw new IllegalArgumentException("Missing required environment variable: " + key);
        }
        return v;
    }

    private static int parseInt(String key) {
        String value = requiredEnv(key);
        try {
            return Integer.parseInt(value);
        } catch (NumberFormatException err) {
            throw new IllegalArgumentException("Invalid integer for " + key + ": " + value, err);
        }
    }

    private static long parseLong(String key) {
        String value = requiredEnv(key);
        try {
            return Long.parseLong(value);
        } catch (NumberFormatException err) {
            throw new IllegalArgumentException("Invalid long for " + key + ": " + value, err);
        }
    }
}
