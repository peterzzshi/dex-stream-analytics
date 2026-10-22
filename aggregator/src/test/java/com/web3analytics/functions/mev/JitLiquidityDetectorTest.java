package com.web3analytics.functions.mev;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;
import java.util.List;
import org.junit.jupiter.api.Test;

/**
 * Behavioral tests for JIT liquidity detection. Focus: Mint→Swap→Burn sequence within block
 * window.
 */
class JitLiquidityDetectorTest {

  // Test addresses
  private static final String POOL_ADDRESS = "0xpool";
  private static final String PROVIDER_ADDRESS = "0xprovider";
  private static final String TRADER_ADDRESS = "0xtrader";
  private static final String OTHER_PROVIDER = "0xother";

  // Token constants
  private static final String TOKEN0_ADDRESS = "0xtoken0";
  private static final String TOKEN1_ADDRESS = "0xtoken1";
  private static final String WETH = "WETH";
  private static final String USDC = "USDC";

  // Chain constants
  private static final String CHAIN_ETHEREUM = "eip155:1";
  private static final String CHAIN_NAME = "Ethereum";

  private final JitLiquidityDetector detector = new JitLiquidityDetector();
  private final SessionContext context = new SessionContext(
      POOL_ADDRESS, 1000, 2000, CHAIN_ETHEREUM, CHAIN_NAME, WETH, USDC, 999L
  );

  @Test
  void detectsWhenSameProviderMintsLargeSwapBurnsWithinTwoBlocks() {
    List<DexEvent> events = List.of(
        new MintEvent(
            "evt-mint-100",
            100,
            100_000,
            "0xtx-mint",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "1000000",
            "2000000",
            100_000
        ),
        new SwapEvent(
            "evt-swap-101",
            101,
            101_000,
            "0xtx-swap",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            TRADER_ADDRESS,
            "0xrecipient",
            "5000",
            "-10000",
            "0",
            "0",
            2.0,
            10.0,
            21000,
            "50000000000",
            101
        ),
        new BurnEvent(
            "evt-burn-102",
            102,
            102_000,
            "0xtx-burn",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "0xrecipient",
            "1000000",
            "2000000",
            102_000
        )
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).hasSize(1);
    MevCandidate candidate = candidates.getFirst();
    assertThat(candidate.patternType()).isEqualTo("JIT_LIQUIDITY");
    assertThat(candidate.attackerAddress()).isEqualTo(PROVIDER_ADDRESS);
  }

  @Test
  void ignoresWhenProviderAddressesDontMatch() {
    List<DexEvent> events = List.of(
        new MintEvent(
            "evt-mint-100",
            100,
            100_000,
            "0xtx-mint",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "1000000",
            "2000000",
            100_000
        ),
        new SwapEvent(
            "evt-swap-101",
            101,
            101_000,
            "0xtx-swap",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            TRADER_ADDRESS,
            "0xrecipient",
            "5000",
            "-10000",
            "0",
            "0",
            2.0,
            10.0,
            21000,
            "50000000000",
            101
        ),
        new BurnEvent(
            "evt-burn-102",
            102,
            102_000,
            "0xtx-burn",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            OTHER_PROVIDER,
            "0xrecipient",
            "1000000",
            "2000000",
            102_000
        )
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).isEmpty();
  }

  @Test
  void ignoresWhenBurnIsMoreThanTwoBlocksAfterMint() {
    List<DexEvent> events = List.of(
        new MintEvent(
            "evt-mint-100",
            100,
            100_000,
            "0xtx-mint",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "1000000",
            "2000000",
            100_000
        ),
        new SwapEvent(
            "evt-swap-101",
            101,
            101_000,
            "0xtx-swap",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            TRADER_ADDRESS,
            "0xrecipient",
            "5000",
            "-10000",
            "0",
            "0",
            2.0,
            10.0,
            21000,
            "50000000000",
            101
        ),
        new BurnEvent(
            "evt-burn-103",
            103,
            103_000,
            "0xtx-burn",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "0xrecipient",
            "1000000",
            "2000000",
            103_000
        )
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).isEmpty();
  }

  @Test
  void ignoresWhenNoSwapBetweenMintAndBurn() {
    List<DexEvent> events = List.of(
        new MintEvent(
            "evt-mint-100",
            100,
            100_000,
            "0xtx-mint",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "1000000",
            "2000000",
            100_000
        ),
        new BurnEvent(
            "evt-burn-102",
            102,
            102_000,
            "0xtx-burn",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "0xrecipient",
            "1000000",
            "2000000",
            102_000
        )
    );

    List<MevCandidate> candidates = detector.detect(events, context);

    assertThat(candidates).isEmpty();
  }

  @Test
  void handlesEmptyEventList() {
    List<MevCandidate> candidates = detector.detect(List.of(), context);
    assertThat(candidates).isEmpty();
  }

  @Test
  void handlesOnlyMints() {
    List<DexEvent> events = List.of(
        new MintEvent(
            "evt-mint-100",
            100,
            100_000,
            "0xtx-mint",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            USDC,
            PROVIDER_ADDRESS,
            "1000000",
            "2000000",
            100_000
        )
    );
    List<MevCandidate> candidates = detector.detect(events, context);
    assertThat(candidates).isEmpty();
  }
}
