package com.web3analytics.functions;

import com.web3analytics.models.DexEvent;
import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.MevAlert;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

import java.math.BigInteger;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

import static com.web3analytics.functions.Web3Math.normalizeAddress;
import static com.web3analytics.functions.Web3Math.parseBigInt;

/**
 * Session-window function detecting MEV patterns within burst activity on a single pair.
 *
 * <p>Detected patterns:
 * <ul>
 *   <li><b>SANDWICH_ATTACK</b>: Attacker front-runs + back-runs a victim swap in the same block</li>
 *   <li><b>JIT_LIQUIDITY</b>: Mint → large Swap → Burn in same or adjacent blocks</li>
 * </ul>
 */
public class MevDetectionFunction
        extends ProcessWindowFunction<DexEvent, MevAlert, String, TimeWindow> {

    @Override
    public void process(
            String pairAddress,
            Context context,
            Iterable<DexEvent> events,
            Collector<MevAlert> out
    ) {
        List<MevAlert> alerts = detect(
                pairAddress,
                context.window().getStart(),
                context.window().getEnd(),
                events,
                Instant.now().toEpochMilli()
        );
        alerts.forEach(out::collect);
    }

    static List<MevAlert> detect(
            String pairAddress,
            long windowStart,
            long windowEnd,
            Iterable<DexEvent> events,
            long detectedAt
    ) {
        List<DexEvent> sorted = new ArrayList<>();
        events.forEach(sorted::add);
        sorted.sort(Comparator.comparingLong(DexEvent::blockTimestamp)
                .thenComparingInt(DexEvent::logIndex));

        List<MevAlert> alerts = new ArrayList<>();

        // Group by block for intra-block pattern detection
        Map<Long, List<DexEvent>> byBlock = new HashMap<>();
        for (DexEvent e : sorted) {
            byBlock.computeIfAbsent(e.blockNumber(), k -> new ArrayList<>()).add(e);
        }

        String token0Symbol = null;
        String token1Symbol = null;
        for (DexEvent e : sorted) {
            if (e instanceof SwapEvent s) {
                if (token0Symbol == null) token0Symbol = s.token0Symbol();
                if (token1Symbol == null) token1Symbol = s.token1Symbol();
            } else if (e instanceof MintEvent m) {
                if (token0Symbol == null) token0Symbol = m.token0Symbol();
                if (token1Symbol == null) token1Symbol = m.token1Symbol();
            }
            if (token0Symbol != null && token1Symbol != null) break;
        }

        // Detect sandwich attacks per block
        for (var entry : byBlock.entrySet()) {
            long blockNumber = entry.getKey();
            List<SwapEvent> blockSwaps = entry.getValue().stream()
                    .filter(SwapEvent.class::isInstance)
                    .map(SwapEvent.class::cast)
                    .sorted(Comparator.comparingInt(SwapEvent::logIndex))
                    .toList();

            alerts.addAll(detectSandwich(
                    pairAddress, blockNumber, blockSwaps, windowStart, windowEnd,
                    token0Symbol, token1Symbol, detectedAt));
        }

        // Detect JIT liquidity across session
        alerts.addAll(detectJitLiquidity(
                pairAddress, sorted, windowStart, windowEnd,
                token0Symbol, token1Symbol, detectedAt));

        return alerts;
    }

    static List<MevAlert> detectSandwich(
            String pairAddress, long blockNumber, List<SwapEvent> swaps,
            long windowStart, long windowEnd,
            String token0Symbol, String token1Symbol, long detectedAt
    ) {
        if (swaps.size() < 3) return List.of();

        List<MevAlert> alerts = new ArrayList<>();

        for (int i = 0; i < swaps.size() - 2; i++) {
            SwapEvent frontRun = swaps.get(i);
            String attackerAddr = normalizeAddress(frontRun.sender());
            SwapDirection frontDir = direction(frontRun);
            if (frontDir == SwapDirection.UNKNOWN) continue;

            for (int j = i + 2; j < swaps.size(); j++) {
                SwapEvent backRun = swaps.get(j);
                String backAddr = normalizeAddress(backRun.sender());
                SwapDirection backDir = direction(backRun);

                if (!attackerAddr.equals(backAddr)) continue;
                if (frontDir == backDir) continue;

                // Found: same sender, opposite direction, with victim(s) between
                List<String> victims = new ArrayList<>();
                Set<String> txHashes = new HashSet<>();
                txHashes.add(frontRun.transactionHash());
                txHashes.add(backRun.transactionHash());

                for (int v = i + 1; v < j; v++) {
                    String victimAddr = normalizeAddress(swaps.get(v).sender());
                    if (!victimAddr.equals(attackerAddr)) {
                        victims.add(victimAddr);
                    }
                    txHashes.add(swaps.get(v).transactionHash());
                }

                if (victims.isEmpty()) continue;

                double profitEstimate = estimateProfit(frontRun, backRun);
                String severity = profitEstimate > 100.0 ? "HIGH"
                        : profitEstimate > 10.0 ? "MEDIUM" : "LOW";

                alerts.add(new MevAlert(
                        UUID.randomUUID().toString(),
                        "SANDWICH_ATTACK",
                        windowStart, windowEnd, pairAddress,
                        token0Symbol, token1Symbol,
                        blockNumber, attackerAddr, victims,
                        profitEstimate, j - i + 1, j - i + 1,
                        severity,
                        String.format("Sandwich: %s front-ran %d victim(s) in block %d",
                                attackerAddr, victims.size(), blockNumber),
                        txHashes.stream().sorted().toList(),
                        detectedAt
                ));

                break; // one sandwich per front-run position
            }
        }
        return alerts;
    }

    static List<MevAlert> detectJitLiquidity(
            String pairAddress, List<DexEvent> events,
            long windowStart, long windowEnd,
            String token0Symbol, String token1Symbol, long detectedAt
    ) {
        // JIT pattern: Mint and Burn within 2 blocks of a swap, from the same provider
        List<MintEvent> mints = new ArrayList<>();
        List<BurnEvent> burns = new ArrayList<>();
        List<SwapEvent> swaps = new ArrayList<>();

        for (DexEvent e : events) {
            switch (e) {
                case MintEvent m -> mints.add(m);
                case BurnEvent b -> burns.add(b);
                case SwapEvent s -> swaps.add(s);
                default -> { }
            }
        }

        if (mints.isEmpty() || burns.isEmpty() || swaps.isEmpty()) return List.of();

        List<MevAlert> alerts = new ArrayList<>();

        for (MintEvent mint : mints) {
            String mintProvider = normalizeAddress(mint.sender());
            long mintBlock = mint.blockNumber();

            for (BurnEvent burn : burns) {
                String burnProvider = normalizeAddress(burn.sender());
                long burnBlock = burn.blockNumber();

                // Same provider (or both routed through same contract), within 2 blocks
                if (!mintProvider.equals(burnProvider)) continue;
                if (Math.abs(burnBlock - mintBlock) > 2) continue;

                // Is there a swap between mint and burn blocks?
                boolean hasSwapBetween = swaps.stream().anyMatch(s ->
                        s.blockNumber() >= mintBlock && s.blockNumber() <= burnBlock);

                if (!hasSwapBetween) continue;

                Set<String> txHashes = new HashSet<>();
                txHashes.add(mint.transactionHash());
                txHashes.add(burn.transactionHash());
                swaps.stream()
                        .filter(s -> s.blockNumber() >= mintBlock && s.blockNumber() <= burnBlock)
                        .forEach(s -> txHashes.add(s.transactionHash()));

                alerts.add(new MevAlert(
                        UUID.randomUUID().toString(),
                        "JIT_LIQUIDITY",
                        windowStart, windowEnd, pairAddress,
                        token0Symbol, token1Symbol,
                        mintBlock, mintProvider, List.of(),
                        0.0, 0,
                        txHashes.size(),
                        "MEDIUM",
                        String.format("JIT liquidity: %s added/removed liquidity around swap in blocks %d-%d",
                                mintProvider, mintBlock, burnBlock),
                        txHashes.stream().sorted().toList(),
                        detectedAt
                ));
            }
        }
        return alerts;
    }

    private static double estimateProfit(SwapEvent frontRun, SwapEvent backRun) {
        Double frontUSD = frontRun.volumeUSD();
        Double backUSD = backRun.volumeUSD();
        if (frontUSD == null || backUSD == null) return 0.0;
        return Math.abs(backUSD - frontUSD);
    }

    enum SwapDirection { BUY_TOKEN0, BUY_TOKEN1, UNKNOWN }

    static SwapDirection direction(SwapEvent swap) {
        BigInteger a0In = parseBigInt(swap.amount0In());
        BigInteger a1In = parseBigInt(swap.amount1In());
        BigInteger a0Out = parseBigInt(swap.amount0Out());
        BigInteger a1Out = parseBigInt(swap.amount1Out());

        boolean sellingToken0 = a0In.signum() > 0 && a1Out.signum() > 0;
        boolean sellingToken1 = a1In.signum() > 0 && a0Out.signum() > 0;

        if (sellingToken1 && !sellingToken0) return SwapDirection.BUY_TOKEN0;
        if (sellingToken0 && !sellingToken1) return SwapDirection.BUY_TOKEN1;
        return SwapDirection.UNKNOWN;
    }
}
