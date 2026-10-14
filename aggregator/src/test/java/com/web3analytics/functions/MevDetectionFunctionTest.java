package com.web3analytics.functions;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MevAlert;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class MevDetectionFunctionTest {

    private static final String PAIR = "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827";
    private static final String ATTACKER = "0xattacker";
    private static final String VICTIM = "0xvictim";
    private static final String LP_PROVIDER = "0xlpprovider";

    @Test
    void detectsSandwichAttack() {
        // Block 100: attacker buys token0, victim buys token0, attacker sells token0
        List<DexEvent> events = List.of(
                swap(100, 0, ATTACKER, "0", "5000", "1000", "0", 2.0, 10.0),  // buy token0
                swap(100, 1, VICTIM, "0", "3000", "500", "0", 1.8, 5.4),       // buy token0 (slipped)
                swap(100, 2, ATTACKER, "1000", "0", "0", "5200", 2.1, 10.5)    // sell token0
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);

        assertEquals(1, alerts.size());
        MevAlert alert = alerts.get(0);
        assertEquals("SANDWICH_ATTACK", alert.alertType());
        assertEquals(ATTACKER.toLowerCase(), alert.attackerAddress());
        assertEquals(1, alert.victimAddresses().size());
        assertTrue(alert.victimAddresses().contains(VICTIM.toLowerCase()));
        assertEquals(100, alert.blockNumber());
        assertEquals(3, alert.involvedSwapCount());
    }

    @Test
    void noAlertForNormalTrading() {
        // Three different traders in the same block, no sandwich pattern
        List<DexEvent> events = List.of(
                swap(100, 0, "0xtrader1", "0", "1000", "500", "0", 2.0, 5.0),
                swap(100, 1, "0xtrader2", "0", "2000", "800", "0", 1.9, 7.6),
                swap(100, 2, "0xtrader3", "500", "0", "0", "1200", 2.1, 5.0)
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);
        assertTrue(alerts.isEmpty());
    }

    @Test
    void noAlertForTwoSwaps() {
        // Only 2 swaps - not enough for sandwich
        List<DexEvent> events = List.of(
                swap(100, 0, ATTACKER, "0", "5000", "1000", "0", 2.0, 10.0),
                swap(100, 1, VICTIM, "0", "3000", "500", "0", 1.8, 5.4)
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);
        assertTrue(alerts.isEmpty());
    }

    @Test
    void noSandwichWhenSameDirection() {
        // Attacker buys, victim buys, attacker buys again (no reversal)
        List<DexEvent> events = List.of(
                swap(100, 0, ATTACKER, "0", "5000", "1000", "0", 2.0, 10.0),
                swap(100, 1, VICTIM, "0", "3000", "500", "0", 1.8, 5.4),
                swap(100, 2, ATTACKER, "0", "5000", "1000", "0", 2.0, 10.0)
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);
        assertTrue(alerts.isEmpty());
    }

    @Test
    void detectsJitLiquidity() {
        long ts = 1700000000;
        // Block 100: mint, then swap, then burn - same provider
        List<DexEvent> events = List.of(
                mint(100, 0, LP_PROVIDER, ts),
                swap(100, 1, "0xtrader", "0", "5000", "1000", "0", 2.0, 10.0),
                burn(100, 2, LP_PROVIDER, ts)
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);

        assertEquals(1, alerts.size());
        assertEquals("JIT_LIQUIDITY", alerts.get(0).alertType());
        assertEquals(LP_PROVIDER.toLowerCase(), alerts.get(0).attackerAddress());
    }

    @Test
    void noJitWhenDifferentProviders() {
        long ts = 1700000000;
        List<DexEvent> events = List.of(
                mint(100, 0, "0xproviderA", ts),
                swap(100, 1, "0xtrader", "0", "5000", "1000", "0", 2.0, 10.0),
                burn(100, 2, "0xproviderB", ts)
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);
        assertTrue(alerts.stream().noneMatch(a -> a.alertType().equals("JIT_LIQUIDITY")));
    }

    @Test
    void noJitWithoutSwapBetween() {
        long ts = 1700000000;
        // Mint at block 100, burn at block 105 - blocks too far apart
        List<DexEvent> events = List.of(
                mint(100, 0, LP_PROVIDER, ts),
                burn(105, 1, LP_PROVIDER, ts + 10)
        );

        List<MevAlert> alerts = MevDetectionFunction.detect(PAIR, 0, 10000, events, 999);
        assertTrue(alerts.isEmpty());
    }

    @Test
    void swapDirectionDetection() {
        // Buying token0: sending token1 in, receiving token0 out
        SwapEvent buyToken0 = swap(100, 0, "0xaddr", "0", "5000", "1000", "0", 2.0, 10.0);
        assertEquals(MevDetectionFunction.SwapDirection.BUY_TOKEN0,
                MevDetectionFunction.direction(buyToken0));

        // Buying token1: sending token0 in, receiving token1 out
        SwapEvent buyToken1 = swap(100, 0, "0xaddr", "1000", "0", "0", "5000", 0.5, 10.0);
        assertEquals(MevDetectionFunction.SwapDirection.BUY_TOKEN1,
                MevDetectionFunction.direction(buyToken1));
    }

    private SwapEvent swap(long block, int logIndex, String sender,
                           String a0In, String a1In, String a0Out, String a1Out,
                           double price, Double volumeUSD) {
        return new SwapEvent(
                "evt-" + block + "-" + logIndex, block, block, "0xtx" + block + logIndex,
                logIndex, PAIR, "0xtoken0", "0xtoken1", "WMATIC", "USDC",
                sender, "0xrecipient", a0In, a1In, a0Out, a1Out,
                price, volumeUSD, 21000, "50000000000", block
        );
    }

    private MintEvent mint(long block, int logIndex, String sender, long ts) {
        return new MintEvent(
                "evt-mint-" + block, block, ts, "0xtx-mint-" + block,
                logIndex, PAIR, "0xtoken0", "0xtoken1", "WMATIC", "USDC",
                sender, "1000000", "2000000", ts
        );
    }

    private BurnEvent burn(long block, int logIndex, String sender, long ts) {
        return new BurnEvent(
                "evt-burn-" + block, block, ts, "0xtx-burn-" + block,
                logIndex, PAIR, "0xtoken0", "0xtoken1", "WMATIC", "USDC",
                sender, "0xrecipient", "1000000", "2000000", ts
        );
    }
}
