package com.web3analytics.functions.mev;

import com.web3analytics.models.MevAlert;

import java.util.UUID;

/**
 * Converts MEV candidates to alerts with severity classification.
 */
public final class AlertBuilder {

  private AlertBuilder() {
    throw new UnsupportedOperationException("Utility class");
  }

  public static MevAlert build(MevCandidate candidate, SessionContext context) {
    String severity = classifySeverity(candidate);

    return new MevAlert(
        UUID.randomUUID().toString(),
        candidate.patternType(),
        context.windowStart(),
        context.windowEnd(),
        context.chain(),
        context.chainName(),
        context.pairAddress(),
        context.token0Symbol(),
        context.token1Symbol(),
        candidate.blockNumber(),
        candidate.attackerAddress(),
        candidate.victimAddresses(),
        candidate.profitEstimate(),
        candidate.swapCount(),
        candidate.totalTransactionCount(),
        severity,
        candidate.description(),
        candidate.transactionHashes().stream().sorted().toList(),
        context.detectedAt()
    );
  }

  private static String classifySeverity(MevCandidate candidate) {
    double profit = candidate.profitEstimate();
    if (profit > 100.0) {
      return "HIGH";
    }
    if (profit > 10.0) {
      return "MEDIUM";
    }
    return "LOW";
  }
}
