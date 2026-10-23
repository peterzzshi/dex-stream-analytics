package com.web3analytics.config;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

class FlinkConfigTest {

  private static Map<String, String> validEnv() {
    final Map<String, String> env = new HashMap<>();
    env.put("KAFKA_BOOTSTRAP_SERVERS", "localhost:9092");
    env.put("TOPIC_TRADING_EVENTS", "trading");
    env.put("TOPIC_LIQUIDITY_EVENTS", "liquidity");
    env.put("TOPIC_TRADING_ANALYTICS", "trading-analytics");
    env.put("TOPIC_LIQUIDITY_ANALYTICS", "liquidity-analytics");
    env.put("TOPIC_PATTERN_ANALYTICS", "patterns");
    env.put("TOPIC_MARKET_TRENDS", "trends");
    env.put("FLINK_CONSUMER_GROUP", "test-group");
    env.put("FLINK_PARALLELISM", "4");
    env.put("FLINK_CHECKPOINT_MS", "60000");
    env.put("TRADING_WINDOW_MINUTES", "5");
    env.put("LIQUIDITY_WINDOW_MINUTES", "60");
    env.put("SESSION_GAP_SECONDS", "3");
    env.put("TREND_WINDOW_MINUTES", "30");
    env.put("TREND_SLIDE_MINUTES", "5");
    return env;
  }

  private static Stream<String> missingOrBlankKafkaBootstrapServers() {
    return Stream.of(null, "  ");
  }

  private static Stream<Arguments> invalidIntegerFields() {
    return Stream.of(
        Arguments.of("FLINK_PARALLELISM", "not-a-number"),
        Arguments.of("FLINK_CHECKPOINT_MS", "invalid"));
  }

  @Test
  void fromEnv_loadsAllRequiredVariables() {
    final Map<String, String> env = FlinkConfigTest.validEnv();

    final FlinkConfig config = FlinkConfig.fromEnv(env::get);

    assertEquals("localhost:9092", config.kafkaBootstrap());
    assertEquals("trading", config.topicTradingEvents());
    assertEquals(4, config.parallelism());
    assertEquals(60000, config.checkpointMs());
    assertEquals(5, config.tradingWindowMinutes());
  }

  @ParameterizedTest
  @MethodSource("missingOrBlankKafkaBootstrapServers")
  void fromEnv_throwsWhenRequiredFieldMissingOrBlank(final String rawValue) {
    final Map<String, String> env = FlinkConfigTest.validEnv();
    if (rawValue == null) {
      env.remove("KAFKA_BOOTSTRAP_SERVERS");
    } else {
      env.put("KAFKA_BOOTSTRAP_SERVERS", rawValue);
    }

    final IllegalArgumentException ex = assertThrows(
        IllegalArgumentException.class,
        () -> FlinkConfig.fromEnv(env::get)
    );

    assertEquals("Missing required env var: KAFKA_BOOTSTRAP_SERVERS", ex.getMessage());
  }

  @ParameterizedTest
  @MethodSource("invalidIntegerFields")
  void fromEnv_throwsOnInvalidInteger(final String key, final String invalidValue) {
    final Map<String, String> env = FlinkConfigTest.validEnv();
    env.put(key, invalidValue);

    final IllegalArgumentException ex = assertThrows(
        IllegalArgumentException.class,
        () -> FlinkConfig.fromEnv(env::get)
    );

    assertEquals("Invalid integer for " + key + ": " + invalidValue, ex.getMessage());
  }

  @Test
  void fromEnv_trimsWhitespace() {
    final Map<String, String> env = FlinkConfigTest.validEnv();
    env.put("KAFKA_BOOTSTRAP_SERVERS", "  localhost:9092  ");

    final FlinkConfig config = FlinkConfig.fromEnv(env::get);

    assertEquals("localhost:9092", config.kafkaBootstrap());
  }
}
