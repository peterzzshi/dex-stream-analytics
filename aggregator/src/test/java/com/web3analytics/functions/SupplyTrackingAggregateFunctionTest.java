package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.bridge.BridgeProtocol;
import com.web3analytics.bridge.BridgeSupplyEvent;
import com.web3analytics.bridge.SupplyOperation;
import com.web3analytics.model.SupplyTrackingAccumulator;
import com.web3analytics.model.TriState;
import java.math.BigDecimal;
import java.math.BigInteger;
import net.jqwik.api.Arbitraries;
import net.jqwik.api.Arbitrary;
import net.jqwik.api.ForAll;
import net.jqwik.api.Property;
import net.jqwik.api.Provide;
import org.junit.jupiter.api.Test;

class SupplyTrackingAggregateFunctionTest {

  private final SupplyTrackingAggregateFunction function =
      new SupplyTrackingAggregateFunction();

  private static BridgeSupplyEvent mintEvent(final String bridge, final String chain,
      final String amount, final String symbol) {
    return BridgeSupplyEvent.newBuilder()
        .setEventId("evt-mint-" + amount)
        .setTransactionHash("0xabc")
        .setBridgeAddress(bridge)
        .setChain(chain)
        .setChainName("ethereum")
        .setOperation(SupplyOperation.mint)
        .setProtocol(BridgeProtocol.wormhole)
        .setAmount(new BigDecimal(new BigInteger(amount)))
        .setWrappedTokenSymbol(symbol)
        .setBlockNumber(1000L)
        .setBlockTimestamp(1234567890L)
        .setLogIndex(0)
        .setGasUsed(50000L)
        .setGasPrice("100000000")
        .setEventTimestamp(1234567890L)
        .setRecipient("0xrecipient")
        .setWrappedToken("0xtoken")
        .build();
  }

  private static BridgeSupplyEvent burnEvent(final String bridge, final String chain,
      final String amount, final String symbol) {
    return BridgeSupplyEvent.newBuilder()
        .setEventId("evt-burn-" + amount)
        .setTransactionHash("0xabc")
        .setBridgeAddress(bridge)
        .setChain(chain)
        .setChainName("ethereum")
        .setOperation(SupplyOperation.burn)
        .setProtocol(BridgeProtocol.wormhole)
        .setAmount(new BigDecimal(new BigInteger(amount)))
        .setWrappedTokenSymbol(symbol)
        .setBlockNumber(1000L)
        .setBlockTimestamp(1234567890L)
        .setLogIndex(0)
        .setGasUsed(50000L)
        .setGasPrice("100000000")
        .setEventTimestamp(1234567890L)
        .setRecipient("0xrecipient")
        .setWrappedToken("0xtoken")
        .build();
  }

  @Provide
  static Arbitrary<BridgeSupplyEvent[]> mintBurnSequence() {
    return Arbitraries.integers().between(1, 1000)
        .flatMap(amount -> Arbitraries.of(
            SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", amount.toString(),
                "WETH"),
            SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", amount.toString(),
                "WETH")
        ))
        .list().ofMinSize(3).ofMaxSize(10)
        .map(list -> list.toArray(new BridgeSupplyEvent[0]));
  }

  @Test
  void mergeDefersUndefinedTokenSymbolToOtherAccumulator() {
    final SupplyTrackingAccumulator a = this.function.createAccumulator();
    final SupplyTrackingAccumulator b = this.function.createAccumulator();

    a.tokenSymbol = TriState.undefined();
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), b);

    final SupplyTrackingAccumulator merged = this.function.merge(a, b);

    assertThat(merged.tokenSymbol.get()).isEqualTo("WETH");
  }

  @Test
  void mintIncrementsCountAndSumsAmount() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();
    final BridgeSupplyEvent mint = SupplyTrackingAggregateFunctionTest.mintEvent("0x123",
        "ethereum", "100", "WETH");

    this.function.add(mint, acc);

    assertThat(acc.mintCount).isEqualTo(1);
    assertThat(acc.burnCount).isZero();
    assertThat(acc.totalMinted).isEqualTo(new BigInteger("100"));
    assertThat(acc.totalBurned).isEqualTo(BigInteger.ZERO);
  }

  @Test
  void burnIncrementsCountAndSumsAmount() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();
    final BridgeSupplyEvent burn = SupplyTrackingAggregateFunctionTest.burnEvent("0x123",
        "ethereum", "200", "WETH");

    this.function.add(burn, acc);

    assertThat(acc.mintCount).isZero();
    assertThat(acc.burnCount).isEqualTo(1);
    assertThat(acc.totalMinted).isEqualTo(BigInteger.ZERO);
    assertThat(acc.totalBurned).isEqualTo(new BigInteger("200"));
  }

  @Test
  void firstEventPopulatesMetadata() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();
    final BridgeSupplyEvent mint = SupplyTrackingAggregateFunctionTest.mintEvent("0x123",
        "ethereum", "100", "WETH");

    this.function.add(mint, acc);

    assertThat(acc.bridgeAddress).isEqualTo("0x123");
    assertThat(acc.chain).isEqualTo("ethereum");
    assertThat(acc.protocol).isEqualTo("wormhole");
  }

  @Test
  void firstEventCapturesTokenSymbol() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();
    final BridgeSupplyEvent mint = SupplyTrackingAggregateFunctionTest.mintEvent("0x123",
        "ethereum", "100", "WETH");

    this.function.add(mint, acc);

    assertThat(acc.tokenSymbol.isDefined()).isTrue();
    assertThat(acc.tokenSymbol.get()).isEqualTo("WETH");
  }

  @Test
  void subsequentEventsDoNotOverwriteTokenSymbol() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "200", "WBTC"), acc);

    assertThat(acc.tokenSymbol.get()).isEqualTo("WETH");
    assertThat(acc.mintCount).isEqualTo(2);
  }

  @Test
  void nullTokenSymbolIsCapturedAsNull() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();
    final BridgeSupplyEvent mintWithNull = SupplyTrackingAggregateFunctionTest.mintEvent("0x123",
        "ethereum", "100", null);

    this.function.add(mintWithNull, acc);

    assertThat(acc.tokenSymbol.isNull()).isTrue();
  }

  @Test
  void multipleMintsAccumulateAmount() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "250", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "50", "WETH"), acc);

    assertThat(acc.mintCount).isEqualTo(3);
    assertThat(acc.totalMinted).isEqualTo(new BigInteger("400"));
  }

  @Test
  void multipleBurnsAccumulateAmount() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "100", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "150", "WETH"), acc);

    assertThat(acc.burnCount).isEqualTo(2);
    assertThat(acc.totalBurned).isEqualTo(new BigInteger("250"));
  }

  @Test
  void mixedMintsAndBurns() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "50", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "200", "WETH"), acc);

    assertThat(acc.mintCount).isEqualTo(2);
    assertThat(acc.burnCount).isEqualTo(1);
    assertThat(acc.totalMinted).isEqualTo(new BigInteger("300"));
    assertThat(acc.totalBurned).isEqualTo(new BigInteger("50"));
  }

  @Test
  void getResultCalculatesNetMinted() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "1000", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "300", "WETH"), acc);

    final SupplyTrackingAccumulator result = this.function.getResult(acc);

    assertThat(result.netMinted).isEqualTo(new BigInteger("700"));
    assertThat(result.totalMinted).isEqualTo(new BigInteger("1000"));
    assertThat(result.totalBurned).isEqualTo(new BigInteger("300"));
  }

  @Test
  void getResultNetMintedCanBeNegative() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "500", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "800", "WETH"), acc);

    final SupplyTrackingAccumulator result = this.function.getResult(acc);

    assertThat(result.netMinted).isEqualTo(new BigInteger("-300"));
  }

  @Test
  void getResultNetMintedIsZeroWhenEqual() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "500", "WETH"), acc);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "500", "WETH"), acc);

    final SupplyTrackingAccumulator result = this.function.getResult(acc);

    assertThat(result.netMinted).isEqualTo(BigInteger.ZERO);
  }

  @Test
  void getResultCreatesNewInstance() {
    final SupplyTrackingAccumulator acc = this.function.createAccumulator();
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), acc);

    final SupplyTrackingAccumulator result = this.function.getResult(acc);

    assertThat(result).isNotSameAs(acc);
    assertThat(result.mintCount).isEqualTo(acc.mintCount);
    assertThat(result.tokenSymbol).isEqualTo(acc.tokenSymbol);
  }

  @Test
  void mergeSumsCountsAndAmounts() {
    final SupplyTrackingAccumulator a = this.function.createAccumulator();
    final SupplyTrackingAccumulator b = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), a);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "200", "WETH"), a);

    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "50", "WETH"), b);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "75", "WETH"), b);

    final SupplyTrackingAccumulator merged = this.function.merge(a, b);

    assertThat(merged.mintCount).isEqualTo(2);
    assertThat(merged.burnCount).isEqualTo(2);
    assertThat(merged.totalMinted).isEqualTo(new BigInteger("300"));
    assertThat(merged.totalBurned).isEqualTo(new BigInteger("125"));
    assertThat(merged.netMinted).isEqualTo(new BigInteger("175"));
  }

  @Test
  void mergePreservesDefinedTokenSymbol() {
    final SupplyTrackingAccumulator a = this.function.createAccumulator();
    final SupplyTrackingAccumulator b = this.function.createAccumulator();

    this.function.add(
        SupplyTrackingAggregateFunctionTest.mintEvent("0x123", "ethereum", "100", "WETH"), a);
    this.function.add(
        SupplyTrackingAggregateFunctionTest.burnEvent("0x123", "ethereum", "50", "WETH"), b);

    final SupplyTrackingAccumulator merged = this.function.merge(a, b);

    assertThat(merged.tokenSymbol.get()).isEqualTo("WETH");
  }

  @Property
  void mergeIsAssociative(
      @ForAll("mintBurnSequence") final BridgeSupplyEvent[] events) {
    if (events.length < 3) {
      return;
    }

    final SupplyTrackingAccumulator a = this.function.createAccumulator();
    final SupplyTrackingAccumulator b = this.function.createAccumulator();
    final SupplyTrackingAccumulator c = this.function.createAccumulator();

    this.function.add(events[0], a);
    this.function.add(events[1], b);
    this.function.add(events[2], c);

    final SupplyTrackingAccumulator leftAssoc = this.function.merge(this.function.merge(a, b), c);
    final SupplyTrackingAccumulator rightAssoc = this.function.merge(a, this.function.merge(b, c));

    assertThat(leftAssoc.mintCount).isEqualTo(rightAssoc.mintCount);
    assertThat(leftAssoc.burnCount).isEqualTo(rightAssoc.burnCount);
    assertThat(leftAssoc.totalMinted).isEqualTo(rightAssoc.totalMinted);
    assertThat(leftAssoc.totalBurned).isEqualTo(rightAssoc.totalBurned);
    assertThat(leftAssoc.netMinted).isEqualTo(rightAssoc.netMinted);
  }
}
