package com.web3analytics.model;

public class ReentrancyAccumulator implements WindowStamped {

  public String transactionHash;
  public String bridgeAddress;
  public String chain;
  public int lockCount = 0;
  public int unlockCount = 0;
  public long windowEnd;

  public ReentrancyAccumulator() {
  }

  public ReentrancyAccumulator(final String transactionHash, final String bridgeAddress,
      final String chain, final long windowEnd) {
    this.transactionHash = transactionHash;
    this.bridgeAddress = bridgeAddress;
    this.chain = chain;
    this.windowEnd = windowEnd;
  }

  @Override
  public void setWindowEnd(final long windowEnd) {
    this.windowEnd = windowEnd;
  }
}
