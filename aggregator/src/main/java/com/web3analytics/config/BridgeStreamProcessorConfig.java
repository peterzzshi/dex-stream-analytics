package com.web3analytics.config;

import java.util.function.UnaryOperator;

/**
 * Every value the bridge job reads from its environment, resolved once at startup. No field has a
 * default: the deployment config is the single source of truth, so a missing or malformed variable
 * fails the job at launch instead of silently running with a value nobody chose.
 */
public record BridgeStreamProcessorConfig(
    String kafkaBootstrapServers,
    String bridgeTransferTopic,
    String bridgeSupplyTopic,
    String transferVolumeEndpoint,
    String supplyTrackingEndpoint,
    String crossChainFlowEndpoint,
    String exploitDetectionEndpoint,
    String reentrancyDetectionEndpoint,
    int watermarkDelaySeconds,
    int transferWindowSeconds,
    int supplyWindowSeconds,
    int flowWindowSeconds,
    int exploitSessionGapSeconds,
    int reentrancyWindowSeconds,
    int restartMaxAttempts,
    int restartDelaySeconds,
    int checkpointIntervalSeconds) {

  public static BridgeStreamProcessorConfig fromEnv() {
    return fromEnv(System::getenv);
  }

  static BridgeStreamProcessorConfig fromEnv(final UnaryOperator<String> env) {
    return new BridgeStreamProcessorConfig(
        required(env, "KAFKA_BOOTSTRAP_SERVERS"),
        required(env, "BRIDGE_TRANSFER_TOPIC"),
        required(env, "BRIDGE_SUPPLY_TOPIC"),
        required(env, "TRANSFER_VOLUME_ENDPOINT"),
        required(env, "SUPPLY_TRACKING_ENDPOINT"),
        required(env, "CROSS_CHAIN_FLOW_ENDPOINT"),
        required(env, "EXPLOIT_DETECTION_ENDPOINT"),
        required(env, "REENTRANCY_DETECTION_ENDPOINT"),
        positiveInt(env, "WATERMARK_DELAY_SECONDS"),
        positiveInt(env, "TRANSFER_WINDOW_SECONDS"),
        positiveInt(env, "SUPPLY_WINDOW_SECONDS"),
        positiveInt(env, "FLOW_WINDOW_SECONDS"),
        positiveInt(env, "EXPLOIT_SESSION_GAP_SECONDS"),
        positiveInt(env, "REENTRANCY_WINDOW_SECONDS"),
        positiveInt(env, "RESTART_MAX_ATTEMPTS"),
        positiveInt(env, "RESTART_DELAY_SECONDS"),
        positiveInt(env, "CHECKPOINT_INTERVAL_SECONDS"));
  }

  private static String required(final UnaryOperator<String> env, final String key) {
    final String value = env.apply(key);
    if (value == null || value.isBlank()) {
      throw new IllegalArgumentException("Missing required env var: " + key);
    }
    return value.trim();
  }

  private static int positiveInt(final UnaryOperator<String> env, final String key) {
    final String raw = required(env, key);
    final int value;
    try {
      value = Integer.parseInt(raw);
    } catch (final NumberFormatException e) {
      throw new IllegalArgumentException("Invalid integer for " + key + ": " + raw, e);
    }
    if (value <= 0) {
      throw new IllegalArgumentException(key + " must be positive, got " + value);
    }
    return value;
  }
}
