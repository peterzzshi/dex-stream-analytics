package com.web3analytics.functions.bridge;

import com.web3analytics.bridge.BridgeBurnEvent;
import com.web3analytics.bridge.BridgeLockEvent;
import com.web3analytics.bridge.BridgeMintEvent;
import com.web3analytics.bridge.BridgeUnlockEvent;
import java.io.Serial;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.math.RoundingMode;
import org.apache.flink.api.common.functions.AggregateFunction;
import org.apache.flink.api.java.tuple.Tuple4;

public class CrossChainFlowAggregateFunction implements
    AggregateFunction<
        Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent>,
        CrossChainFlowAccumulator,
        CrossChainFlowAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  private static BigInteger parseBigInt(final String amount) {
    try {
      return new BigInteger(amount);
    } catch (final NumberFormatException e) {
      return BigInteger.ZERO;
    }
  }

  private static double calculateImbalanceRatio(final BigInteger inflow, final BigInteger outflow) {
    final BigInteger total = inflow.add(outflow);
    if (total.equals(BigInteger.ZERO)) {
      return 0.0;
    }

    final BigDecimal inflowDecimal = new BigDecimal(inflow);
    final BigDecimal totalDecimal = new BigDecimal(total);

    return inflowDecimal.divide(totalDecimal, 4, RoundingMode.HALF_UP).doubleValue();
  }

  @Override
  public CrossChainFlowAccumulator createAccumulator() {
    return new CrossChainFlowAccumulator();
  }

  @Override
  public CrossChainFlowAccumulator add(
      final Tuple4<BridgeLockEvent, BridgeUnlockEvent, BridgeMintEvent, BridgeBurnEvent> tuple,
      final CrossChainFlowAccumulator acc) {

    if (tuple.f0 != null) {
      final BridgeLockEvent event = tuple.f0;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.protocol = event.getProtocol().toString();
      }
      acc.totalInflow = acc.totalInflow.add(parseBigInt(event.getAmount()));
    }

    if (tuple.f1 != null) {
      final BridgeUnlockEvent event = tuple.f1;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.protocol = event.getProtocol().toString();
      }
      acc.totalOutflow = acc.totalOutflow.add(parseBigInt(event.getAmount()));
    }

    if (tuple.f2 != null) {
      final BridgeMintEvent event = tuple.f2;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.protocol = event.getProtocol().toString();
      }
      acc.totalInflow = acc.totalInflow.add(parseBigInt(event.getAmount()));
    }

    if (tuple.f3 != null) {
      final BridgeBurnEvent event = tuple.f3;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.protocol = event.getProtocol().toString();
      }
      acc.totalOutflow = acc.totalOutflow.add(parseBigInt(event.getAmount()));
    }

    return acc;
  }

  @Override
  public CrossChainFlowAccumulator getResult(final CrossChainFlowAccumulator acc) {
    acc.netFlow = acc.totalInflow.subtract(acc.totalOutflow);
    acc.imbalanceRatio = calculateImbalanceRatio(acc.totalInflow, acc.totalOutflow);
    return acc;
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
