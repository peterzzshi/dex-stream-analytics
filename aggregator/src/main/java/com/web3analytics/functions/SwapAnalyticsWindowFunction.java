package com.web3analytics.functions;

import static com.web3analytics.functions.WindowUtils.buildWindowId;

import com.web3analytics.models.AggregatedAnalytics;
import java.time.Instant;
import java.util.Iterator;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

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
      Iterator<AggregatedAnalytics> iterator = elements.iterator();
      if (!iterator.hasNext()) {
        throw new IllegalStateException("Window function received empty elements");
      }

      AggregatedAnalytics metrics = iterator.next();
        long windowStart = context.window().getStart();
        long windowEnd = context.window().getEnd();

        out.collect(new AggregatedAnalytics(
                buildWindowId(pairAddress, windowStart, windowEnd),
                windowStart, windowEnd,
                metrics.chain(),
                metrics.chainName(),
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
}
