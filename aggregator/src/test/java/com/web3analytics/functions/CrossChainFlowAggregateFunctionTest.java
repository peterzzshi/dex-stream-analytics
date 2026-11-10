package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.within;

import com.web3analytics.model.BridgeActivity;
import com.web3analytics.model.CrossChainFlowAccumulator;
import com.web3analytics.model.EventKind;
import java.math.BigInteger;
import java.util.stream.Stream;
import net.jqwik.api.Arbitraries;
import net.jqwik.api.Arbitrary;
import net.jqwik.api.ForAll;
import net.jqwik.api.Property;
import net.jqwik.api.Provide;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

class CrossChainFlowAggregateFunctionTest {

  private final CrossChainFlowAggregateFunction function =
      new CrossChainFlowAggregateFunction();

  static Stream<Arguments> imbalanceRatioScenarios() {
    return Stream.of(
        Arguments.of("700", "300", 0.7),
        Arguments.of(null, null, 0.0),
        Arguments.of("500", "500", 0.5),
        Arguments.of("333", "667", 0.333)
    );
  }

  private static BridgeActivity activity(final EventKind kind, final String amount) {
    return new BridgeActivity(kind, "0x123", "ethereum", "wormhole", new BigInteger(amount));
  }

  @Provide
  static Arbitrary<BridgeActivity[]> activitySequence() {
    return Arbitraries.integers().between(1, 1000)
        .flatMap(amount -> Arbitraries.of(EventKind.values())
            .map(kind -> CrossChainFlowAggregateFunctionTest.activity(kind, amount.toString())))
        .list().ofMinSize(1).ofMaxSize(10)
        .map(list -> list.toArray(new BridgeActivity[0]));
  }

  @Test
  void lockIncrementsTotalInflow() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();
    final BridgeActivity lock = CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "100");

    this.function.add(lock, acc);

    assertThat(acc.totalInflow).isEqualTo(new BigInteger("100"));
    assertThat(acc.totalOutflow).isEqualTo(BigInteger.ZERO);
  }

  @Test
  void unlockIncrementsTotalOutflow() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();
    final BridgeActivity unlock = CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK,
        "200");

    this.function.add(unlock, acc);

    assertThat(acc.totalInflow).isEqualTo(BigInteger.ZERO);
    assertThat(acc.totalOutflow).isEqualTo(new BigInteger("200"));
  }

  @Test
  void mintIncrementsTotalInflow() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();
    final BridgeActivity mint = CrossChainFlowAggregateFunctionTest.activity(EventKind.MINT, "300");

    this.function.add(mint, acc);

    assertThat(acc.totalInflow).isEqualTo(new BigInteger("300"));
    assertThat(acc.totalOutflow).isEqualTo(BigInteger.ZERO);
  }

  @Test
  void burnIncrementsTotalOutflow() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();
    final BridgeActivity burn = CrossChainFlowAggregateFunctionTest.activity(EventKind.BURN, "400");

    this.function.add(burn, acc);

    assertThat(acc.totalInflow).isEqualTo(BigInteger.ZERO);
    assertThat(acc.totalOutflow).isEqualTo(new BigInteger("400"));
  }

  @Test
  void firstEventPopulatesMetadata() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();
    final BridgeActivity lock = CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "100");

    this.function.add(lock, acc);

    assertThat(acc.bridgeAddress).isEqualTo("0x123");
    assertThat(acc.protocol).isEqualTo("wormhole");
  }

  @Test
  void multipleInflowEventsAccumulate() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "100"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.MINT, "200"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "50"), acc);

    assertThat(acc.totalInflow).isEqualTo(new BigInteger("350"));
  }

  @Test
  void multipleOutflowEventsAccumulate() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "100"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.BURN, "150"), acc);

    assertThat(acc.totalOutflow).isEqualTo(new BigInteger("250"));
  }

  @Test
  void mixedInflowAndOutflow() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "1000"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "300"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.MINT, "500"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.BURN, "200"), acc);

    assertThat(acc.totalInflow).isEqualTo(new BigInteger("1500"));
    assertThat(acc.totalOutflow).isEqualTo(new BigInteger("500"));
  }

  @Test
  void getResultCalculatesNetFlow() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "1000"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "300"), acc);

    final CrossChainFlowAccumulator result = this.function.getResult(acc);

    assertThat(result.netFlow).isEqualTo(new BigInteger("700"));
  }

  @Test
  void getResultNetFlowCanBeNegative() {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "500"), acc);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "800"), acc);

    final CrossChainFlowAccumulator result = this.function.getResult(acc);

    assertThat(result.netFlow).isEqualTo(new BigInteger("-300"));
  }

  @ParameterizedTest
  @MethodSource("imbalanceRatioScenarios")
  void getResultCalculatesImbalanceRatio(final String lockAmount, final String unlockAmount,
      final double expectedRatio) {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    if (lockAmount != null) {
      this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, lockAmount),
          acc);
    }
    if (unlockAmount != null) {
      this.function.add(
          CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, unlockAmount), acc);
    }

    final CrossChainFlowAccumulator result = this.function.getResult(acc);

    assertThat(result.imbalanceRatio).isCloseTo(expectedRatio, within(0.0001));
  }

  @Test
  void mergeSumsInflowAndOutflow() {
    final CrossChainFlowAccumulator a = this.function.createAccumulator();
    final CrossChainFlowAccumulator b = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "500"), a);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "200"), a);

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.MINT, "300"), b);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.BURN, "100"), b);

    final CrossChainFlowAccumulator merged = this.function.merge(a, b);

    assertThat(merged.totalInflow).isEqualTo(new BigInteger("800"));
    assertThat(merged.totalOutflow).isEqualTo(new BigInteger("300"));
  }

  @Test
  void mergeCalculatesNetFlowAndImbalanceRatio() {
    final CrossChainFlowAccumulator a = this.function.createAccumulator();
    final CrossChainFlowAccumulator b = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "600"), a);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "400"), b);

    final CrossChainFlowAccumulator merged = this.function.merge(a, b);

    assertThat(merged.netFlow).isEqualTo(new BigInteger("200"));
    assertThat(merged.imbalanceRatio).isCloseTo(0.6, within(0.0001));
  }

  @Test
  void mergePreservesMetadata() {
    final CrossChainFlowAccumulator a = this.function.createAccumulator();
    final CrossChainFlowAccumulator b = this.function.createAccumulator();

    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.LOCK, "100"), a);
    this.function.add(CrossChainFlowAggregateFunctionTest.activity(EventKind.UNLOCK, "50"), b);

    final CrossChainFlowAccumulator merged = this.function.merge(a, b);

    assertThat(merged.bridgeAddress).isEqualTo("0x123");
    assertThat(merged.protocol).isEqualTo("wormhole");
  }

  @Property
  void mergeIsAssociative(@ForAll("activitySequence") final BridgeActivity[] activities) {
    if (activities.length < 3) {
      return;
    }

    final CrossChainFlowAccumulator a = this.function.createAccumulator();
    final CrossChainFlowAccumulator b = this.function.createAccumulator();
    final CrossChainFlowAccumulator c = this.function.createAccumulator();

    this.function.add(activities[0], a);
    this.function.add(activities[1], b);
    this.function.add(activities[2], c);

    final CrossChainFlowAccumulator leftAssoc = this.function.merge(this.function.merge(a, b), c);
    final CrossChainFlowAccumulator rightAssoc = this.function.merge(a, this.function.merge(b, c));

    assertThat(leftAssoc.totalInflow).isEqualTo(rightAssoc.totalInflow);
    assertThat(leftAssoc.totalOutflow).isEqualTo(rightAssoc.totalOutflow);
    assertThat(leftAssoc.netFlow).isEqualTo(rightAssoc.netFlow);
    assertThat(leftAssoc.imbalanceRatio).isCloseTo(rightAssoc.imbalanceRatio, within(0.0001));
  }

  @Property
  void imbalanceRatioIsAlwaysBetweenZeroAndOne(
      @ForAll("activitySequence") final BridgeActivity[] activities) {
    final CrossChainFlowAccumulator acc = this.function.createAccumulator();

    for (final BridgeActivity activity : activities) {
      this.function.add(activity, acc);
    }

    final CrossChainFlowAccumulator result = this.function.getResult(acc);

    assertThat(result.imbalanceRatio).isBetween(0.0, 1.0);
  }
}
