package com.web3analytics.functions;

import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.MarketTrend;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class MarketTrendWindowFunctionTest {

    @Test
    void bullishTrend() {
        AggregatedAnalytics metrics = analytics(1.0, 1.5, 1.6, 0.9, 1.2, 5000.0, 20, 8);

        MarketTrend trend = MarketTrendWindowFunction.toMarketTrend(
                "0xpair", 0, 300000, metrics, 999);

        assertEquals("BULLISH", trend.trend());
        assertTrue(trend.priceChangePercent() > 0);
        assertEquals(1.2, trend.avgPrice());
        assertEquals(20, trend.swapCount());
        assertEquals(8, trend.uniqueTraders());
    }

    @Test
    void bearishTrend() {
        AggregatedAnalytics metrics = analytics(1.5, 1.0, 1.6, 0.9, 1.2, 5000.0, 15, 5);

        MarketTrend trend = MarketTrendWindowFunction.toMarketTrend(
                "0xpair", 0, 300000, metrics, 999);

        assertEquals("BEARISH", trend.trend());
        assertTrue(trend.priceChangePercent() < 0);
    }

    @Test
    void neutralTrend() {
        AggregatedAnalytics metrics = analytics(1.0, 1.005, 1.01, 0.99, 1.0, 3000.0, 10, 4);

        MarketTrend trend = MarketTrendWindowFunction.toMarketTrend(
                "0xpair", 0, 300000, metrics, 999);

        assertEquals("NEUTRAL", trend.trend());
    }

    @Test
    void volatilityCalculation() {
        // High=2.0, Low=1.0 → range=1.0, twap=1.5 → volatility=0.667
        AggregatedAnalytics metrics = analytics(1.2, 1.8, 2.0, 1.0, 1.5, 10000.0, 50, 20);

        MarketTrend trend = MarketTrendWindowFunction.toMarketTrend(
                "0xpair", 0, 300000, metrics, 999);

        assertTrue(trend.volatility() > 0.6 && trend.volatility() < 0.7);
    }

    @Test
    void windowIdFormat() {
        String id = MarketTrendWindowFunction.buildWindowId("0xpair", 1000, 2000);
        assertEquals("0xpair:trend:1000:2000", id);
    }

    @Test
    void zeroOpenPriceHandled() {
        AggregatedAnalytics metrics = analytics(0.0, 1.0, 1.0, 0.0, 0.5, null, 1, 1);

        MarketTrend trend = MarketTrendWindowFunction.toMarketTrend(
                "0xpair", 0, 300000, metrics, 999);

        assertEquals(0.0, trend.priceChangePercent());
    }

    private AggregatedAnalytics analytics(
            double open, double close, double high, double low, double twap,
            Double volumeUSD, int swapCount, int uniqueTraders
    ) {
        return new AggregatedAnalytics(
                "test-window", 0, 300000, "0xpair",
                "WMATIC", "USDC",
                twap, open, close, high, low, 0.0,
                "100000", "200000", volumeUSD,
                swapCount, uniqueTraders,
                "100.00 USD", "0xwhale",
                500000, "50000000000",
                0, List.of(), 999
        );
    }
}
