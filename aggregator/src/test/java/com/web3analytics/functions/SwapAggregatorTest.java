package com.web3analytics.functions;

import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.SwapEvent;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class SwapAggregatorTest {

    private SwapAggregator aggregator;

    @BeforeEach
    void setUp() {
        aggregator = new SwapAggregator();
    }

    // --- helpers ---

    private SwapEvent swap(String pair, String sender, String recipient,
                           double price, String amount0In, String amount0Out,
                           String amount1In, String amount1Out,
                           Double volumeUSD, long gasUsed, String gasPrice,
                           long blockTimestamp) {
        return new SwapEvent(
                "evt-" + System.nanoTime(), 100L, blockTimestamp,
                "0xtx", 0, pair, "0xtoken0", "0xtoken1", "WETH", "USDC",
                sender, recipient,
                amount0In, amount1In, amount0Out, amount1Out,
                price, volumeUSD, gasUsed, gasPrice, blockTimestamp
        );
    }

    private SwapEvent simpleSwap(double price, String amount0In, Double volumeUSD) {
        return swap("0xpair", "0xalice", "0xbob", price,
                amount0In, "0", "0", "0",
                volumeUSD, 21000L, "50000000000", 1700000000L);
    }

    // --- TWAP ---

    @Test
    void twap_singleSwap_equalToPrice() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(2000.0, "1000", null), acc);

        AggregatedAnalytics result = aggregator.getResult(acc);
        assertEquals(2000.0, result.twap(), 1e-6);
    }

    @Test
    void twap_twoSwaps_volumeWeighted() {
        var acc = aggregator.createAccumulator();
        // swap1: price=1000, vol=1000  → weight 1_000_000
        aggregator.add(simpleSwap(1000.0, "1000", null), acc);
        // swap2: price=3000, vol=3000  → weight 9_000_000
        aggregator.add(simpleSwap(3000.0, "3000", null), acc);

        // TWAP = (1000*1000 + 3000*3000) / (1000+3000) = 10_000_000 / 4000 = 2500
        AggregatedAnalytics result = aggregator.getResult(acc);
        assertEquals(2500.0, result.twap(), 1e-4);
    }

    // --- OHLC ---

    @Test
    void ohlc_multipleSwaps_correctValues() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(100.0, "1", null), acc);
        aggregator.add(simpleSwap(150.0, "1", null), acc);
        aggregator.add(simpleSwap(80.0,  "1", null), acc);
        aggregator.add(simpleSwap(120.0, "1", null), acc);

        AggregatedAnalytics r = aggregator.getResult(acc);
        assertEquals(100.0, r.openPrice(),  1e-6, "open should be first price");
        assertEquals(120.0, r.closePrice(), 1e-6, "close should be last price");
        assertEquals(150.0, r.highPrice(),  1e-6);
        assertEquals(80.0,  r.lowPrice(),   1e-6);
    }

    // --- Swap count ---

    @Test
    void swapCount_matchesAddedEvents() {
        var acc = aggregator.createAccumulator();
        for (int i = 0; i < 5; i++) {
            aggregator.add(simpleSwap(1.0, "100", null), acc);
        }
        assertEquals(5, aggregator.getResult(acc).swapCount());
    }

    // --- Unique traders ---

    @Test
    void uniqueTraders_distinctAddresses() {
        var acc = aggregator.createAccumulator();
        // alice→bob, carol→dave: 4 unique traders
        aggregator.add(swap("0xpair", "0xalice", "0xbob",   1.0, "100", "0", "0", "0", null, 21000, "1", 1L), acc);
        aggregator.add(swap("0xpair", "0xcarol", "0xdave",  1.0, "100", "0", "0", "0", null, 21000, "1", 2L), acc);

        assertEquals(4, aggregator.getResult(acc).uniqueTraders());
    }

    @Test
    void uniqueTraders_sameAddressBothSides_countedOnce() {
        var acc = aggregator.createAccumulator();
        // sender == recipient → only 1 unique trader
        aggregator.add(swap("0xpair", "0xalice", "0xalice", 1.0, "100", "0", "0", "0", null, 21000, "1", 1L), acc);

        assertEquals(1, aggregator.getResult(acc).uniqueTraders());
    }

    // --- Arbitrage detection ---

    @Test
    void arbitrage_senderEqualsRecipient_detected() {
        var acc = aggregator.createAccumulator();
        aggregator.add(swap("0xpair", "0xarb", "0xarb",   1.0, "100", "0", "0", "0", null, 21000, "1", 1L), acc);
        aggregator.add(swap("0xpair", "0xalice", "0xbob", 1.0, "100", "0", "0", "0", null, 21000, "1", 2L), acc);

        assertEquals(1, aggregator.getResult(acc).arbitrageCount());
    }

    // --- Repeated traders ---

    @Test
    void repeatedTraders_appearsInMultipleSwaps() {
        var acc = aggregator.createAccumulator();
        aggregator.add(swap("0xpair", "0xalice", "0xbob", 1.0, "100", "0", "0", "0", null, 21000, "1", 1L), acc);
        aggregator.add(swap("0xpair", "0xalice", "0xcarol", 1.0, "100", "0", "0", "0", null, 21000, "1", 2L), acc);
        aggregator.add(swap("0xpair", "0xalice", "0xdave", 1.0, "100", "0", "0", "0", null, 21000, "1", 3L), acc);

        assertTrue(aggregator.getResult(acc).repeatedTraders().contains("0xalice"));
    }

    @Test
    void ohlc_outOfOrderArrival_usesEventTime() {
        var acc = aggregator.createAccumulator();
        aggregator.add(swap("0xpair", "0xalice", "0xbob", 200.0, "10", "0", "0", "0", null, 21000, "1", 20L), acc);
        aggregator.add(swap("0xpair", "0xalice", "0xbob", 100.0, "10", "0", "0", "0", null, 21000, "1", 10L), acc);
        aggregator.add(swap("0xpair", "0xalice", "0xbob", 300.0, "10", "0", "0", "0", null, 21000, "1", 30L), acc);

        AggregatedAnalytics result = aggregator.getResult(acc);
        assertEquals(100.0, result.openPrice(), 1e-6);
        assertEquals(300.0, result.closePrice(), 1e-6);
    }

    // --- Gas ---

    @Test
    void gas_totalsAndAverageCorrect() {
        var acc = aggregator.createAccumulator();
        aggregator.add(swap("0xpair", "0xa", "0xb", 1.0, "100", "0", "0", "0", null, 21000L, "100", 1L), acc);
        aggregator.add(swap("0xpair", "0xa", "0xb", 1.0, "100", "0", "0", "0", null, 42000L, "200", 2L), acc);

        AggregatedAnalytics r = aggregator.getResult(acc);
        assertEquals(63000L, r.totalGasUsed());
        assertEquals("150", r.averageGasPrice()); // (100+200)/2
    }

    // --- USD volume ---

    @Test
    void volumeUSD_nullWhenNoUsdData() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1.0, "100", null), acc);

        assertNull(aggregator.getResult(acc).volumeUSD());
    }

    @Test
    void volumeUSD_summedWhenPresent() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1.0, "100", 500.0), acc);
        aggregator.add(simpleSwap(1.0, "100", 300.0), acc);

        assertEquals(800.0, aggregator.getResult(acc).volumeUSD(), 1e-4);
    }

    // --- Merge ---

    @Test
    void merge_combinesTwoAccumulators() {
        var a = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1000.0, "1000", null), a);

        var b = aggregator.createAccumulator();
        aggregator.add(simpleSwap(2000.0, "1000", null), b);

        var merged = aggregator.merge(a, b);
        AggregatedAnalytics r = aggregator.getResult(merged);

        assertEquals(2, r.swapCount());
        assertEquals(1500.0, r.twap(), 1e-4); // equal volumes → simple average
        assertEquals(1000.0, r.openPrice(), 1e-6);
        assertEquals(2000.0, r.closePrice(), 1e-6);
    }

    @Test
    void merge_emptyAccumulatorLeft_returnsRight() {
        var empty = aggregator.createAccumulator();
        var b = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1.0, "100", null), b);

        var merged = aggregator.merge(empty, b);
        assertEquals(1, aggregator.getResult(merged).swapCount());
    }

    @Test
    void merge_largestSwapWithoutUsd_usesTokenVolumeFallback() {
        var a = aggregator.createAccumulator();
        aggregator.add(swap("0xpair", "0xsmall", "0xbob", 1.0, "100", "0", "0", "0", null, 21000, "1", 1L), a);

        var b = aggregator.createAccumulator();
        aggregator.add(swap("0xpair", "0xlarge", "0xbob", 1.0, "500", "0", "0", "0", null, 21000, "1", 2L), b);

        var merged = aggregator.merge(a, b);
        AggregatedAnalytics result = aggregator.getResult(merged);
        assertEquals("500", result.largestSwapValue());
        assertEquals("0xlarge", result.largestSwapAddress());
    }

    // --- Empty result ---

    @Test
    void emptyAccumulator_returnsZeroResult() {
        var acc = aggregator.createAccumulator();
        AggregatedAnalytics r = aggregator.getResult(acc);

        assertEquals(0, r.swapCount());
        assertEquals(0.0, r.twap());
        assertNull(r.volumeUSD());
    }
}
