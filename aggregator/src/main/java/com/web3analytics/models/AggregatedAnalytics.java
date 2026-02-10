package com.web3analytics.models;

public record AggregatedAnalytics(
        String windowId,
        long windowStart,
        long windowEnd,
        String pairAddress,
        String token0Symbol,
        String token1Symbol,
        double twap,
        double openPrice,
        double closePrice,
        double highPrice,
        double lowPrice,
        double priceVolatility,
        String totalVolume0,
        String totalVolume1,
        Double volumeUSD,
        int swapCount,
        int uniqueTraders,
        String largestSwapValue,
        String largestSwapAddress,
        long totalGasUsed,
        String averageGasPrice,
        int arbitrageCount,
        java.util.List<String> repeatedTraders,
        long processedAt
) {}
