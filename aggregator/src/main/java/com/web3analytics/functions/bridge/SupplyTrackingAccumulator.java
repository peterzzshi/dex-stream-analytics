package com.web3analytics.functions.bridge;

import com.web3analytics.types.TriState;
import java.math.BigInteger;

public class SupplyTrackingAccumulator {

  public String bridgeAddress;
  public String chain;
  public String protocol;
  public TriState<String> tokenSymbol = TriState.undefined();
  public int mintCount = 0;
  public int burnCount = 0;
  public BigInteger totalMinted = BigInteger.ZERO;
  public BigInteger totalBurned = BigInteger.ZERO;
  public BigInteger netMinted = BigInteger.ZERO;
  public long windowEnd;

  public SupplyTrackingAccumulator() {
  }

  public SupplyTrackingAccumulator(final String bridgeAddress, final String chain,
      final String protocol, final long windowEnd) {
    this.bridgeAddress = bridgeAddress;
    this.chain = chain;
    this.protocol = protocol;
    this.windowEnd = windowEnd;
  }
}
