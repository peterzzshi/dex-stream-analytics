package com.web3analytics.functions.mev;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;
import com.web3analytics.models.TransferEvent;
import java.util.List;
import org.junit.jupiter.api.Test;

/**
 * Behavioral tests for metadata extraction from event streams. Focus: Correct chain/symbol
 * extraction rules, handling missing/conflicting data.
 */
class MetadataExtractorTest {

  // Test addresses
  private static final String POOL_ADDRESS = "0xpair";
  private static final String SENDER_ADDRESS = "0xsender";
  private static final String RECIPIENT_ADDRESS = "0xrecipient";
  private static final String FROM_ADDRESS = "0xfrom";
  private static final String TO_ADDRESS = "0xto";

  // Token constants
  private static final String TOKEN0_ADDRESS = "0xtoken0";
  private static final String TOKEN1_ADDRESS = "0xtoken1";
  private static final String WETH = "WETH";
  private static final String USDC = "USDC";
  private static final String WMATIC = "WMATIC";
  private static final String DAI = "DAI";

  // Chain constants
  private static final String CHAIN_ETHEREUM = "eip155:1";
  private static final String CHAIN_POLYGON = "eip155:137";
  private static final String CHAIN_NAME_ETHEREUM = "Ethereum";
  private static final String CHAIN_NAME_POLYGON = "Polygon";

  @Test
  void extractsChainFromFirstEvent() {
    List<DexEvent> events = List.of(
        swap(100, CHAIN_ETHEREUM, CHAIN_NAME_ETHEREUM, WMATIC, USDC),
        swap(101, CHAIN_POLYGON, CHAIN_NAME_POLYGON, WETH, DAI)
    );

    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, events, 999L);

    assertThat(ctx.chain()).isEqualTo(CHAIN_ETHEREUM);
    assertThat(ctx.chainName()).isEqualTo(CHAIN_NAME_ETHEREUM);
  }

  @Test
  void extractsFirstNonNullSymbolsFromSwapEvents() {
    List<DexEvent> events = List.of(
        swapWithSymbols(100, null, USDC),
        swapWithSymbols(101, WMATIC, null),
        swapWithSymbols(102, "OVERRIDE", "OVERRIDE")  // should be ignored
    );

    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, events, 999L);

    assertThat(ctx.token0Symbol()).isEqualTo(WMATIC);
    assertThat(ctx.token1Symbol()).isEqualTo(USDC);
  }

  @Test
  void extractsSymbolsFromMintEventsWhenSwapsAbsent() {
    List<DexEvent> events = List.of(
        new MintEvent(
            "evt-mint-100",
            100,
            100_000,
            "0xtx-100",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME_ETHEREUM,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WMATIC,
            USDC,
            SENDER_ADDRESS,
            "1000000",
            "2000000",
            100_000
        )
    );

    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, events, 999L);

    assertThat(ctx.token0Symbol()).isEqualTo(WMATIC);
    assertThat(ctx.token1Symbol()).isEqualTo(USDC);
  }

  @Test
  void extractsSymbolsFromBurnEventsWhenOthersAbsent() {
    List<DexEvent> events = List.of(
        new BurnEvent(
            "evt-burn-100",
            100,
            100_000,
            "0xtx-100",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME_ETHEREUM,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            WETH,
            DAI,
            SENDER_ADDRESS,
            RECIPIENT_ADDRESS,
            "1000000",
            "2000000",
            100_000
        )
    );

    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, events, 999L);

    assertThat(ctx.token0Symbol()).isEqualTo(WETH);
    assertThat(ctx.token1Symbol()).isEqualTo(DAI);
  }

  @Test
  void ignoresTransferEventsForSymbolExtraction() {
    List<DexEvent> events = List.of(
        new TransferEvent(
            "evt-transfer-100",
            100,
            100_000,
            "0xtx-100",
            0,
            CHAIN_ETHEREUM,
            CHAIN_NAME_ETHEREUM,
            POOL_ADDRESS,
            TOKEN0_ADDRESS,
            TOKEN1_ADDRESS,
            "IGNORED0",
            "IGNORED1",
            FROM_ADDRESS,
            TO_ADDRESS,
            "1000000",
            100_000
        ),
        swap(101, CHAIN_ETHEREUM, CHAIN_NAME_ETHEREUM, WMATIC, USDC)
    );

    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, events, 999L);

    assertThat(ctx.token0Symbol()).isEqualTo(WMATIC);
    assertThat(ctx.token1Symbol()).isEqualTo(USDC);
  }

  @Test
  void stopsEarlyOnceBothSymbolsFound() {
    List<DexEvent> events = List.of(
        swap(100, CHAIN_ETHEREUM, CHAIN_NAME_ETHEREUM, WMATIC, USDC),
        swap(101, CHAIN_POLYGON, CHAIN_NAME_POLYGON, "OVERRIDE0", "OVERRIDE1")
    );

    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, events, 999L);

    assertThat(ctx.token0Symbol()).isEqualTo(WMATIC);
    assertThat(ctx.token1Symbol()).isEqualTo(USDC);
  }

  @Test
  void handlesEmptyEventList() {
    SessionContext ctx = MetadataExtractor.buildContext(POOL_ADDRESS, 0, 10000, List.of(), 999L);

    assertThat(ctx.chain()).isNull();
    assertThat(ctx.chainName()).isNull();
    assertThat(ctx.token0Symbol()).isNull();
    assertThat(ctx.token1Symbol()).isNull();
  }

  private SwapEvent swap(long block, String chain, String chainName, String t0, String t1) {
    return swapWithSymbols(block, t0, t1, chain, chainName);
  }

  private SwapEvent swapWithSymbols(long block, String t0Symbol, String t1Symbol) {
    return swapWithSymbols(block, t0Symbol, t1Symbol, CHAIN_ETHEREUM, CHAIN_NAME_ETHEREUM);
  }

  private SwapEvent swapWithSymbols(long block, String t0Symbol, String t1Symbol, String chain,
      String chainName) {
    return new SwapEvent(
        "evt-swap-" + block,
        block,
        block * 1000,
        "0xtx-" + block,
        0,
        chain,
        chainName,
        POOL_ADDRESS,
        TOKEN0_ADDRESS,
        TOKEN1_ADDRESS,
        t0Symbol,
        t1Symbol,
        SENDER_ADDRESS,
        RECIPIENT_ADDRESS,
        "1000",
        "0",
        "0",
        "200",
        2.0,
        10.0,
        21000,
        "50000000000",
        block
    );
  }
}
