package com.web3analytics.models;

/**
 * Rolling market trend computed from sliding windows over swap events.
 * Captures price momentum, volume trends, and market direction.
 */
public record MarketTrend(
        String windowId,
        long windowStart,
        long windowEnd,
        String pairAddress,
        String token0Symbol,
        String token1Symbol,
        double avgPrice,
        double openPrice,
        double closePrice,
        double priceChangePercent,
        Double volumeUSD,
        int swapCount,
        int uniqueTraders,
        double volatility,
        String trend,
        long processedAt
) {}
