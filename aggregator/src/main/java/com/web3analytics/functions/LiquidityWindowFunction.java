package com.web3analytics.functions;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.LiquidityAnalytics;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.TransferEvent;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

import java.math.BigInteger;
import java.time.Instant;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;

/**
 * Full-window processor for LP behavior metrics.
 *
 * This intentionally demonstrates a window-only operator for lower-frequency streams.
 */
public class LiquidityWindowFunction
        extends ProcessWindowFunction<DexEvent, LiquidityAnalytics, String, TimeWindow> {

    @Override
    public void process(
            String pairAddress,
            Context context,
            Iterable<DexEvent> events,
            Collector<LiquidityAnalytics> out
    ) {
        out.collect(summarize(
                pairAddress,
                context.window().getStart(),
                context.window().getEnd(),
                events,
                Instant.now().toEpochMilli()
        ));
    }

    static LiquidityAnalytics summarize(
            String pairAddress,
            long windowStart,
            long windowEnd,
            Iterable<DexEvent> events,
            long processedAt
    ) {
        int mintCount = 0;
        int burnCount = 0;

        BigInteger totalMintAmount0 = BigInteger.ZERO;
        BigInteger totalMintAmount1 = BigInteger.ZERO;
        BigInteger totalBurnAmount0 = BigInteger.ZERO;
        BigInteger totalBurnAmount1 = BigInteger.ZERO;

        Set<String> mintProviders = new HashSet<>();
        Set<String> burnProviders = new HashSet<>();

        String token0Symbol = null;
        String token1Symbol = null;

        for (DexEvent event : events) {
            switch (event) {
                case MintEvent mint -> {
                    mintCount++;
                    totalMintAmount0 = totalMintAmount0.add(parseBigInt(mint.amount0()));
                    totalMintAmount1 = totalMintAmount1.add(parseBigInt(mint.amount1()));
                    mintProviders.add(normalizeAddress(mint.sender()));
                    token0Symbol = preferSymbol(token0Symbol, mint.token0Symbol());
                    token1Symbol = preferSymbol(token1Symbol, mint.token1Symbol());
                }
                case BurnEvent burn -> {
                    burnCount++;
                    totalBurnAmount0 = totalBurnAmount0.add(parseBigInt(burn.amount0()));
                    totalBurnAmount1 = totalBurnAmount1.add(parseBigInt(burn.amount1()));
                    burnProviders.add(normalizeAddress(burn.sender()));
                    token0Symbol = preferSymbol(token0Symbol, burn.token0Symbol());
                    token1Symbol = preferSymbol(token1Symbol, burn.token1Symbol());
                }
                case com.web3analytics.models.SwapEvent ignored -> {
                    // Liquidity stream is Mint/Burn only; keep exhaustive switch for safety.
                }
                case TransferEvent ignored -> {
                    // Transfer events are ingested for LP token correlation phases, not aggregated yet.
                }
            }
        }

        Set<String> uniqueProviders = new HashSet<>(mintProviders);
        uniqueProviders.addAll(burnProviders);

        Set<String> churnedProviders = new HashSet<>(mintProviders);
        churnedProviders.retainAll(burnProviders);

        return new LiquidityAnalytics(
                buildWindowId(pairAddress, windowStart, windowEnd),
                windowStart,
                windowEnd,
                pairAddress,
                token0Symbol,
                token1Symbol,
                mintCount,
                burnCount,
                totalMintAmount0.toString(),
                totalMintAmount1.toString(),
                totalBurnAmount0.toString(),
                totalBurnAmount1.toString(),
                totalMintAmount0.subtract(totalBurnAmount0).toString(),
                totalMintAmount1.subtract(totalBurnAmount1).toString(),
                mintProviders.size(),
                burnProviders.size(),
                uniqueProviders.size(),
                churnedProviders.size(),
                processedAt
        );
    }

    static String buildWindowId(String pairAddress, long windowStart, long windowEnd) {
        return pairAddress + ":" + windowStart + ":" + windowEnd;
    }

    private static String preferSymbol(String current, String candidate) {
        if (current != null && !current.isBlank()) {
            return current;
        }
        if (candidate == null || candidate.isBlank()) {
            return current;
        }
        return candidate;
    }

    private static BigInteger parseBigInt(String value) {
        if (!isSignedInteger(value)) {
            return BigInteger.ZERO;
        }
        return new BigInteger(value.trim());
    }

    private static boolean isSignedInteger(String value) {
        if (value == null) {
            return false;
        }

        String trimmed = value.trim();
        if (trimmed.isEmpty()) {
            return false;
        }

        int start = trimmed.charAt(0) == '-' ? 1 : 0;
        if (start == trimmed.length()) {
            return false;
        }

        for (int i = start; i < trimmed.length(); i++) {
            if (!Character.isDigit(trimmed.charAt(i))) {
                return false;
            }
        }

        return true;
    }

    private static String normalizeAddress(String address) {
        if (address == null) {
            return "";
        }
        return address.toLowerCase(Locale.ROOT);
    }
}
