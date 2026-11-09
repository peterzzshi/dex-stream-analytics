package com.web3analytics.functions.bridge;

import com.web3analytics.types.TriState;
import java.math.BigInteger;

public class TransferVolumeAccumulator {

  public String bridgeAddress;
  public String chain;
  public String protocol;
  public TriState<String> token0Symbol = TriState.undefined();
  public TriState<String> token1Symbol = TriState.undefined();
  public int lockCount = 0;
  public int unlockCount = 0;
  public BigInteger totalLockAmount = BigInteger.ZERO;
  public BigInteger totalUnlockAmount = BigInteger.ZERO;
  public long windowEnd;

  public TransferVolumeAccumulator() {
  }

  public TransferVolumeAccumulator(final String bridgeAddress, final String chain,
      final String protocol, final long windowEnd) {
    this.bridgeAddress = bridgeAddress;
    this.chain = chain;
    this.protocol = protocol;
    this.windowEnd = windowEnd;
  }
}
