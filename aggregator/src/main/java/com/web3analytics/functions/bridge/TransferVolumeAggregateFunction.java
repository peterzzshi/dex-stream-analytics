package com.web3analytics.functions.bridge;

import com.web3analytics.bridge.BridgeLockEvent;
import com.web3analytics.bridge.BridgeUnlockEvent;
import com.web3analytics.functions.Web3Math;
import com.web3analytics.types.TriState;
import java.io.Serial;
import org.apache.flink.api.common.functions.AggregateFunction;
import org.apache.flink.api.java.tuple.Tuple2;

public class TransferVolumeAggregateFunction implements
    AggregateFunction<
        Tuple2<BridgeLockEvent, BridgeUnlockEvent>,
        TransferVolumeAccumulator,
        TransferVolumeAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  @Override
  public TransferVolumeAccumulator createAccumulator() {
    return new TransferVolumeAccumulator();
  }

  @Override
  public TransferVolumeAccumulator add(
      final Tuple2<BridgeLockEvent, BridgeUnlockEvent> tuple,
      final TransferVolumeAccumulator acc) {

    if (tuple.f0 != null) {
      final BridgeLockEvent event = tuple.f0;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.chain = event.getChain();
        acc.protocol = event.getProtocol().toString();
      }
      if (acc.token0Symbol.isUndefined()) {
        acc.token0Symbol = TriState.of(event.getTokenSymbol());
      }
      acc.lockCount++;
      acc.totalLockAmount = acc.totalLockAmount.add(Web3Math.parseBigInt(event.getAmount()));
    }

    if (tuple.f1 != null) {
      final BridgeUnlockEvent event = tuple.f1;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.chain = event.getChain();
        acc.protocol = event.getProtocol().toString();
      }
      if (acc.token1Symbol.isUndefined()) {
        acc.token1Symbol = TriState.of(event.getTokenSymbol());
      }
      acc.unlockCount++;
      acc.totalUnlockAmount = acc.totalUnlockAmount.add(Web3Math.parseBigInt(event.getAmount()));
    }

    return acc;
  }

  @Override
  public TransferVolumeAccumulator getResult(final TransferVolumeAccumulator acc) {
    return acc;
  }

  @Override
  public TransferVolumeAccumulator merge(
      final TransferVolumeAccumulator a,
      final TransferVolumeAccumulator b) {
    final TransferVolumeAccumulator merged = new TransferVolumeAccumulator(
        a.bridgeAddress,
        a.chain,
        a.protocol,
        a.windowEnd
    );

    merged.token0Symbol = a.token0Symbol.isUndefined() ? b.token0Symbol : a.token0Symbol;
    merged.token1Symbol = a.token1Symbol.isUndefined() ? b.token1Symbol : a.token1Symbol;
    merged.lockCount = a.lockCount + b.lockCount;
    merged.unlockCount = a.unlockCount + b.unlockCount;
    merged.totalLockAmount = a.totalLockAmount.add(b.totalLockAmount);
    merged.totalUnlockAmount = a.totalUnlockAmount.add(b.totalUnlockAmount);

    return merged;
  }
}
