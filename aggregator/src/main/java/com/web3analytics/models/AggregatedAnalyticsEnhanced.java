package com.web3analytics.models;

import com.web3analytics.types.TriState;

/**
 * Example: Enhanced AggregatedAnalytics using TriState for optional fields.
 * 
 * Demonstrates practical application of tri-state optionals:
 * - volumeUSD: calculated | not-yet-calculated | N/A-for-non-USD-pair
 * - token0Symbol: resolved | pending | failed
 * - token1Symbol: resolved | pending | failed
 * 
 * Benefits over nullable Double:
 * 1. Explicit distinction between "not calculated" vs "doesn't apply"
 * 2. Type-safe handling (compiler-enforced pattern matching)
 * 3. Serialization control (omit vs null vs value)
 * 4. FP operations (map, flatMap, orElse)
 * 
 * Note: This is a demonstration class showing how TriState could be integrated.
 * The actual AggregatedAnalytics uses Avro-generated code (double/Double).
 */
public record AggregatedAnalyticsEnhanced(
    String windowId,
    long windowStart,
    long windowEnd,
    String pairAddress,
    
    // Price metrics
    double twap,
    double openPrice,
    double closePrice,
    double highPrice,
    double lowPrice,
    double priceVolatility,
    
    // Volume metrics
    double totalVolume0,
    double totalVolume1,
    int swapCount,
    int uniqueTraders,
    
    // Optional fields using TriState
    TriState<Double> volumeUSD,        // Calculated | Pending | N/A
    TriState<String> token0Symbol,     // Resolved | Pending | Failed
    TriState<String> token1Symbol,     // Resolved | Pending | Failed
    
    // Pattern detection
    int arbitrageCount,
    int repeatedTraders,
    TriState<String> largestSwapAddress, // Found | Not-tracked | No-swaps
    
    // Gas metrics
    long totalGasUsed,
    double averageGasPrice
) {
    
    /**
     * Example: Convert to display-friendly format.
     */
    public String getVolumeDisplay() {
        return volumeUSD.fold(
            value -> String.format("$%.2f", value),
            () -> "Calculating...",
            () -> "N/A"
        );
    }
    
    /**
     * Example: Get token pair display name.
     */
    public String getPairDisplay() {
        String token0 = token0Symbol.orElse("Token0");
        String token1 = token1Symbol.orElse("Token1");
        return token0 + "/" + token1;
    }
    
    /**
     * Example: Check if analytics are complete (all optional fields resolved).
     */
    public boolean isComplete() {
        return volumeUSD.isDefined() 
            && token0Symbol.isDefined() 
            && token1Symbol.isDefined();
    }
    
    /**
     * Example: Calculate completion percentage.
     */
    public double getCompletionPercentage() {
        int total = 3; // volumeUSD, token0Symbol, token1Symbol
        int defined = 0;
        if (volumeUSD.isDefined()) defined++;
        if (token0Symbol.isDefined()) defined++;
        if (token1Symbol.isDefined()) defined++;
        return (defined * 100.0) / total;
    }
    
    /**
     * Builder pattern for constructing analytics with tri-state fields.
     */
    public static class Builder {
        private String windowId;
        private long windowStart;
        private long windowEnd;
        private String pairAddress;
        private double twap;
        private double openPrice;
        private double closePrice;
        private double highPrice;
        private double lowPrice;
        private double priceVolatility;
        private double totalVolume0;
        private double totalVolume1;
        private int swapCount;
        private int uniqueTraders;
        private TriState<Double> volumeUSD = TriState.undefined();
        private TriState<String> token0Symbol = TriState.undefined();
        private TriState<String> token1Symbol = TriState.undefined();
        private int arbitrageCount;
        private int repeatedTraders;
        private TriState<String> largestSwapAddress = TriState.undefined();
        private long totalGasUsed;
        private double averageGasPrice;
        
        public Builder windowId(String windowId) {
            this.windowId = windowId;
            return this;
        }
        
        public Builder pairAddress(String pairAddress) {
            this.pairAddress = pairAddress;
            return this;
        }
        
        public Builder volumeUSD(Double value) {
            this.volumeUSD = value == null ? TriState.ofNull() : TriState.of(value);
            return this;
        }
        
        public Builder volumeUSDUndefined() {
            this.volumeUSD = TriState.undefined();
            return this;
        }
        
        public Builder token0Symbol(String symbol) {
            this.token0Symbol = symbol == null ? TriState.ofNull() : TriState.of(symbol);
            return this;
        }
        
        // ... other builder methods ...
        
        public AggregatedAnalyticsEnhanced build() {
            return new AggregatedAnalyticsEnhanced(
                windowId, windowStart, windowEnd, pairAddress,
                twap, openPrice, closePrice, highPrice, lowPrice, priceVolatility,
                totalVolume0, totalVolume1, swapCount, uniqueTraders,
                volumeUSD, token0Symbol, token1Symbol,
                arbitrageCount, repeatedTraders, largestSwapAddress,
                totalGasUsed, averageGasPrice
            );
        }
    }
}
