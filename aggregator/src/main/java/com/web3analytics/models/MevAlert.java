package com.web3analytics.models;

import java.util.List;

/**
 * MEV (Maximal Extractable Value) alert detected from cross-event pattern analysis.
 * Produced by session windows that group same-block activity per pair.
 */
public record MevAlert(
        String alertId,
        String alertType,
        long windowStart,
        long windowEnd,
        String pairAddress,
        String token0Symbol,
        String token1Symbol,
        long blockNumber,
        String attackerAddress,
        List<String> victimAddresses,
        double estimatedProfitUSD,
        int involvedSwapCount,
        int involvedEventCount,
        String severity,
        String description,
        List<String> involvedTransactions,
        long detectedAt
) {}
