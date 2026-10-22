package com.web3analytics.functions.mev;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.models.DexEvent;
import com.web3analytics.models.SwapEvent;
import java.util.List;
import org.junit.jupiter.api.Test;

/**
 * Behavioral tests for sandwich attack detection. Focus: Detection rules, not field-by-field
 * assertions.
 */
class SandwichDetectorTest {

  // Test addresses
  private static final String POOL_ADDRESS = "0xpool";
  private static final String ATTACKER_ADDRESS = "0xattacker";
  private static final String VICTIM_ADDRESS = "0xvictim";
  private static final String OTHER_ADDRESS = "0xother";

  // Token constants
  private static final String TOKEN0_ADDRESS = "0xtoken0";
  private static final String TOKEN1_ADDRESS = "0xtoken1";
  private static final String WETH = "WETH";
  private static final String USDC = "USDC";

  // Chain constants
  private static final String CHAIN_ETHEREUM = "eip155:1";
  private static final String CHAIN_NAME = "Ethereum";

  private final SandwichDetector detector = new SandwichDetector();
  private final SessionContext context = new SessionContext(
      POOL_ADDRESS, 1000, 2000, CHAIN_ETHEREUM, CHAIN_NAME, WETH, USDC, 999L
  );

  @Test
  void detectsWhenAttackerBracketsVictimWithOppositeDirectionSwaps() {
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, ATTACKER_ADDRESS, "1000", "500"),
        swapBuyToken0(100, 20, VICTIM_ADDRESS, "2000", "1000"),
        swapSellToken0(100, 30, ATTACKER_ADDRESS, "1000", "500")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).hasSize(1);
    MevCandidate candidate = candidates.getFirst();
    assertThat(candidate.patternType()).isEqualTo("SANDWICH_ATTACK");
    assertThat(candidate.attackerAddress()).isEqualTo(ATTACKER_ADDRESS);
    assertThat(candidate.victimAddresses()).containsExactly(VICTIM_ADDRESS);
  }

  @Test
  void detectsMultipleVictimsInSingleSandwich() {
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, ATTACKER_ADDRESS, "1000", "500"),
        swapBuyToken0(100, 20, VICTIM_ADDRESS, "500", "250"),
        swapBuyToken0(100, 30, OTHER_ADDRESS, "600", "300"),
        swapSellToken0(100, 40, ATTACKER_ADDRESS, "1000", "500")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).hasSize(1);
    assertThat(candidates.getFirst().victimAddresses()).containsExactlyInAnyOrder(VICTIM_ADDRESS,
        OTHER_ADDRESS);
  }

  @Test
  void ignoresWhenFrontRunAndBackRunHaveSameDirection() {
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, ATTACKER_ADDRESS, "1000", "500"),
        swapBuyToken0(100, 20, VICTIM_ADDRESS, "2000", "1000"),
        swapBuyToken0(100, 30, ATTACKER_ADDRESS, "1000", "500")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).isEmpty();
  }

  @Test
  void ignoresWhenNoVictimsBetweenAttackerSwaps() {
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, ATTACKER_ADDRESS, "1000", "500"),
        swapSellToken0(100, 20, ATTACKER_ADDRESS, "1000", "500")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).isEmpty();
  }

  @Test
  void detectsEvenWhenAttackerSwapsAreInSameTransaction() {
    // Note: Current implementation doesn't filter by transaction boundaries
    // A more sophisticated detector might ignore these cases
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, ATTACKER_ADDRESS, "0xtx-same", "1000", "500"),
        swapBuyToken0(100, 20, VICTIM_ADDRESS, "0xtx-victim", "2000", "1000"),
        swapSellToken0(100, 30, ATTACKER_ADDRESS, "0xtx-same", "1000", "500")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).hasSize(1);
    assertThat(candidates.getFirst().transactionHashes()).contains("0xtx-same", "0xtx-victim");
  }

  @Test
  void normalizesAddressesToLowercaseForComparison() {
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, "0xATTACKER", "1000", "500"),
        swapBuyToken0(100, 20, VICTIM_ADDRESS, "2000", "1000"),
        swapSellToken0(100, 30, "0xattacker", "1000", "500")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).hasSize(1);
    assertThat(candidates.getFirst().attackerAddress()).isEqualTo("0xattacker");
  }

  @Test
  void detectsMultipleSandwichesInSameBlock() {
    List<DexEvent> events = List.of(
        swapBuyToken0(100, 10, ATTACKER_ADDRESS, "1000", "500"),
        swapBuyToken0(100, 20, VICTIM_ADDRESS, "500", "250"),
        swapSellToken0(100, 30, ATTACKER_ADDRESS, "1000", "500"),
        swapBuyToken0(100, 40, OTHER_ADDRESS, "2000", "1000"),
        swapBuyToken0(100, 50, VICTIM_ADDRESS, "1000", "500"),
        swapSellToken0(100, 60, OTHER_ADDRESS, "2000", "1000")
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).hasSize(2);
  }

  @Test
  void handlesEmptyEventList() {
    List<MevCandidate> candidates = detector.detect(List.of(), context);
    assertThat(candidates).isEmpty();
  }

  @Test
  void handlesSingleEvent() {
    List<DexEvent> events = List.of(swapBuyToken0(100, 10, ATTACKER_ADDRESS, "1000", "500"));
    List<MevCandidate> candidates = detector.detect(events, context);
    assertThat(candidates).isEmpty();
  }

  /**
   * Creates a swap that BUYS token0 (sells token1). Token1 goes IN, Token0 comes OUT.
   */
  private SwapEvent swapBuyToken0(long block, int logIndex, String sender, String amount1In,
      String amount0Out) {
    return swapBuyToken0(block, logIndex, sender, "0xtx-" + block + "-" + logIndex, amount1In,
        amount0Out);
  }

  private SwapEvent swapBuyToken0(long block, int logIndex, String sender, String txHash,
      String amount1In, String amount0Out) {
    return new SwapEvent(
        "evt-" + block + "-" + logIndex,
        block,
        block * 1000,
        txHash,
        logIndex,
        CHAIN_ETHEREUM,
        CHAIN_NAME,
        POOL_ADDRESS,
        TOKEN0_ADDRESS,
        TOKEN1_ADDRESS,
        WETH,
        USDC,
        sender,
        "0xrecipient",
        "0",           // amount0In
        amount1In,     // amount1In
        amount0Out,    // amount0Out
        "0",           // amount1Out
        2.0,
        10.0,
        21000,
        "50000000000",
        block
    );
  }

  /**
   * Creates a swap that SELLS token0 (buys token1). Token0 goes IN, Token1 comes OUT.
   */
  private SwapEvent swapSellToken0(long block, int logIndex, String sender, String amount0In,
      String amount1Out) {
    return swapSellToken0(block, logIndex, sender, "0xtx-" + block + "-" + logIndex, amount0In,
        amount1Out);
  }

  private SwapEvent swapSellToken0(long block, int logIndex, String sender, String txHash,
      String amount0In, String amount1Out) {
    return new SwapEvent(
        "evt-" + block + "-" + logIndex,
        block,
        block * 1000,
        txHash,
        logIndex,
        CHAIN_ETHEREUM,
        CHAIN_NAME,
        POOL_ADDRESS,
        TOKEN0_ADDRESS,
        TOKEN1_ADDRESS,
        WETH,
        USDC,
        sender,
        "0xrecipient",
        amount0In,     // amount0In
        "0",           // amount1In
        "0",           // amount0Out
        amount1Out,    // amount1Out
        2.0,
        10.0,
        21000,
        "50000000000",
        block
    );
  }
}
