package com.web3analytics.config;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Map;
import java.util.function.UnaryOperator;
import org.junit.jupiter.api.Test;

class BridgeStreamProcessorConfigTest {

  private static Map<String, String> validEnv() {
    final Map<String, String> env = new HashMap<>();
    env.put("KAFKA_BOOTSTRAP_SERVERS", "kafka:9092");
    env.put("BRIDGE_TRANSFER_TOPIC", "bridge-transfer-events");
    env.put("BRIDGE_SUPPLY_TOPIC", "bridge-supply-events");
    env.put("TRANSFER_VOLUME_ENDPOINT", "http://analytics:8080/api/bridges/transfer-volume");
    env.put("SUPPLY_TRACKING_ENDPOINT", "http://analytics:8080/api/bridges/supply");
    env.put("CROSS_CHAIN_FLOW_ENDPOINT", "http://analytics:8080/api/bridges/flow");
    env.put("EXPLOIT_DETECTION_ENDPOINT", "http://analytics:8080/api/bridges/alert");
    env.put("REENTRANCY_DETECTION_ENDPOINT", "http://analytics:8080/api/bridges/reentrancy");
    env.put("WATERMARK_DELAY_SECONDS", "5");
    env.put("TRANSFER_WINDOW_SECONDS", "300");
    env.put("SUPPLY_WINDOW_SECONDS", "300");
    env.put("FLOW_WINDOW_SECONDS", "300");
    env.put("EXPLOIT_SESSION_GAP_SECONDS", "60");
    env.put("REENTRANCY_WINDOW_SECONDS", "5");
    env.put("RESTART_MAX_ATTEMPTS", "3");
    env.put("RESTART_DELAY_SECONDS", "10");
    env.put("CHECKPOINT_INTERVAL_SECONDS", "60");
    return env;
  }

  private static UnaryOperator<String> envOf(final Map<String, String> env) {
    return env::get;
  }

  @Test
  void readsEveryValueFromTheEnvironment() {
    final BridgeStreamProcessorConfig config =
        BridgeStreamProcessorConfig.fromEnv(envOf(validEnv()));

    assertEquals("bridge-transfer-events", config.bridgeTransferTopic());
    assertEquals(300, config.flowWindowSeconds());
    assertEquals(5, config.reentrancyWindowSeconds());
    assertEquals(3, config.restartMaxAttempts());
    assertEquals(10, config.restartDelaySeconds());
  }

  @Test
  void missingVariableFailsInsteadOfFallingBackToADefault() {
    final Map<String, String> env = validEnv();
    env.remove("REENTRANCY_WINDOW_SECONDS");

    final IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
        () -> BridgeStreamProcessorConfig.fromEnv(envOf(env)));

    assertEquals("Missing required env var: REENTRANCY_WINDOW_SECONDS", error.getMessage());
  }

  @Test
  void blankVariableCountsAsMissing() {
    final Map<String, String> env = validEnv();
    env.put("BRIDGE_SUPPLY_TOPIC", "   ");

    assertThrows(IllegalArgumentException.class,
        () -> BridgeStreamProcessorConfig.fromEnv(envOf(env)));
  }

  @Test
  void nonNumericWindowIsRejected() {
    final Map<String, String> env = validEnv();
    env.put("TRANSFER_WINDOW_SECONDS", "five-minutes");

    final IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
        () -> BridgeStreamProcessorConfig.fromEnv(envOf(env)));

    assertEquals("Invalid integer for TRANSFER_WINDOW_SECONDS: five-minutes", error.getMessage());
  }

  @Test
  void zeroOrNegativeWindowIsRejected() {
    final Map<String, String> env = validEnv();
    env.put("WATERMARK_DELAY_SECONDS", "0");

    final IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
        () -> BridgeStreamProcessorConfig.fromEnv(envOf(env)));

    assertEquals("WATERMARK_DELAY_SECONDS must be positive, got 0", error.getMessage());
  }

  @Test
  void nonNumericRestartDelayIsRejected() {
    final Map<String, String> env = validEnv();
    env.put("RESTART_DELAY_SECONDS", "10s");

    final IllegalArgumentException error = assertThrows(IllegalArgumentException.class,
        () -> BridgeStreamProcessorConfig.fromEnv(envOf(env)));

    assertEquals("Invalid integer for RESTART_DELAY_SECONDS: 10s", error.getMessage());
  }
}
