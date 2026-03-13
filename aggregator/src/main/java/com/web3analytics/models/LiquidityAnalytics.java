package com.web3analytics.models;

/**
 * 1-hour liquidity behavior analytics computed from Mint/Burn events.
 */
public record LiquidityAnalytics(
        String windowId,
        long windowStart,
        long windowEnd,
        String pairAddress,
        String token0Symbol,
        String token1Symbol,
        int mintCount,
        int burnCount,
        String totalMintAmount0,
        String totalMintAmount1,
        String totalBurnAmount0,
        String totalBurnAmount1,
        String netLiquidityChange0,
        String netLiquidityChange1,
        int uniqueMintProviders,
        int uniqueBurnProviders,
        int uniqueProviders,
        int churnedProviders,
        long processedAt
) {
}
