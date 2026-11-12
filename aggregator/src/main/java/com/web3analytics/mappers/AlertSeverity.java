package com.web3analytics.mappers;

import com.web3analytics.alerts.Severity;
import com.web3analytics.model.CrossChainFlowAccumulator;
import com.web3analytics.model.ExploitDetectionAccumulator;
import com.web3analytics.model.ReentrancyAccumulator;
import com.web3analytics.model.SupplyTrackingAccumulator;
import com.web3analytics.model.TransferVolumeAccumulator;
import java.math.BigInteger;

/**
 * Computes alert severity from aggregated data. Thresholds are calibrated for demonstration
 * purposes and would need tuning based on actual bridge activity patterns.
 */
public final class AlertSeverity {

  private static final BigInteger TRANSFER_CRITICAL = BigInteger.valueOf(1000000);
  private static final BigInteger TRANSFER_HIGH = BigInteger.valueOf(100000);
  private static final BigInteger TRANSFER_MEDIUM = BigInteger.valueOf(10000);

  private static final BigInteger SUPPLY_CRITICAL = BigInteger.valueOf(500000);
  private static final BigInteger SUPPLY_HIGH = BigInteger.valueOf(50000);
  private static final BigInteger SUPPLY_MEDIUM = BigInteger.valueOf(5000);

  private AlertSeverity() {
  }

  public static Severity compute(final TransferVolumeAccumulator acc) {
    final BigInteger totalVolume = acc.totalLockAmount.add(acc.totalUnlockAmount);
    if (totalVolume.compareTo(TRANSFER_CRITICAL) > 0) {
      return Severity.CRITICAL;
    }
    if (totalVolume.compareTo(TRANSFER_HIGH) > 0) {
      return Severity.HIGH;
    }
    if (totalVolume.compareTo(TRANSFER_MEDIUM) > 0) {
      return Severity.MEDIUM;
    }
    return Severity.LOW;
  }

  public static Severity compute(final SupplyTrackingAccumulator acc) {
    final BigInteger absNetMinted = acc.netMinted.abs();
    // Burn-only with no mints is suspicious (drain attack)
    if (acc.mintCount == 0 && acc.burnCount > 0) {
      return Severity.CRITICAL;
    }
    if (absNetMinted.compareTo(SUPPLY_CRITICAL) > 0) {
      return Severity.CRITICAL;
    }
    if (absNetMinted.compareTo(SUPPLY_HIGH) > 0) {
      return Severity.HIGH;
    }
    if (absNetMinted.compareTo(SUPPLY_MEDIUM) > 0) {
      return Severity.MEDIUM;
    }
    return Severity.LOW;
  }

  public static Severity compute(final CrossChainFlowAccumulator acc) {
    // Imbalance ratio already computed by aggregate function
    if (acc.imbalanceRatio > 0.5) {
      return Severity.CRITICAL;
    }
    if (acc.imbalanceRatio > 0.3) {
      return Severity.HIGH;
    }
    if (acc.imbalanceRatio > 0.15) {
      return Severity.MEDIUM;
    }
    return Severity.LOW;
  }

  public static Severity compute(final ExploitDetectionAccumulator acc) {
    final int patternCount = acc.suspiciousPatterns.size();
    if (patternCount > 2) {
      return Severity.CRITICAL;
    }
    if (patternCount == 2) {
      return Severity.HIGH;
    }
    if (patternCount == 1) {
      return Severity.MEDIUM;
    }
    return Severity.LOW;
  }

  public static Severity compute(final ReentrancyAccumulator acc) {
    final int maxCalls = Math.max(acc.lockCount, acc.unlockCount);
    if (maxCalls > 5) {
      return Severity.CRITICAL;
    }
    if (maxCalls > 2) {
      return Severity.HIGH;
    }
    if (maxCalls == 2) {
      return Severity.MEDIUM;
    }
    return Severity.LOW;
  }
}
