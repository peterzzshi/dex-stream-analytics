package com.web3analytics.models;

/**
 * Liquidity behavior analytics computed from Mint/Burn/Transfer events.
 * Transfer events are correlated by transactionHash to compute LP token amounts.
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
        String totalLpTokensMinted,
        String totalLpTokensBurned,
        String netLpTokenChange,
        int transferCount,
        int uniqueMintProviders,
        int uniqueBurnProviders,
        int uniqueProviders,
        int churnedProviders,
        long processedAt
) {
}
