package com.web3analytics.config;

import java.util.function.UnaryOperator;

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
        int checkpointMs,
        int tradingWindowMinutes,
        int liquidityWindowMinutes,
        int sessionGapSeconds,
        int trendWindowMinutes,
        int trendSlideMinutes
) {

    public static FlinkConfig fromEnv() {
      return fromEnv(System::getenv);
    }

  static FlinkConfig fromEnv(final UnaryOperator<String> env) {
        return new FlinkConfig(
            getRequired(env, "KAFKA_BOOTSTRAP_SERVERS"),
            getRequired(env, "TOPIC_TRADING_EVENTS"),
            getRequired(env, "TOPIC_LIQUIDITY_EVENTS"),
            getRequired(env, "TOPIC_TRADING_ANALYTICS"),
            getRequired(env, "TOPIC_LIQUIDITY_ANALYTICS"),
            getRequired(env, "TOPIC_PATTERN_ANALYTICS"),
            getRequired(env, "TOPIC_MARKET_TRENDS"),
            getRequired(env, "FLINK_CONSUMER_GROUP"),
            parseInt(env, "FLINK_PARALLELISM"),
            parseInt(env, "FLINK_CHECKPOINT_MS"),
            parseInt(env, "TRADING_WINDOW_MINUTES"),
            parseInt(env, "LIQUIDITY_WINDOW_MINUTES"),
            parseInt(env, "SESSION_GAP_SECONDS"),
            parseInt(env, "TREND_WINDOW_MINUTES"),
            parseInt(env, "TREND_SLIDE_MINUTES")
        );
    }

  private static String getRequired(final UnaryOperator<String> env, final String key) {
    final String value = env.apply(key);
    if (value == null || value.isBlank()) {
      throw new IllegalArgumentException("Missing required env var: " + key);
    }
    return value.trim();
    }

  private static int parseInt(final UnaryOperator<String> env, final String key) {
        try {
          return Integer.parseInt(getRequired(env, key));
        } catch (final NumberFormatException e) {
          throw new IllegalArgumentException("Invalid integer for " + key + ": " + env.apply(key),
              e);
        }
    }
}
