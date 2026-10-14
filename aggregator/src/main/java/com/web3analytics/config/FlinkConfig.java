package com.web3analytics.config;

import java.time.Duration;
import java.util.Arrays;
import java.util.List;

public record FlinkConfig(
        String kafkaBootstrap,
        String topicTradingEvents,
        String topicLiquidityEvents,
        String topicTradingAnalytics,
        String topicLiquidityAnalytics,
        String topicPatternAnalytics,
        String topicMarketTrends,
        String consumerGroup,
        int parallelism,
        long checkpointMs,
        List<WindowConfig> tradingWindows,
        Duration liquidityWindow,
        Duration sessionGap,
        Duration trendWindow,
        Duration trendSlide,
        String transactionalIdPrefix
) {

    public record WindowConfig(Duration duration, String outputTopic) {}

    public static FlinkConfig fromEnv() {
        String tradingAnalyticsTopic = requiredEnv("TOPIC_TRADING_ANALYTICS");
        return new FlinkConfig(
                requiredEnv("KAFKA_BOOTSTRAP_SERVERS"),
                requiredEnv("TOPIC_TRADING_EVENTS"),
                requiredEnv("TOPIC_LIQUIDITY_EVENTS"),
                tradingAnalyticsTopic,
                requiredEnv("TOPIC_LIQUIDITY_ANALYTICS"),
                optionalEnv("TOPIC_PATTERN_ANALYTICS", "dex-pattern-analytics"),
                optionalEnv("TOPIC_MARKET_TRENDS", "dex-market-trends"),
                requiredEnv("FLINK_CONSUMER_GROUP"),
                parseInt("FLINK_PARALLELISM"),
                parseLong("FLINK_CHECKPOINT_MS"),
                parseTradingWindows(tradingAnalyticsTopic),
                parseLiquidityWindow(),
                parseOptionalDuration("SESSION_GAP_SECONDS", 3),
                parseOptionalDuration("TREND_WINDOW_MINUTES", 30),
                parseOptionalDuration("TREND_SLIDE_MINUTES", 5),
                optionalEnv("KAFKA_TRANSACTIONAL_ID_PREFIX", "dex-analytics")
        );
    }

    private static List<WindowConfig> parseTradingWindows(String baseTopic) {
        String raw = System.getenv("TRADING_WINDOW_MINUTES");
        if (raw == null || raw.isBlank()) {
            return List.of(new WindowConfig(Duration.ofMinutes(5), baseTopic));
        }
        List<WindowConfig> windows = Arrays.stream(raw.split(","))
                .map(String::trim)
                .filter(s -> !s.isEmpty())
                .map(s -> {
                    int minutes = parseMinutes(s);
                    String topic = baseTopic + "-" + minutes + "m";
                    return new WindowConfig(Duration.ofMinutes(minutes), topic);
                })
                .toList();
        if (windows.isEmpty()) {
            return List.of(new WindowConfig(Duration.ofMinutes(5), baseTopic));
        }
        return windows;
    }

    private static Duration parseLiquidityWindow() {
        String raw = System.getenv("LIQUIDITY_WINDOW_MINUTES");
        if (raw == null || raw.isBlank()) {
            return Duration.ofMinutes(60);
        }
        return Duration.ofMinutes(parseMinutes(raw.trim()));
    }

    private static int parseMinutes(String value) {
        try {
            int minutes = Integer.parseInt(value);
            if (minutes <= 0) {
                throw new IllegalArgumentException("Window minutes must be positive: " + value);
            }
            return minutes;
        } catch (NumberFormatException err) {
            throw new IllegalArgumentException("Invalid window minutes: " + value, err);
        }
    }

    private static String requiredEnv(String key) {
        String v = System.getenv(key);
        if (v == null || v.isBlank()) {
            throw new IllegalArgumentException("Missing required environment variable: " + key);
        }
        return v;
    }

    private static String optionalEnv(String key, String defaultValue) {
        String v = System.getenv(key);
        if (v == null || v.isBlank()) {
            return defaultValue;
        }
        return v.trim();
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

    private static Duration parseOptionalDuration(String key, int defaultValue) {
        String raw = System.getenv(key);
        if (raw == null || raw.isBlank()) {
            return key.contains("SECONDS")
                    ? Duration.ofSeconds(defaultValue)
                    : Duration.ofMinutes(defaultValue);
        }
        int parsed = parseMinutes(raw.trim());
        return key.contains("SECONDS")
                ? Duration.ofSeconds(parsed)
                : Duration.ofMinutes(parsed);
    }
}
