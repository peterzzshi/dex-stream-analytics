package com.web3analytics.functions;

import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.MarketTrend;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

import java.time.Instant;

import static com.web3analytics.functions.Web3Math.round8;

/**
 * Converts aggregated swap metrics from a sliding window into a {@link MarketTrend}.
 * Reuses {@link SwapAggregator} for incremental computation; this function adds
 * trend direction and price-change semantics.
 */
public class MarketTrendWindowFunction
        extends ProcessWindowFunction<AggregatedAnalytics, MarketTrend, String, TimeWindow> {

    private static final double BULLISH_THRESHOLD = 0.01;
    private static final double BEARISH_THRESHOLD = -0.01;

    @Override
    public void process(
            String pairAddress,
            Context context,
            Iterable<AggregatedAnalytics> elements,
            Collector<MarketTrend> out
    ) {
        AggregatedAnalytics metrics = elements.iterator().next();
        out.collect(toMarketTrend(
                pairAddress,
                context.window().getStart(),
                context.window().getEnd(),
                metrics,
                Instant.now().toEpochMilli()
        ));
    }

    static MarketTrend toMarketTrend(
            String pairAddress,
            long windowStart,
            long windowEnd,
            AggregatedAnalytics metrics,
            long processedAt
    ) {
        double priceChange = 0.0;
        if (metrics.openPrice() > 0) {
            priceChange = (metrics.closePrice() - metrics.openPrice()) / metrics.openPrice();
        }

        double volatility = 0.0;
        if (metrics.twap() > 0) {
            volatility = (metrics.highPrice() - metrics.lowPrice()) / metrics.twap();
        }

        String trend;
        if (priceChange > BULLISH_THRESHOLD) {
            trend = "BULLISH";
        } else if (priceChange < BEARISH_THRESHOLD) {
            trend = "BEARISH";
        } else {
            trend = "NEUTRAL";
        }

        return new MarketTrend(
                buildWindowId(pairAddress, windowStart, windowEnd),
                windowStart, windowEnd,
                pairAddress,
                metrics.token0Symbol(),
                metrics.token1Symbol(),
                round8(metrics.twap()),
                round8(metrics.openPrice()),
                round8(metrics.closePrice()),
                round8(priceChange * 100),
                metrics.volumeUSD(),
                metrics.swapCount(),
                metrics.uniqueTraders(),
                round8(volatility),
                trend,
                processedAt
        );
    }

    static String buildWindowId(String pairAddress, long windowStart, long windowEnd) {
        return pairAddress + ":trend:" + windowStart + ":" + windowEnd;
    }
}
