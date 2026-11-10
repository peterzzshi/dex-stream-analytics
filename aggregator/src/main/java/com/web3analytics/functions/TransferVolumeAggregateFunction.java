package com.web3analytics.functions;

import com.web3analytics.bridge.BridgeTransferEvent;
import com.web3analytics.model.TransferVolumeAccumulator;
import com.web3analytics.model.TriState;
import java.io.Serial;
import org.apache.flink.api.common.functions.AggregateFunction;

/**
 * Lock and unlock arrive on one stream and are told apart by the {@code direction} field, so the
 * two counters update from one code path rather than from two branches over a union type.
 */
public class TransferVolumeAggregateFunction implements
    AggregateFunction<BridgeTransferEvent, TransferVolumeAccumulator, TransferVolumeAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  @Override
  public TransferVolumeAccumulator createAccumulator() {
    return new TransferVolumeAccumulator();
  }

  @Override
  public TransferVolumeAccumulator add(
      final BridgeTransferEvent event,
      final TransferVolumeAccumulator acc) {

    if (acc.bridgeAddress == null) {
      acc.bridgeAddress = event.getBridgeAddress();
      acc.chain = event.getChain();
      acc.protocol = event.getProtocol().toString();
    }

    switch (event.getDirection()) {
      case lock -> {
        if (acc.token0Symbol.isUndefined()) {
          acc.token0Symbol = TriState.ofNullable(event.getTokenSymbol());
        }
        acc.lockCount++;
        acc.totalLockAmount = acc.totalLockAmount.add(event.getAmount().unscaledValue());
      }
      case unlock -> {
        if (acc.token1Symbol.isUndefined()) {
          acc.token1Symbol = TriState.ofNullable(event.getTokenSymbol());
        }
        acc.unlockCount++;
        acc.totalUnlockAmount = acc.totalUnlockAmount.add(event.getAmount().unscaledValue());
      }
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
