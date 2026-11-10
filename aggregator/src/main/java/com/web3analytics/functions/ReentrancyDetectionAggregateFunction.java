package com.web3analytics.functions;

import com.web3analytics.bridge.BridgeTransferEvent;
import com.web3analytics.model.ReentrancyAccumulator;
import java.io.Serial;
import org.apache.flink.api.common.functions.AggregateFunction;

/**
 * Groups lock/unlock events by transaction hash to detect multiple calls within a single atomic
 * transaction — a signal reentrancy guards are bypassed or absent. Keyed per-transaction rather
 * than per-bridge, unlike the other three pipelines, since reentrancy is a property of the
 * transaction, not the window's bridge-level totals.
 */
public class ReentrancyDetectionAggregateFunction implements
    AggregateFunction<BridgeTransferEvent, ReentrancyAccumulator, ReentrancyAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  @Override
  public ReentrancyAccumulator createAccumulator() {
    return new ReentrancyAccumulator();
  }

  @Override
  public ReentrancyAccumulator add(
      final BridgeTransferEvent event,
      final ReentrancyAccumulator acc) {

    if (acc.transactionHash == null) {
      acc.transactionHash = event.getTransactionHash();
      acc.bridgeAddress = event.getBridgeAddress();
      acc.chain = event.getChain();
    }

    switch (event.getDirection()) {
      case lock -> acc.lockCount++;
      case unlock -> acc.unlockCount++;
    }

    return acc;
  }

  @Override
  public ReentrancyAccumulator getResult(final ReentrancyAccumulator acc) {
    return acc;
  }

  @Override
  public ReentrancyAccumulator merge(
      final ReentrancyAccumulator a,
      final ReentrancyAccumulator b) {
    final ReentrancyAccumulator merged = new ReentrancyAccumulator(
        a.transactionHash, a.bridgeAddress, a.chain, a.windowEnd);
    merged.lockCount = a.lockCount + b.lockCount;
    merged.unlockCount = a.unlockCount + b.unlockCount;
    return merged;
  }
}
