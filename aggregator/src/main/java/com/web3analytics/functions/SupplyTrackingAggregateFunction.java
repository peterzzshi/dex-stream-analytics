package com.web3analytics.functions;

import com.web3analytics.bridge.BridgeSupplyEvent;
import com.web3analytics.model.SupplyTrackingAccumulator;
import com.web3analytics.model.TriState;
import java.io.Serial;
import org.apache.flink.api.common.functions.AggregateFunction;

/**
 * Mint and burn arrive on one stream and are told apart by the {@code operation} field.
 */
public class SupplyTrackingAggregateFunction implements
    AggregateFunction<BridgeSupplyEvent, SupplyTrackingAccumulator, SupplyTrackingAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  @Override
  public SupplyTrackingAccumulator createAccumulator() {
    return new SupplyTrackingAccumulator();
  }

  @Override
  public SupplyTrackingAccumulator add(
      final BridgeSupplyEvent event,
      final SupplyTrackingAccumulator acc) {

    if (acc.bridgeAddress == null) {
      acc.bridgeAddress = event.getBridgeAddress();
      acc.chain = event.getChain();
      acc.protocol = event.getProtocol().toString();
    }
    if (acc.tokenSymbol.isUndefined()) {
      acc.tokenSymbol = TriState.ofNullable(event.getWrappedTokenSymbol());
    }

    switch (event.getOperation()) {
      case mint -> {
        acc.mintCount++;
        acc.totalMinted = acc.totalMinted.add(event.getAmount().unscaledValue());
      }
      case burn -> {
        acc.burnCount++;
        acc.totalBurned = acc.totalBurned.add(event.getAmount().unscaledValue());
      }
    }

    return acc;
  }

  @Override
  public SupplyTrackingAccumulator getResult(final SupplyTrackingAccumulator acc) {
    final SupplyTrackingAccumulator result = new SupplyTrackingAccumulator(
        acc.bridgeAddress, acc.chain, acc.protocol, acc.windowEnd);
    result.tokenSymbol = acc.tokenSymbol;
    result.mintCount = acc.mintCount;
    result.burnCount = acc.burnCount;
    result.totalMinted = acc.totalMinted;
    result.totalBurned = acc.totalBurned;
    result.netMinted = acc.totalMinted.subtract(acc.totalBurned);
    return result;
  }

  @Override
  public SupplyTrackingAccumulator merge(
      final SupplyTrackingAccumulator a,
      final SupplyTrackingAccumulator b) {
    final SupplyTrackingAccumulator merged = new SupplyTrackingAccumulator(
        a.bridgeAddress,
        a.chain,
        a.protocol,
        a.windowEnd
    );

    merged.tokenSymbol = a.tokenSymbol.isUndefined() ? b.tokenSymbol : a.tokenSymbol;
    merged.mintCount = a.mintCount + b.mintCount;
    merged.burnCount = a.burnCount + b.burnCount;
    merged.totalMinted = a.totalMinted.add(b.totalMinted);
    merged.totalBurned = a.totalBurned.add(b.totalBurned);
    merged.netMinted = merged.totalMinted.subtract(merged.totalBurned);

    return merged;
  }
}
