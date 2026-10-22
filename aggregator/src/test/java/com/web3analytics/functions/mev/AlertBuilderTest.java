package com.web3analytics.functions.mev;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.models.MevAlert;
import java.util.List;
import java.util.Set;
import org.junit.jupiter.api.Test;

/**
 * Behavioral tests for MEV alert construction. Focus: Severity classification thresholds,
 * transaction hash ordering, field propagation.
 */
class AlertBuilderTest {

  // Test addresses
  private static final String POOL_ADDRESS = "0xpair";
  private static final String ATTACKER_ADDRESS = "0xattacker";
  private static final String VICTIM_ADDRESS = "0xvictim";

  // Token constants
  private static final String WMATIC = "WMATIC";
  private static final String USDC = "USDC";

  // Chain constants
  private static final String CHAIN_ETHEREUM = "eip155:1";
  private static final String CHAIN_NAME = "Ethereum";

  private final SessionContext context = new SessionContext(
      POOL_ADDRESS, 1000, 2000, CHAIN_ETHEREUM, CHAIN_NAME, WMATIC, USDC, 999L
  );

  @Test
  void classifiesHighSeverityWhenProfitExceedsOneHundred() {
    MevCandidate candidate = candidate(150.0, Set.of("0xtx1"));

    MevAlert alert = AlertBuilder.build(candidate, context);

    assertThat(alert.severity()).isEqualTo("HIGH");
  }

  @Test
  void classifiesMediumSeverityWhenProfitBetweenTenAndOneHundred() {
    MevAlert alert = AlertBuilder.build(candidate(50.0, Set.of("0xtx1")), context);

    assertThat(alert.severity()).isEqualTo("MEDIUM");
  }

  @Test
  void classifiesLowSeverityWhenProfitAtOrBelowTen() {
    MevAlert alert = AlertBuilder.build(candidate(5.0, Set.of("0xtx1")), context);

    assertThat(alert.severity()).isEqualTo("LOW");
  }

  @Test
  void classifiesBoundaryAtExactlyTen() {
    assertThat(AlertBuilder.build(candidate(10.0, Set.of("0x")), context).severity())
        .isEqualTo("LOW");
  }

  @Test
  void classifiesBoundaryAtExactlyOneHundred() {
    assertThat(AlertBuilder.build(candidate(100.0, Set.of("0x")), context).severity())
        .isEqualTo("MEDIUM");
  }

  @Test
  void sortsTransactionHashesDeterministically() {
    MevCandidate candidate = candidate(50.0, Set.of("0xzzz", "0xaaa", "0xmmm"));

    MevAlert alert = AlertBuilder.build(candidate, context);

    assertThat(alert.involvedTransactions()).containsExactly("0xaaa", "0xmmm", "0xzzz");
  }

  @Test
  void propagatesContextFieldsToAlert() {
    MevAlert alert = AlertBuilder.build(candidate(50.0, Set.of("0xtx1")), context);

    assertThat(alert.pairAddress()).isEqualTo(POOL_ADDRESS);
    assertThat(alert.windowStart()).isEqualTo(1000);
    assertThat(alert.windowEnd()).isEqualTo(2000);
    assertThat(alert.chain()).isEqualTo(CHAIN_ETHEREUM);
    assertThat(alert.chainName()).isEqualTo(CHAIN_NAME);
    assertThat(alert.token0Symbol()).isEqualTo(WMATIC);
    assertThat(alert.token1Symbol()).isEqualTo(USDC);
    assertThat(alert.detectedAt()).isEqualTo(999);
  }

  private MevCandidate candidate(double profitEstimate, Set<String> txHashes) {
    return new MevCandidate(
        "SANDWICH_ATTACK",
        100,
        ATTACKER_ADDRESS,
        List.of(VICTIM_ADDRESS),
        profitEstimate,
        3,
        3,
        txHashes,
        "Sandwich: " + ATTACKER_ADDRESS + " front-ran 1 victim(s) in block 100"
    );
  }
}
