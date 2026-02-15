package com.web3analytics.config;

public record FlinkConfig(
        String kafkaBootstrap,
        String inputTopic,
        String outputTopic,
        String consumerGroup,
        int parallelism,
        long checkpointMs
) {
    public static FlinkConfig fromEnv() {
        return new FlinkConfig(
                env("KAFKA_BOOTSTRAP", "kafka:9092"),
                env("TOPIC_DEX_EVENTS", "dex-events"),
                env("TOPIC_DEX_ANALYTICS", "dex-analytics"),
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
