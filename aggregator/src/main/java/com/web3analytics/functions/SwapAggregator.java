package com.web3analytics.functions;

import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.SwapEvent;
import com.web3analytics.types.TriState;
import org.apache.flink.api.common.functions.AggregateFunction;

import java.math.BigInteger;
import java.time.Instant;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

import static com.web3analytics.functions.Web3Math.normalizeAddress;
import static com.web3analytics.functions.Web3Math.parseBigInt;
import static com.web3analytics.functions.Web3Math.round8;

/**
 * Incremental swap aggregation. Window metadata is attached later by
 * {@link SwapAnalyticsWindowFunction}.
 */
public class SwapAggregator
        implements AggregateFunction<SwapEvent, SwapAggregator.Accumulator, AggregatedAnalytics> {

    public static class Accumulator {
        String pairAddress;
        TriState<String> token0Symbol = TriState.undefined();
        TriState<String> token1Symbol = TriState.undefined();

        double weightedPriceSum = 0.0;
        double totalVolume = 0.0;

        // OHLC with logIndex tie-breaker for same-block ordering
        double openPrice = Double.NaN;
        long openEventTime = Long.MAX_VALUE;
        int openLogIndex = Integer.MAX_VALUE;

        double closePrice = Double.NaN;
        long closeEventTime = Long.MIN_VALUE;
        int closeLogIndex = Integer.MIN_VALUE;

        double highPrice = Double.NEGATIVE_INFINITY;
        double lowPrice = Double.POSITIVE_INFINITY;

        BigInteger totalVolume0 = BigInteger.ZERO;
        BigInteger totalVolume1 = BigInteger.ZERO;
        double volumeUSD = 0.0;
        TriState<Double> volumeUSDState = TriState.undefined();

        int swapCount = 0;
        Map<String, Integer> traderCounts = new HashMap<>();

        long totalGasUsed = 0;
        BigInteger totalGasPrice = BigInteger.ZERO;

        LargestSwapCandidate largestSwap = LargestSwapCandidate.empty();
        int arbitrageCount = 0;
    }

    private record LargestSwapCandidate(Double usdValue, BigInteger token0Volume, String trader) {
        static LargestSwapCandidate empty() {
            return new LargestSwapCandidate(null, BigInteger.ZERO, "");
        }

        static LargestSwapCandidate fromEvent(SwapEvent event, BigInteger token0Volume, String trader) {
            return new LargestSwapCandidate(event.volumeUSD(), token0Volume, trader);
        }

        boolean hasUsdValue() {
            return usdValue != null;
        }
    }

    @Override
    public Accumulator createAccumulator() {
        return new Accumulator();
    }

    @Override
    public Accumulator add(SwapEvent event, Accumulator acc) {
        if (acc.pairAddress == null) {
            acc.pairAddress = event.pairAddress();
        }

        acc.token0Symbol = mergePreferred(acc.token0Symbol, event.token0Symbol());
        acc.token1Symbol = mergePreferred(acc.token1Symbol, event.token1Symbol());

        double price = event.price();
        BigInteger vol0 = parseBigInt(event.amount0In()).add(parseBigInt(event.amount0Out()));
        BigInteger vol1 = parseBigInt(event.amount1In()).add(parseBigInt(event.amount1Out()));
        double swapVolume = vol0.doubleValue();

        long eventTime = event.getEventTimeMillis();
        int logIndex = event.logIndex();

        applyOpen(acc, price, eventTime, logIndex);
        applyClose(acc, price, eventTime, logIndex);

        acc.weightedPriceSum += price * swapVolume;
        acc.totalVolume += swapVolume;

        acc.highPrice = Math.max(acc.highPrice, price);
        acc.lowPrice = Math.min(acc.lowPrice, price);

        acc.totalVolume0 = acc.totalVolume0.add(vol0);
        acc.totalVolume1 = acc.totalVolume1.add(vol1);

        if (event.volumeUSD() != null) {
            acc.volumeUSD += event.volumeUSD();
            acc.volumeUSDState = TriState.of(acc.volumeUSD);
        }

        String sender = normalizeAddress(event.sender());
        String recipient = normalizeAddress(event.recipient());

        acc.traderCounts.merge(sender, 1, Integer::sum);
        if (!recipient.equals(sender)) {
            acc.traderCounts.merge(recipient, 1, Integer::sum);
        }

        if (recipient.equals(sender)) {
            acc.arbitrageCount++;
        }

        acc.totalGasUsed += event.gasUsed();
        acc.totalGasPrice = acc.totalGasPrice.add(parseBigInt(event.gasPrice()));

        acc.largestSwap = chooseLargest(acc.largestSwap,
                LargestSwapCandidate.fromEvent(event, vol0, sender));

        acc.swapCount++;
        return acc;
    }

    @Override
    public AggregatedAnalytics getResult(Accumulator acc) {
        if (acc.swapCount == 0) {
            return emptyResult(acc);
        }

        double twap = acc.totalVolume > 0
                ? acc.weightedPriceSum / acc.totalVolume
                : acc.closePrice;

        double priceRange = acc.highPrice - acc.lowPrice;
        double priceVolatility = twap > 0 ? priceRange / twap : 0.0;

        String avgGasPrice = acc.totalGasPrice
                .divide(BigInteger.valueOf(acc.swapCount))
                .toString();

        List<String> repeatedTraders = acc.traderCounts.entrySet().stream()
                .filter(entry -> entry.getValue() >= 3)
                .map(Map.Entry::getKey)
                .toList();

        return new AggregatedAnalytics(
                UUID.randomUUID().toString(),
                acc.openEventTime,
                acc.closeEventTime,
                acc.pairAddress,
                toNullableString(acc.token0Symbol),
                toNullableString(acc.token1Symbol),
                round8(twap),
                round8(acc.openPrice),
                round8(acc.closePrice),
                round8(acc.highPrice),
                round8(acc.lowPrice),
                round8(priceVolatility),
                acc.totalVolume0.toString(),
                acc.totalVolume1.toString(),
                toNullableDouble(acc.volumeUSDState),
                acc.swapCount,
                acc.traderCounts.size(),
                formatLargestSwap(acc.largestSwap),
                acc.largestSwap.trader(),
                acc.totalGasUsed,
                avgGasPrice,
                acc.arbitrageCount,
                repeatedTraders,
                Instant.now().toEpochMilli()
        );
    }

    @Override
    public Accumulator merge(Accumulator a, Accumulator b) {
        if (isEmpty(a)) return b;
        if (isEmpty(b)) return a;

        Accumulator merged = new Accumulator();
        merged.pairAddress = a.pairAddress;
        merged.token0Symbol = mergeTriState(a.token0Symbol, b.token0Symbol);
        merged.token1Symbol = mergeTriState(a.token1Symbol, b.token1Symbol);

        merged.weightedPriceSum = a.weightedPriceSum + b.weightedPriceSum;
        merged.totalVolume = a.totalVolume + b.totalVolume;

        applyOpen(merged, a.openPrice, a.openEventTime, a.openLogIndex);
        applyOpen(merged, b.openPrice, b.openEventTime, b.openLogIndex);
        applyClose(merged, a.closePrice, a.closeEventTime, a.closeLogIndex);
        applyClose(merged, b.closePrice, b.closeEventTime, b.closeLogIndex);

        merged.highPrice = Math.max(a.highPrice, b.highPrice);
        merged.lowPrice = Math.min(a.lowPrice, b.lowPrice);

        merged.totalVolume0 = a.totalVolume0.add(b.totalVolume0);
        merged.totalVolume1 = a.totalVolume1.add(b.totalVolume1);
        merged.volumeUSD = a.volumeUSD + b.volumeUSD;
        if (a.volumeUSDState.isDefined() || b.volumeUSDState.isDefined()) {
            merged.volumeUSDState = TriState.of(merged.volumeUSD);
        }

        merged.swapCount = a.swapCount + b.swapCount;
        merged.traderCounts = new HashMap<>(a.traderCounts);
        b.traderCounts.forEach((k, v) -> merged.traderCounts.merge(k, v, Integer::sum));

        merged.totalGasUsed = a.totalGasUsed + b.totalGasUsed;
        merged.totalGasPrice = a.totalGasPrice.add(b.totalGasPrice);

        merged.arbitrageCount = a.arbitrageCount + b.arbitrageCount;
        merged.largestSwap = chooseLargest(a.largestSwap, b.largestSwap);

        return merged;
    }

    private static void applyOpen(Accumulator acc, double price, long eventTime, int logIndex) {
        if (eventTime < acc.openEventTime
                || (eventTime == acc.openEventTime && logIndex < acc.openLogIndex)) {
            acc.openPrice = price;
            acc.openEventTime = eventTime;
            acc.openLogIndex = logIndex;
        }
    }

    private static void applyClose(Accumulator acc, double price, long eventTime, int logIndex) {
        if (eventTime > acc.closeEventTime
                || (eventTime == acc.closeEventTime && logIndex >= acc.closeLogIndex)) {
            acc.closePrice = price;
            acc.closeEventTime = eventTime;
            acc.closeLogIndex = logIndex;
        }
    }

    private static LargestSwapCandidate chooseLargest(LargestSwapCandidate a, LargestSwapCandidate b) {
        if (a == null) return b;
        if (b == null) return a;
        if (a.hasUsdValue() && b.hasUsdValue()) return a.usdValue() >= b.usdValue() ? a : b;
        if (a.hasUsdValue() != b.hasUsdValue()) return a.hasUsdValue() ? a : b;
        return a.token0Volume().compareTo(b.token0Volume()) >= 0 ? a : b;
    }

    private static boolean isEmpty(Accumulator acc) {
        return acc == null || acc.swapCount == 0;
    }

    private static String formatLargestSwap(LargestSwapCandidate c) {
        if (c == null) return "0";
        if (c.usdValue() != null) return String.format("%.2f USD", c.usdValue());
        return c.token0Volume().toString();
    }

    private static AggregatedAnalytics emptyResult(Accumulator acc) {
        long now = Instant.now().toEpochMilli();
        return new AggregatedAnalytics(
                UUID.randomUUID().toString(), now, now,
                acc.pairAddress != null ? acc.pairAddress : "",
                toNullableString(acc.token0Symbol), toNullableString(acc.token1Symbol),
                0, 0, 0, 0, 0, 0,
                "0", "0", null, 0, 0, "0", "", 0, "0", 0, List.of(), now
        );
    }

    private static TriState<String> mergePreferred(TriState<String> current, String candidate) {
        if (current.isDefined()) return current;
        if (candidate == null || candidate.isBlank()) return current;
        return TriState.of(candidate);
    }

    private static <T> TriState<T> mergeTriState(TriState<T> left, TriState<T> right) {
        if (left.isDefined()) return left;
        if (right.isDefined()) return right;
        if (left.isNull() || right.isNull()) return TriState.ofNull();
        return TriState.undefined();
    }

    private static String toNullableString(TriState<String> state) {
        return state.fold(v -> v, () -> null, () -> null);
    }

    private static Double toNullableDouble(TriState<Double> state) {
        return state.fold(Web3Math::round8, () -> null, () -> null);
    }
}
