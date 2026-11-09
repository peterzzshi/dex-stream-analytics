package com.web3analytics.functions.bridge;

import java.math.BigInteger;

public class CrossChainFlowAccumulator {
    public String bridgeAddress;
    public String protocol;
    public BigInteger totalInflow = BigInteger.ZERO;
    public BigInteger totalOutflow = BigInteger.ZERO;
    public BigInteger netFlow = BigInteger.ZERO;
    public double imbalanceRatio = 0.0;
    public long windowEnd;

    public CrossChainFlowAccumulator() {}

    public CrossChainFlowAccumulator(String bridgeAddress, String protocol, long windowEnd) {
        this.bridgeAddress = bridgeAddress;
        this.protocol = protocol;
        this.windowEnd = windowEnd;
    }
}
