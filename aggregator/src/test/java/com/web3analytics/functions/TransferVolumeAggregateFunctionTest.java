package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.bridge.BridgeProtocol;
import com.web3analytics.bridge.BridgeTransferEvent;
import com.web3analytics.bridge.TransferDirection;
import com.web3analytics.model.TransferVolumeAccumulator;
import com.web3analytics.model.TriState;
import java.math.BigDecimal;
import java.math.BigInteger;
import net.jqwik.api.Arbitraries;
import net.jqwik.api.Arbitrary;
import net.jqwik.api.ForAll;
import net.jqwik.api.Property;
import net.jqwik.api.Provide;
import org.junit.jupiter.api.Test;

class TransferVolumeAggregateFunctionTest {

  private final TransferVolumeAggregateFunction function =
      new TransferVolumeAggregateFunction();

  private static BridgeTransferEvent lockEvent(final String bridge, final String chain,
      final String amount, final String symbol) {
    return BridgeTransferEvent.newBuilder()
        .setEventId("evt-lock-" + amount)
        .setTransactionHash("0xabc")
        .setBridgeAddress(bridge)
        .setChain(chain)
        .setChainName("ethereum")
        .setDirection(TransferDirection.lock)
        .setProtocol(BridgeProtocol.wormhole)
        .setAmount(new BigDecimal(new BigInteger(amount)))
        .setTokenSymbol(symbol)
        .setBlockNumber(1000L)
        .setBlockTimestamp(1234567890L)
        .setLogIndex(0)
        .setGasUsed(50000L)
        .setGasPrice("100000000")
        .setEventTimestamp(1234567890L)
        .setSender("0xsender")
        .setRecipient("0xrecipient")
        .setToken("0xtoken")
        .build();
  }

  private static BridgeTransferEvent unlockEvent(final String bridge, final String chain,
      final String amount, final String symbol) {
    return BridgeTransferEvent.newBuilder()
        .setEventId("evt-unlock-" + amount)
        .setTransactionHash("0xabc")
        .setBridgeAddress(bridge)
        .setChain(chain)
        .setChainName("ethereum")
        .setDirection(TransferDirection.unlock)
        .setProtocol(BridgeProtocol.wormhole)
        .setAmount(new BigDecimal(new BigInteger(amount)))
        .setTokenSymbol(symbol)
        .setBlockNumber(1000L)
        .setBlockTimestamp(1234567890L)
        .setLogIndex(0)
        .setGasUsed(50000L)
        .setGasPrice("100000000")
        .setEventTimestamp(1234567890L)
        .setSender("0xsender")
        .setRecipient("0xrecipient")
        .setToken("0xtoken")
        .build();
  }

  @Provide
  static Arbitrary<BridgeTransferEvent[]> lockUnlockSequence() {
    return Arbitraries.integers().between(1, 1000)
        .flatMap(amount -> Arbitraries.of(
            TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", amount.toString(),
                "USDC"),
            TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", amount.toString(),
                "WETH")
        ))
        .list().ofMinSize(3).ofMaxSize(10)
        .map(list -> list.toArray(new BridgeTransferEvent[0]));
  }

  @Test
  void mergeDefersUndefinedToken1SymbolToOtherAccumulator() {
    final TransferVolumeAccumulator a = this.function.createAccumulator();
    final TransferVolumeAccumulator b = this.function.createAccumulator();

    a.token1Symbol = TriState.undefined();
    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "100", "WETH"), b);

    final TransferVolumeAccumulator merged = this.function.merge(a, b);

    assertThat(merged.token1Symbol.get()).isEqualTo("WETH");
  }

  @Test
  void lockIncrementsCountAndSumsAmount() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent lock = TransferVolumeAggregateFunctionTest.lockEvent("0x123",
        "ethereum", "100", "USDC");

    this.function.add(lock, acc);

    assertThat(acc.lockCount).isEqualTo(1);
    assertThat(acc.unlockCount).isZero();
    assertThat(acc.totalLockAmount).isEqualTo(new BigInteger("100"));
    assertThat(acc.totalUnlockAmount).isEqualTo(BigInteger.ZERO);
  }

  @Test
  void unlockIncrementsCountAndSumsAmount() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent unlock = TransferVolumeAggregateFunctionTest.unlockEvent("0x123",
        "ethereum", "200", "WETH");

    this.function.add(unlock, acc);

    assertThat(acc.lockCount).isZero();
    assertThat(acc.unlockCount).isEqualTo(1);
    assertThat(acc.totalLockAmount).isEqualTo(BigInteger.ZERO);
    assertThat(acc.totalUnlockAmount).isEqualTo(new BigInteger("200"));
  }

  @Test
  void firstLockCapturesToken0Symbol() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), acc);

    assertThat(acc.token0Symbol.isDefined()).isTrue();
    assertThat(acc.token0Symbol.get()).isEqualTo("USDC");
    assertThat(acc.token1Symbol.isUndefined()).isTrue();
  }

  @Test
  void firstUnlockCapturesToken1Symbol() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "200", "WETH"), acc);

    assertThat(acc.token1Symbol.isDefined()).isTrue();
    assertThat(acc.token1Symbol.get()).isEqualTo("WETH");
    assertThat(acc.token0Symbol.isUndefined()).isTrue();
  }

  @Test
  void subsequentLocksDoNotOverwriteToken0Symbol() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "200", "DAI"), acc);

    assertThat(acc.token0Symbol.get()).isEqualTo("USDC");
    assertThat(acc.lockCount).isEqualTo(2);
  }

  @Test
  void subsequentUnlocksDoNotOverwriteToken1Symbol() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "100", "WETH"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "200", "WBTC"), acc);

    assertThat(acc.token1Symbol.get()).isEqualTo("WETH");
    assertThat(acc.unlockCount).isEqualTo(2);
  }

  @Test
  void nullTokenSymbolIsCapturedAsNull() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent lockWithNull = TransferVolumeAggregateFunctionTest.lockEvent("0x123",
        "ethereum", "100", null);

    this.function.add(lockWithNull, acc);

    assertThat(acc.token0Symbol.isNull()).isTrue();
  }

  @Test
  void multipleLocksAccumulateAmount() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "250", "USDC"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "50", "USDC"), acc);

    assertThat(acc.lockCount).isEqualTo(3);
    assertThat(acc.totalLockAmount).isEqualTo(new BigInteger("400"));
  }

  @Test
  void multipleUnlocksAccumulateAmount() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "100", "WETH"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "150", "WETH"), acc);

    assertThat(acc.unlockCount).isEqualTo(2);
    assertThat(acc.totalUnlockAmount).isEqualTo(new BigInteger("250"));
  }

  @Test
  void mixedLocksAndUnlocks() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "50", "WETH"), acc);
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "200", "USDC"), acc);

    assertThat(acc.lockCount).isEqualTo(2);
    assertThat(acc.unlockCount).isEqualTo(1);
    assertThat(acc.totalLockAmount).isEqualTo(new BigInteger("300"));
    assertThat(acc.totalUnlockAmount).isEqualTo(new BigInteger("50"));
    assertThat(acc.token0Symbol.get()).isEqualTo("USDC");
    assertThat(acc.token1Symbol.get()).isEqualTo("WETH");
  }

  @Test
  void getResultReturnsAccumulatorUnchanged() {
    final TransferVolumeAccumulator acc = this.function.createAccumulator();
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), acc);

    final TransferVolumeAccumulator result = this.function.getResult(acc);

    assertThat(result).isSameAs(acc);
  }

  @Test
  void mergeSumsCountsAndAmounts() {
    final TransferVolumeAccumulator a = this.function.createAccumulator();
    final TransferVolumeAccumulator b = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), a);
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "200", "USDC"), a);

    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "50", "WETH"), b);
    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "75", "WETH"), b);

    final TransferVolumeAccumulator merged = this.function.merge(a, b);

    assertThat(merged.lockCount).isEqualTo(2);
    assertThat(merged.unlockCount).isEqualTo(2);
    assertThat(merged.totalLockAmount).isEqualTo(new BigInteger("300"));
    assertThat(merged.totalUnlockAmount).isEqualTo(new BigInteger("125"));
  }

  @Test
  void mergePreservesDefinedToken0Symbol() {
    final TransferVolumeAccumulator a = this.function.createAccumulator();
    final TransferVolumeAccumulator b = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), a);
    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "50", "WETH"), b);

    final TransferVolumeAccumulator merged = this.function.merge(a, b);

    assertThat(merged.token0Symbol.get()).isEqualTo("USDC");
  }

  @Test
  void mergeDefersUndefinedToken0SymbolToOtherAccumulator() {
    final TransferVolumeAccumulator a = this.function.createAccumulator();
    final TransferVolumeAccumulator b = this.function.createAccumulator();

    a.token0Symbol = TriState.undefined();
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "100", "USDC"), b);

    final TransferVolumeAccumulator merged = this.function.merge(a, b);

    assertThat(merged.token0Symbol.get()).isEqualTo("USDC");
  }

  @Test
  void mergePreservesDefinedToken1Symbol() {
    final TransferVolumeAccumulator a = this.function.createAccumulator();
    final TransferVolumeAccumulator b = this.function.createAccumulator();

    this.function.add(
        TransferVolumeAggregateFunctionTest.unlockEvent("0x123", "ethereum", "100", "WETH"), a);
    this.function.add(
        TransferVolumeAggregateFunctionTest.lockEvent("0x123", "ethereum", "50", "USDC"), b);

    final TransferVolumeAccumulator merged = this.function.merge(a, b);

    assertThat(merged.token1Symbol.get()).isEqualTo("WETH");
  }

  @Property
  void mergeIsAssociative(
      @ForAll("lockUnlockSequence") final BridgeTransferEvent[] events) {
    if (events.length < 3) {
      return;
    }

    final TransferVolumeAccumulator a = this.function.createAccumulator();
    final TransferVolumeAccumulator b = this.function.createAccumulator();
    final TransferVolumeAccumulator c = this.function.createAccumulator();

    this.function.add(events[0], a);
    this.function.add(events[1], b);
    this.function.add(events[2], c);

    final TransferVolumeAccumulator leftAssoc = this.function.merge(this.function.merge(a, b), c);
    final TransferVolumeAccumulator rightAssoc = this.function.merge(a, this.function.merge(b, c));

    assertThat(leftAssoc.lockCount).isEqualTo(rightAssoc.lockCount);
    assertThat(leftAssoc.unlockCount).isEqualTo(rightAssoc.unlockCount);
    assertThat(leftAssoc.totalLockAmount).isEqualTo(rightAssoc.totalLockAmount);
    assertThat(leftAssoc.totalUnlockAmount).isEqualTo(rightAssoc.totalUnlockAmount);
  }
}
