package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.models.AggregatedAnalytics;
import com.web3analytics.models.SwapEvent;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class SwapAggregatorTest {

  // Test addresses
  private static final String POOL_ADDRESS = "0xpair";
  private static final String ALICE = "0xalice";
  private static final String BOB = "0xbob";
  private static final String CAROL = "0xcarol";
  private static final String DAVE = "0xdave";
  private static final String ARB = "0xarb";
  private static final String TRADER_A = "0xa";
  private static final String TRADER_B = "0xb";
  private static final String SMALL_TRADER = "0xsmall";
  private static final String LARGE_TRADER = "0xlarge";

  // Token constants
  private static final String TOKEN0_ADDRESS = "0xtoken0";
  private static final String TOKEN1_ADDRESS = "0xtoken1";
  private static final String WETH = "WETH";
  private static final String USDC = "USDC";

  // Chain constants
  private static final String CHAIN_ETHEREUM = "eip155:1";
  private static final String CHAIN_NAME = "Ethereum";

  // Default values
  private static final long DEFAULT_BLOCK = 100L;
  private static final long DEFAULT_GAS_USED = 21000L;
  private static final String DEFAULT_GAS_PRICE = "50000000000";
  private static final String DEFAULT_TX_HASH = "0xtx";

    private SwapAggregator aggregator;

    @BeforeEach
    void setUp() {
        aggregator = new SwapAggregator();
    }

  private static org.assertj.core.data.Offset<Double> within(double offset) {
    return org.assertj.core.data.Offset.offset(offset);
    }

    @Test
    void twapForSingleSwapEqualsPrice() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(2000.0, "1000", null), acc);

        AggregatedAnalytics result = aggregator.getResult(acc);
      assertThat(result.twap()).isCloseTo(2000.0, within(1e-6));
    }

    @Test
    void twapForMultipleSwapsIsVolumeWeighted() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1000.0, "1000", null), acc);
        aggregator.add(simpleSwap(3000.0, "3000", null), acc);

        // TWAP = (1000*1000 + 3000*3000) / (1000+3000) = 10_000_000 / 4000 = 2500
        AggregatedAnalytics result = aggregator.getResult(acc);
      assertThat(result.twap()).isCloseTo(2500.0, within(1e-4));
    }

    @Test
    void ohlcTracksOpenHighLowCloseCorrectly() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(ALICE, BOB, 100.0, "1", null, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE, 1L),
          acc);
      aggregator.add(swap(ALICE, BOB, 150.0, "1", null, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE, 2L),
          acc);
      aggregator.add(swap(ALICE, BOB, 80.0, "1", null, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE, 3L),
          acc);
      aggregator.add(swap(ALICE, BOB, 120.0, "1", null, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE, 4L),
          acc);

        AggregatedAnalytics r = aggregator.getResult(acc);
      assertThat(r.openPrice()).isCloseTo(100.0, within(1e-6));
      assertThat(r.closePrice()).isCloseTo(120.0, within(1e-6));
      assertThat(r.highPrice()).isCloseTo(150.0, within(1e-6));
      assertThat(r.lowPrice()).isCloseTo(80.0, within(1e-6));
    }

    @Test
    void swapCountMatchesAddedEvents() {
        var acc = aggregator.createAccumulator();
        for (int i = 0; i < 5; i++) {
            aggregator.add(simpleSwap(1.0, "100", null), acc);
        }
      assertThat(aggregator.getResult(acc).swapCount()).isEqualTo(5);
    }

    @Test
    void uniqueTradersCountsDistinctAddresses() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(ALICE, BOB, 1.0, "100", null, DEFAULT_GAS_USED, "1", 1L), acc);
      aggregator.add(swap(CAROL, DAVE, 1.0, "100", null, DEFAULT_GAS_USED, "1", 2L), acc);

      assertThat(aggregator.getResult(acc).uniqueTraders()).isEqualTo(4);
    }

    @Test
    void uniqueTradersCountsSameAddressOnce() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(ALICE, ALICE, 1.0, "100", null, DEFAULT_GAS_USED, "1", 1L), acc);

      assertThat(aggregator.getResult(acc).uniqueTraders()).isEqualTo(1);
    }

    @Test
    void arbitrageDetectedWhenSenderEqualsRecipient() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(ARB, ARB, 1.0, "100", null, DEFAULT_GAS_USED, "1", 1L), acc);
      aggregator.add(swap(ALICE, BOB, 1.0, "100", null, DEFAULT_GAS_USED, "1", 2L), acc);

      assertThat(aggregator.getResult(acc).arbitrageCount()).isEqualTo(1);
    }

    @Test
    void repeatedTradersIdentifiesTraderInMultipleSwaps() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(ALICE, BOB, 1.0, "100", null, DEFAULT_GAS_USED, "1", 1L), acc);
      aggregator.add(swap(ALICE, CAROL, 1.0, "100", null, DEFAULT_GAS_USED, "1", 2L), acc);
      aggregator.add(swap(ALICE, DAVE, 1.0, "100", null, DEFAULT_GAS_USED, "1", 3L), acc);

      assertThat(aggregator.getResult(acc).repeatedTraders()).contains(ALICE);
    }

    @Test
    void ohlcUsesEventTimeNotArrivalTime() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(ALICE, BOB, 200.0, "10", null, DEFAULT_GAS_USED, "1", 20L), acc);
      aggregator.add(swap(ALICE, BOB, 100.0, "10", null, DEFAULT_GAS_USED, "1", 10L), acc);
      aggregator.add(swap(ALICE, BOB, 300.0, "10", null, DEFAULT_GAS_USED, "1", 30L), acc);

        AggregatedAnalytics result = aggregator.getResult(acc);
      assertThat(result.openPrice()).isCloseTo(100.0, within(1e-6));
      assertThat(result.closePrice()).isCloseTo(300.0, within(1e-6));
    }

    @Test
    void gasMetricsCalculateTotalsAndAverages() {
        var acc = aggregator.createAccumulator();
      aggregator.add(swap(TRADER_A, TRADER_B, 1.0, "100", null, 21000L, "100", 1L), acc);
      aggregator.add(swap(TRADER_A, TRADER_B, 1.0, "100", null, 42000L, "200", 2L), acc);

        AggregatedAnalytics r = aggregator.getResult(acc);
      assertThat(r.totalGasUsed()).isEqualTo(63000L);
      assertThat(r.averageGasPrice()).isEqualTo("150");
    }

    @Test
    void volumeUsdIsNullWhenNoUsdData() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1.0, "100", null), acc);

      assertThat(aggregator.getResult(acc).volumeUSD()).isNull();
    }

    @Test
    void volumeUsdSumsWhenPresent() {
        var acc = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1.0, "100", 500.0), acc);
        aggregator.add(simpleSwap(1.0, "100", 300.0), acc);

      assertThat(aggregator.getResult(acc).volumeUSD()).isCloseTo(800.0, within(1e-4));
    }

    @Test
    void mergeCombinesTwoAccumulators() {
        var a = aggregator.createAccumulator();
      aggregator.add(
          swap(ALICE, BOB, 1000.0, "1000", null, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE, 1L), a);

        var b = aggregator.createAccumulator();
      aggregator.add(
          swap(ALICE, BOB, 2000.0, "1000", null, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE, 2L), b);

        var merged = aggregator.merge(a, b);
        AggregatedAnalytics r = aggregator.getResult(merged);

      assertThat(r.swapCount()).isEqualTo(2);
      assertThat(r.twap()).isCloseTo(1500.0, within(1e-4));
      assertThat(r.openPrice()).isCloseTo(1000.0, within(1e-6));
      assertThat(r.closePrice()).isCloseTo(2000.0, within(1e-6));
    }

    @Test
    void mergeWithEmptyLeftReturnsRight() {
        var empty = aggregator.createAccumulator();
        var b = aggregator.createAccumulator();
        aggregator.add(simpleSwap(1.0, "100", null), b);

        var merged = aggregator.merge(empty, b);
      assertThat(aggregator.getResult(merged).swapCount()).isEqualTo(1);
    }

    @Test
    void mergeTracksLargestSwapWithoutUsd() {
        var a = aggregator.createAccumulator();
      aggregator.add(swap(SMALL_TRADER, BOB, 1.0, "100", null, DEFAULT_GAS_USED, "1", 1L), a);

        var b = aggregator.createAccumulator();
      aggregator.add(swap(LARGE_TRADER, BOB, 1.0, "500", null, DEFAULT_GAS_USED, "1", 2L), b);

        var merged = aggregator.merge(a, b);
        AggregatedAnalytics result = aggregator.getResult(merged);
      assertThat(result.largestSwapValue()).isEqualTo("500");
      assertThat(result.largestSwapAddress()).isEqualTo(LARGE_TRADER);
    }

    @Test
    void emptyAccumulatorReturnsZeroResult() {
        var acc = aggregator.createAccumulator();
        AggregatedAnalytics r = aggregator.getResult(acc);

      assertThat(r.swapCount()).isZero();
      assertThat(r.twap()).isZero();
      assertThat(r.volumeUSD()).isNull();
    }

  private SwapEvent simpleSwap(double price, String amount0In, Double volumeUSD) {
    return swap(ALICE, BOB, price, amount0In, volumeUSD, DEFAULT_GAS_USED, DEFAULT_GAS_PRICE,
        1700000000L);
  }

  private SwapEvent swap(String sender, String recipient, double price, String amount0In,
      Double volumeUSD, long gasUsed, String gasPrice, long blockTimestamp) {
    return new SwapEvent(
        "evt-" + System.nanoTime(),
        DEFAULT_BLOCK,
        blockTimestamp,
        DEFAULT_TX_HASH,
        0,
        CHAIN_ETHEREUM,
        CHAIN_NAME,
        POOL_ADDRESS,
        TOKEN0_ADDRESS,
        TOKEN1_ADDRESS,
        WETH,
        USDC,
        sender,
        recipient,
        amount0In,
        "0",
        "0",
        "0",
        price,
        volumeUSD,
        gasUsed,
        gasPrice,
        blockTimestamp
    );
  }
}
