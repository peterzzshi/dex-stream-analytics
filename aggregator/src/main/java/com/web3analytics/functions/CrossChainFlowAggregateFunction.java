package com.web3analytics.functions;

import com.web3analytics.model.BridgeActivity;
import com.web3analytics.model.CrossChainFlowAccumulator;
import java.io.Serial;
import java.math.BigInteger;
import org.apache.flink.api.common.functions.AggregateFunction;

public class CrossChainFlowAggregateFunction implements
    AggregateFunction<BridgeActivity, CrossChainFlowAccumulator, CrossChainFlowAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  private static double calculateImbalanceRatio(final BigInteger inflow, final BigInteger outflow) {
    final BigInteger total = inflow.add(outflow);
    if (total.equals(BigInteger.ZERO)) {
      return 0.0;
    }
    // Rounded to 4dp for reporting; double carries that without loss, and going via BigDecimal
    // would only compute precision that the double return type discards.
    return Math.round(inflow.doubleValue() / total.doubleValue() * 10_000.0) / 10_000.0;
  }

  @Override
  public CrossChainFlowAccumulator createAccumulator() {
    return new CrossChainFlowAccumulator();
  }

  @Override
  public CrossChainFlowAccumulator add(
      final BridgeActivity activity,
      final CrossChainFlowAccumulator acc) {

    if (acc.bridgeAddress == null) {
      acc.bridgeAddress = activity.bridgeAddress();
      acc.protocol = activity.protocol();
    }

    switch (activity.kind()) {
      case LOCK, MINT -> acc.totalInflow = acc.totalInflow.add(activity.amount());
      case UNLOCK, BURN -> acc.totalOutflow = acc.totalOutflow.add(activity.amount());
    }

    return acc;
  }

  @Override
  public CrossChainFlowAccumulator getResult(final CrossChainFlowAccumulator acc) {
    final BigInteger netFlow = acc.totalInflow.subtract(acc.totalOutflow);
    final double imbalanceRatio = calculateImbalanceRatio(acc.totalInflow, acc.totalOutflow);
    final CrossChainFlowAccumulator result = new CrossChainFlowAccumulator(
        acc.bridgeAddress, acc.protocol, acc.windowEnd);
    result.totalInflow = acc.totalInflow;
    result.totalOutflow = acc.totalOutflow;
    result.netFlow = netFlow;
    result.imbalanceRatio = imbalanceRatio;
    return result;
  }

  @Override
  public CrossChainFlowAccumulator merge(
      final CrossChainFlowAccumulator a,
      final CrossChainFlowAccumulator b) {
    final CrossChainFlowAccumulator merged = new CrossChainFlowAccumulator(
        a.bridgeAddress,
        a.protocol,
        a.windowEnd
    );

    merged.totalInflow = a.totalInflow.add(b.totalInflow);
    merged.totalOutflow = a.totalOutflow.add(b.totalOutflow);
    merged.netFlow = merged.totalInflow.subtract(merged.totalOutflow);
    merged.imbalanceRatio = calculateImbalanceRatio(merged.totalInflow, merged.totalOutflow);

    return merged;
  }
}
