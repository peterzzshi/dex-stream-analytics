package com.web3analytics.functions.mev;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.models.DexEvent;
import com.web3analytics.models.SwapEvent;
import java.util.List;
import org.junit.jupiter.api.Test;

/**
 * Behavioral tests for event grouping and sorting. Focus: Sort order contracts (timestamp, then log
 * index), block grouping.
 */
class EventGrouperTest {

  // Test addresses
  private static final String POOL_ADDRESS = "0xpool";
  private static final String SENDER_ADDRESS = "0xsender";

  // Token constants
  private static final String TOKEN0_ADDRESS = "0xtoken0";
  private static final String TOKEN1_ADDRESS = "0xtoken1";
  private static final String WETH = "WETH";
  private static final String USDC = "USDC";

  // Chain constants
  private static final String CHAIN_ETHEREUM = "eip155:1";
  private static final String CHAIN_NAME = "Ethereum";

  @Test
  void sortsByTimestampThenLogIndexWhenTimestampsEqual() {
    List<DexEvent> events = List.of(
        swap(100, 5000, 30),
        swap(100, 5000, 10),
        swap(100, 5000, 20)
    );

    List<DexEvent> sorted = EventGrouper.sortByBlockAndLog(events);

    assertThat(sorted.stream().map(DexEvent::logIndex))
        .containsExactly(10, 20, 30);
  }

  @Test
  void groupsByBlockNumber() {
    List<DexEvent> events = List.of(
        swap(100, 100_000, 10),
        swap(101, 101_000, 20),
        swap(100, 100_500, 30),
        swap(102, 102_000, 40)
    );

    var grouped = EventGrouper.groupByBlock(events);

    assertThat(grouped).hasSize(3);
    assertThat(grouped.get(100L)).hasSize(2);
    assertThat(grouped.get(101L)).hasSize(1);
    assertThat(grouped.get(102L)).hasSize(1);
  }

  @Test
  void sortsAcrossDifferentTimestamps() {
    List<DexEvent> events = List.of(
        swap(100, 5000, 10),
        swap(100, 3000, 20),
        swap(100, 4000, 30)
    );

    List<DexEvent> sorted = EventGrouper.sortByBlockAndLog(events);

    assertThat(sorted.stream().map(DexEvent::blockTimestamp))
        .containsExactly(3000L, 4000L, 5000L);
  }

  @Test
  void handlesEmptyList() {
    assertThat(EventGrouper.sortByBlockAndLog(List.of())).isEmpty();
    assertThat(EventGrouper.groupByBlock(List.of())).isEmpty();
  }

  private SwapEvent swap(long block, long timestamp, int logIndex) {
    return new SwapEvent(
        "evt-" + block + "-" + logIndex,
        block,
        timestamp,
        "0xtx-" + block,
        logIndex,
        CHAIN_ETHEREUM,
        CHAIN_NAME,
        POOL_ADDRESS,
        TOKEN0_ADDRESS,
        TOKEN1_ADDRESS,
        WETH,
        USDC,
        SENDER_ADDRESS,
        "0xrecipient",
        "1000",
        "-500",
        "0",
        "0",
        2.0,
        10.0,
        21000,
        "50000000000",
        block
    );
  }
}
