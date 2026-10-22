package com.web3analytics.functions.mev;

import java.util.List;
import java.util.Set;

/**
 * Intermediate MEV detection result before severity classification.
 */
public record MevCandidate(
    String patternType,
    long blockNumber,
    String attackerAddress,
    List<String> victimAddresses,
    double profitEstimate,
    int swapCount,
    int totalTransactionCount,
    Set<String> transactionHashes,
    String description
) {

}
