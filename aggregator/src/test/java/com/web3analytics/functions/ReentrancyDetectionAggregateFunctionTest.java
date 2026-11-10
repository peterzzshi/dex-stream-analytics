package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.bridge.BridgeProtocol;
import com.web3analytics.bridge.BridgeTransferEvent;
import com.web3analytics.bridge.TransferDirection;
import com.web3analytics.model.ReentrancyAccumulator;
import java.math.BigDecimal;
import net.jqwik.api.Arbitraries;
import net.jqwik.api.Arbitrary;
import net.jqwik.api.ForAll;
import net.jqwik.api.Property;
import net.jqwik.api.Provide;
import org.junit.jupiter.api.Test;

class ReentrancyDetectionAggregateFunctionTest {

  private final ReentrancyDetectionAggregateFunction function =
      new ReentrancyDetectionAggregateFunction();

  private static BridgeTransferEvent lockEvent(final String txHash, final String bridge,
      final String chain) {
    return BridgeTransferEvent.newBuilder()
        .setEventId("evt-" + txHash + "-0")
        .setTransactionHash(txHash)
        .setBridgeAddress(bridge)
        .setChain(chain)
        .setChainName("ethereum")
        .setDirection(TransferDirection.lock)
        .setProtocol(BridgeProtocol.wormhole)
        .setAmount(BigDecimal.valueOf(100))
        .setTokenSymbol("USDC")
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

  private static BridgeTransferEvent unlockEvent(final String txHash, final String bridge,
      final String chain) {
    return BridgeTransferEvent.newBuilder()
        .setEventId("evt-" + txHash + "-0")
        .setTransactionHash(txHash)
        .setBridgeAddress(bridge)
        .setChain(chain)
        .setChainName("ethereum")
        .setDirection(TransferDirection.unlock)
        .setProtocol(BridgeProtocol.wormhole)
        .setAmount(BigDecimal.valueOf(100))
        .setTokenSymbol("USDC")
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
    return Arbitraries.of(
        ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc", "0x123", "ethereum"),
        ReentrancyDetectionAggregateFunctionTest.unlockEvent("0xabc", "0x123", "ethereum")
    ).list().ofMinSize(3).ofMaxSize(10).map(list -> list.toArray(new BridgeTransferEvent[0]));
  }

  @Test
  void mergeSumsCounts() {
    final ReentrancyAccumulator a = this.function.createAccumulator();
    final ReentrancyAccumulator b = this.function.createAccumulator();

    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc", "0x123", "ethereum"), a);
    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc", "0x123", "ethereum"), a);
    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.unlockEvent("0xabc", "0x123", "ethereum"), b);

    final ReentrancyAccumulator merged = this.function.merge(a, b);

    assertThat(merged.lockCount).isEqualTo(2);
    assertThat(merged.unlockCount).isEqualTo(1);
    assertThat(merged.transactionHash).isEqualTo("0xabc");
  }

  @Test
  void lockIncrementLockCount() {
    final ReentrancyAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent event = ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc",
        "0x123", "ethereum");

    this.function.add(event, acc);

    assertThat(acc.lockCount).isEqualTo(1);
    assertThat(acc.unlockCount).isZero();
  }

  @Test
  void unlockIncrementsUnlockCount() {
    final ReentrancyAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent event = ReentrancyDetectionAggregateFunctionTest.unlockEvent("0xabc",
        "0x123", "ethereum");

    this.function.add(event, acc);

    assertThat(acc.lockCount).isZero();
    assertThat(acc.unlockCount).isEqualTo(1);
  }

  @Test
  void firstEventPopulatesMetadata() {
    final ReentrancyAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent event = ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc",
        "0x123", "ethereum");

    this.function.add(event, acc);

    assertThat(acc.transactionHash).isEqualTo("0xabc");
    assertThat(acc.bridgeAddress).isEqualTo("0x123");
    assertThat(acc.chain).isEqualTo("ethereum");
  }

  @Test
  void multipleLocksInSameTransaction() {
    final ReentrancyAccumulator acc = this.function.createAccumulator();
    final BridgeTransferEvent lock1 = ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc",
        "0x123", "ethereum");
    final BridgeTransferEvent lock2 = ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc",
        "0x123", "ethereum");
    final BridgeTransferEvent lock3 = ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc",
        "0x123", "ethereum");

    this.function.add(lock1, acc);
    this.function.add(lock2, acc);
    this.function.add(lock3, acc);

    assertThat(acc.lockCount).isEqualTo(3);
    assertThat(acc.unlockCount).isZero();
  }

  @Test
  void mixedLocksAndUnlocks() {
    final ReentrancyAccumulator acc = this.function.createAccumulator();

    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc", "0x123", "ethereum"), acc);
    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.unlockEvent("0xabc", "0x123", "ethereum"), acc);
    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc", "0x123", "ethereum"), acc);
    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.unlockEvent("0xabc", "0x123", "ethereum"), acc);

    assertThat(acc.lockCount).isEqualTo(2);
    assertThat(acc.unlockCount).isEqualTo(2);
  }

  @Test
  void getResultReturnsAccumulatorUnchanged() {
    final ReentrancyAccumulator acc = this.function.createAccumulator();
    this.function.add(
        ReentrancyDetectionAggregateFunctionTest.lockEvent("0xabc", "0x123", "ethereum"), acc);

    final ReentrancyAccumulator result = this.function.getResult(acc);

    assertThat(result).isSameAs(acc);
    assertThat(result.lockCount).isEqualTo(1);
  }

  @Property
  void mergeIsAssociative(
      @ForAll("lockUnlockSequence") final BridgeTransferEvent[] events) {
    if (events.length < 3) {
      return;
    }

    final ReentrancyAccumulator a = this.function.createAccumulator();
    final ReentrancyAccumulator b = this.function.createAccumulator();
    final ReentrancyAccumulator c = this.function.createAccumulator();

    this.function.add(events[0], a);
    this.function.add(events[1], b);
    this.function.add(events[2], c);

    final ReentrancyAccumulator leftAssoc = this.function.merge(this.function.merge(a, b), c);
    final ReentrancyAccumulator rightAssoc = this.function.merge(a, this.function.merge(b, c));

    assertThat(leftAssoc.lockCount).isEqualTo(rightAssoc.lockCount);
    assertThat(leftAssoc.unlockCount).isEqualTo(rightAssoc.unlockCount);
  }
}
