package com.web3analytics.functions.bridge;

import com.web3analytics.bridge.BridgeBurnEvent;
import com.web3analytics.bridge.BridgeMintEvent;
import com.web3analytics.functions.Web3Math;
import com.web3analytics.types.TriState;
import java.io.Serial;
import org.apache.flink.api.common.functions.AggregateFunction;
import org.apache.flink.api.java.tuple.Tuple2;

public class SupplyTrackingAggregateFunction implements
    AggregateFunction<
        Tuple2<BridgeMintEvent, BridgeBurnEvent>,
        SupplyTrackingAccumulator,
        SupplyTrackingAccumulator> {

  @Serial
  private static final long serialVersionUID = 1L;

  @Override
  public SupplyTrackingAccumulator createAccumulator() {
    return new SupplyTrackingAccumulator();
  }

  @Override
  public SupplyTrackingAccumulator add(
      final Tuple2<BridgeMintEvent, BridgeBurnEvent> tuple,
      final SupplyTrackingAccumulator acc) {

    if (tuple.f0 != null) {
      final BridgeMintEvent event = tuple.f0;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.chain = event.getChain();
        acc.protocol = event.getProtocol().toString();
      }
      if (acc.tokenSymbol.isUndefined()) {
        acc.tokenSymbol = TriState.of(event.getWrappedTokenSymbol());
      }
      acc.mintCount++;
      acc.totalMinted = acc.totalMinted.add(Web3Math.parseBigInt(event.getAmount()));
    }

    if (tuple.f1 != null) {
      final BridgeBurnEvent event = tuple.f1;
      if (acc.bridgeAddress == null) {
        acc.bridgeAddress = event.getBridgeAddress();
        acc.chain = event.getChain();
        acc.protocol = event.getProtocol().toString();
      }
      if (acc.tokenSymbol.isUndefined()) {
        acc.tokenSymbol = TriState.of(event.getWrappedTokenSymbol());
      }
      acc.burnCount++;
      acc.totalBurned = acc.totalBurned.add(Web3Math.parseBigInt(event.getAmount()));
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
