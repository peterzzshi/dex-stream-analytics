package com.web3analytics.models;

import java.util.List;

public record AggregatedAnalytics(
        String windowId,
        long windowStart,
        long windowEnd,
        String chain,
        String chainName,
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
        List<String> repeatedTraders,
        long processedAt
) {

  public AggregatedAnalytics {
    repeatedTraders = List.copyOf(repeatedTraders);
  }
}
