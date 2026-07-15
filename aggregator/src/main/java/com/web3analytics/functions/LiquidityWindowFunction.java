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
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

import static com.web3analytics.functions.Web3Math.normalizeAddress;
import static com.web3analytics.functions.Web3Math.parseBigInt;

/**
 * Full-window scan for LP behavior metrics.
 * Intentionally non-incremental: lower-frequency stream where buffering the full
 * window is acceptable and simplifies the churn/provider-set logic.
 *
 * Transfer events are correlated to Mint/Burn by transactionHash to compute
 * LP token amounts minted/burned per window.
 */
public class LiquidityWindowFunction
        extends ProcessWindowFunction<DexEvent, LiquidityAnalytics, String, TimeWindow> {

    // UniswapV2 zero address used for mint (from) and burn (to) transfers
    private static final String ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";

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
        int transferCount = 0;

        BigInteger totalMintAmount0 = BigInteger.ZERO;
        BigInteger totalMintAmount1 = BigInteger.ZERO;
        BigInteger totalBurnAmount0 = BigInteger.ZERO;
        BigInteger totalBurnAmount1 = BigInteger.ZERO;

        Set<String> mintProviders = new HashSet<>();
        Set<String> burnProviders = new HashSet<>();

        // Track transaction hashes that contain mint/burn for Transfer correlation
        Set<String> mintTxHashes = new HashSet<>();
        Set<String> burnTxHashes = new HashSet<>();

        // Collect Transfer events for second-pass correlation
        Map<String, BigInteger> transferValuesByTx = new HashMap<>();

        String token0Symbol = null;
        String token1Symbol = null;

        for (DexEvent event : events) {
            switch (event) {
                case MintEvent mint -> {
                    mintCount++;
                    totalMintAmount0 = totalMintAmount0.add(parseBigInt(mint.amount0()));
                    totalMintAmount1 = totalMintAmount1.add(parseBigInt(mint.amount1()));
                    mintProviders.add(normalizeAddress(mint.sender()));
                    mintTxHashes.add(mint.transactionHash());
                    token0Symbol = preferSymbol(token0Symbol, mint.token0Symbol());
                    token1Symbol = preferSymbol(token1Symbol, mint.token1Symbol());
                }
                case BurnEvent burn -> {
                    burnCount++;
                    totalBurnAmount0 = totalBurnAmount0.add(parseBigInt(burn.amount0()));
                    totalBurnAmount1 = totalBurnAmount1.add(parseBigInt(burn.amount1()));
                    burnProviders.add(normalizeAddress(burn.sender()));
                    burnTxHashes.add(burn.transactionHash());
                    token0Symbol = preferSymbol(token0Symbol, burn.token0Symbol());
                    token1Symbol = preferSymbol(token1Symbol, burn.token1Symbol());
                }
                case TransferEvent transfer -> {
                    transferCount++;
                    // Accumulate transfer value by txHash for correlation
                    BigInteger value = parseBigInt(transfer.value());
                    transferValuesByTx.merge(transfer.transactionHash(), value, BigInteger::add);
                }
                case com.web3analytics.models.SwapEvent ignored -> { }
            }
        }

        // Correlate: transfers in mint txs are LP tokens minted (from=0x0)
        // Transfers in burn txs are LP tokens burned (to=0x0)
        BigInteger totalLpMinted = BigInteger.ZERO;
        BigInteger totalLpBurned = BigInteger.ZERO;

        for (String txHash : mintTxHashes) {
            BigInteger lpValue = transferValuesByTx.get(txHash);
            if (lpValue != null) {
                totalLpMinted = totalLpMinted.add(lpValue);
            }
        }
        for (String txHash : burnTxHashes) {
            BigInteger lpValue = transferValuesByTx.get(txHash);
            if (lpValue != null) {
                totalLpBurned = totalLpBurned.add(lpValue);
            }
        }

        Set<String> uniqueProviders = new HashSet<>(mintProviders);
        uniqueProviders.addAll(burnProviders);

        Set<String> churnedProviders = new HashSet<>(mintProviders);
        churnedProviders.retainAll(burnProviders);

        return new LiquidityAnalytics(
                buildWindowId(pairAddress, windowStart, windowEnd),
                windowStart, windowEnd, pairAddress,
                token0Symbol, token1Symbol,
                mintCount, burnCount,
                totalMintAmount0.toString(), totalMintAmount1.toString(),
                totalBurnAmount0.toString(), totalBurnAmount1.toString(),
                totalMintAmount0.subtract(totalBurnAmount0).toString(),
                totalMintAmount1.subtract(totalBurnAmount1).toString(),
                totalLpMinted.toString(),
                totalLpBurned.toString(),
                totalLpMinted.subtract(totalLpBurned).toString(),
                transferCount,
                mintProviders.size(), burnProviders.size(),
                uniqueProviders.size(), churnedProviders.size(),
                processedAt
        );
    }

    static String buildWindowId(String pairAddress, long windowStart, long windowEnd) {
        return pairAddress + ":" + windowStart + ":" + windowEnd;
    }

    private static String preferSymbol(String current, String candidate) {
        if (current != null && !current.isBlank()) return current;
        if (candidate == null || candidate.isBlank()) return current;
        return candidate;
    }
}
