package com.web3analytics.model;

import java.math.BigInteger;

public class CrossChainFlowAccumulator implements WindowStamped {
    public String bridgeAddress;
    public String protocol;
  public BigInteger totalInflow = BigInteger.ZERO;
    public BigInteger totalOutflow = BigInteger.ZERO;
    public BigInteger netFlow = BigInteger.ZERO;
    public double imbalanceRatio = 0.0;
    public long windowEnd;

    public CrossChainFlowAccumulator() {}

  public CrossChainFlowAccumulator(final String bridgeAddress, final String protocol,
      final long windowEnd) {
        this.bridgeAddress = bridgeAddress;
        this.protocol = protocol;
        this.windowEnd = windowEnd;
    }

  @Override
  public void setWindowEnd(final long windowEnd) {
    this.windowEnd = windowEnd;
  }
}
