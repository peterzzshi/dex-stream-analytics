package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.functions.mev.MevCandidate;
import com.web3analytics.functions.mev.SessionContext;
import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MevAlert;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;
import java.util.List;
import java.util.Set;
import org.junit.jupiter.api.Test;

/**
 * Behavioral tests for MEV detection function. Focus: Does the orchestration correctly apply
 * per-block and session-wide detectors?
 */
class MevDetectionFunctionTest {

  private static final String PAIR = "0xpair";
    private static final String ATTACKER = "0xattacker";
    private static final String VICTIM = "0xvictim";

  private final MevDetectionFunction function = new MevDetectionFunction();

    @Test
    void detectsSandwichAttackFromPerBlockDetector() {
        // Block 100: attacker buys token0, victim buys token0, attacker sells token0
        List<DexEvent> events = List.of(
            swap(100, 0, ATTACKER, "0", "5000", "1000", "0"),
            swap(100, 1, VICTIM, "0", "3000", "500", "0"),
            swap(100, 2, ATTACKER, "1000", "0", "0", "5200")
        );

      List<MevAlert> alerts = function.detect(PAIR, 0, 10000, events, 999L);

      assertThat(alerts).hasSize(1);
      assertThat(alerts.getFirst().alertType()).isEqualTo("SANDWICH_ATTACK");
      assertThat(alerts.getFirst().victimAddresses()).containsExactly(VICTIM);
    }

    @Test
    void detectsJitLiquidityFromSessionDetector() {
      // Mint at block 100, large swap at block 101, burn at block 102 by same provider
      String provider = "0xlprovider";
        List<DexEvent> events = List.of(
            mint(100, 0, provider),
            swap(101, 0, "0xtrader", "0", "50000", "10000", "0"),
            burn(102, 0, provider)
        );

      List<MevAlert> alerts = function.detect(PAIR, 0, 10000, events, 999L);

      assertThat(alerts).hasSize(1);
      assertThat(alerts.getFirst().alertType()).isEqualTo("JIT_LIQUIDITY");
      assertThat(alerts.getFirst().attackerAddress()).isEqualTo(provider);
    }

    @Test
    void appliesAllPerBlockDetectorsToConcurrentBlocks() {
      // Two blocks, each with sandwich pattern
        List<DexEvent> events = List.of(
            swap(100, 0, ATTACKER, "0", "5000", "1000", "0"),
            swap(100, 1, VICTIM, "0", "3000", "500", "0"),
            swap(100, 2, ATTACKER, "1000", "0", "0", "5200"),
            swap(101, 0, ATTACKER, "0", "4000", "800", "0"),
            swap(101, 1, "0xvictim2", "0", "2000", "400", "0"),
            swap(101, 2, ATTACKER, "800", "0", "0", "4100")
        );

      List<MevAlert> alerts = function.detect(PAIR, 0, 10000, events, 999L);

      assertThat(alerts).hasSize(2);
    }

    @Test
    void canSubstituteCustomDetectorViaConstructor() {
      // Inject a detector that always flags any session with a swap
      java.util.function.BiFunction<List<DexEvent>, SessionContext, List<MevCandidate>> alwaysDetect =
          (evts, ctx) -> evts.stream()
              .filter(SwapEvent.class::isInstance)
              .findFirst()
              .map(e -> new MevCandidate(
                  "CUSTOM_PATTERN", e.blockNumber(), "0xcustom", List.of(),
                  0.0, 1, 1, Set.of(e.transactionHash()), "Custom detection"
              ))
              .stream()
              .toList();

      var functionWithCustomDetector = new MevDetectionFunction(
          List.of(),  // no per-block
          List.of(alwaysDetect)  // custom session-wide
        );

      List<MevAlert> alerts = functionWithCustomDetector.detect(
          PAIR, 0, 10000,
          List.of(swap(100, 0, "0xsender", "0", "1000", "200", "0")),
          999L
      );

      assertThat(alerts).hasSize(1);
      assertThat(alerts.getFirst().alertType()).isEqualTo("CUSTOM_PATTERN");
    }

    @Test
    void emptyEventsProduceNoAlerts() {
      assertThat(function.detect(PAIR, 0, 10000, List.of(), 999L)).isEmpty();
    }

    @Test
    void eventsWithNoMevPatternsProduceNoAlerts() {
      // Single swap, no sandwich or JIT liquidity
        List<DexEvent> events = List.of(
            swap(100, 0, "0xtrader", "0", "1000", "200", "0")
        );

      assertThat(function.detect(PAIR, 0, 10000, events, 999L)).isEmpty();
    }

    @Test
    void sortsEventsBeforeDetection() {
      // Events out of order
        List<DexEvent> events = List.of(
            swap(100, 2, ATTACKER, "1000", "0", "0", "5200"),
            swap(100, 0, ATTACKER, "0", "5000", "1000", "0"),
            swap(100, 1, VICTIM, "0", "3000", "500", "0")
        );

      List<MevAlert> alerts = function.detect(PAIR, 0, 10000, events, 999L);

      // Should still detect sandwich despite out-of-order input
      assertThat(alerts).hasSize(1);
      assertThat(alerts.getFirst().alertType()).isEqualTo("SANDWICH_ATTACK");
    }

    private SwapEvent swap(long block, int logIndex, String sender,
        String a0In, String a1In, String a0Out, String a1Out) {
        return new SwapEvent(
            "evt-" + block + "-" + logIndex, block, block * 1000, "0xtx-" + block + "-" + logIndex,
            logIndex, "eip155:1", "Ethereum", PAIR, "0xtoken0", "0xtoken1",
            "WMATIC", "USDC", sender, "0xrecipient",
            a0In, a1In, a0Out, a1Out, 2.0, 10.0, 21000, "50000000000", block
        );
    }

  private MintEvent mint(long block, int logIndex, String sender) {
        return new MintEvent(
            "evt-mint-" + block, block, block * 1000, "0xtx-mint-" + block,
            logIndex, "eip155:1", "Ethereum", PAIR, "0xtoken0", "0xtoken1",
            "WMATIC", "USDC", sender, "1000000", "2000000", block * 1000
        );
    }

  private BurnEvent burn(long block, int logIndex, String sender) {
        return new BurnEvent(
            "evt-burn-" + block, block, block * 1000, "0xtx-burn-" + block,
            logIndex, "eip155:1", "Ethereum", PAIR, "0xtoken0", "0xtoken1",
            "WMATIC", "USDC", sender, "0xrecipient", "1000000", "2000000", block * 1000
        );
    }
}
