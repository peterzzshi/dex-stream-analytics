package com.web3analytics.functions.mev;

/**
 * Context for a MEV detection session window.
 */
public record SessionContext(
    String pairAddress,
    long windowStart,
    long windowEnd,
    String chain,
    String chainName,
    String token0Symbol,
    String token1Symbol,
    long detectedAt
) {

}
