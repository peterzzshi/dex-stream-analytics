package com.web3analytics.functions;

import com.web3analytics.models.AggregatedAnalytics;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

import java.time.Instant;

/** Replaces placeholder window metadata with authoritative Flink window boundaries. */
public class SwapAnalyticsWindowFunction
        extends ProcessWindowFunction<AggregatedAnalytics, AggregatedAnalytics, String, TimeWindow> {

    @Override
    public void process(
            String pairAddress,
            Context context,
            Iterable<AggregatedAnalytics> elements,
            Collector<AggregatedAnalytics> out
    ) {
        AggregatedAnalytics metrics = elements.iterator().next();
        long windowStart = context.window().getStart();
        long windowEnd = context.window().getEnd();

        out.collect(new AggregatedAnalytics(
                buildWindowId(pairAddress, windowStart, windowEnd),
                windowStart, windowEnd,
                metrics.pairAddress(),
                metrics.token0Symbol(), metrics.token1Symbol(),
                metrics.twap(), metrics.openPrice(), metrics.closePrice(),
                metrics.highPrice(), metrics.lowPrice(), metrics.priceVolatility(),
                metrics.totalVolume0(), metrics.totalVolume1(), metrics.volumeUSD(),
                metrics.swapCount(), metrics.uniqueTraders(),
                metrics.largestSwapValue(), metrics.largestSwapAddress(),
                metrics.totalGasUsed(), metrics.averageGasPrice(),
                metrics.arbitrageCount(), metrics.repeatedTraders(),
                Instant.now().toEpochMilli()
        ));
    }

    static String buildWindowId(String pairAddress, long windowStart, long windowEnd) {
        return pairAddress + ":" + windowStart + ":" + windowEnd;
    }
}
