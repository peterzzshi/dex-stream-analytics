package com.web3analytics.functions.mev;

import static com.web3analytics.functions.Web3Math.parseBigInt;

import com.web3analytics.models.SwapEvent;
import java.math.BigInteger;

/**
 * Classifies swap direction for MEV pattern matching.
 */
public class SwapClassifier {

  public static Direction classify(SwapEvent swap) {
    BigInteger a0In = parseBigInt(swap.amount0In());
    BigInteger a1In = parseBigInt(swap.amount1In());
    BigInteger a0Out = parseBigInt(swap.amount0Out());
    BigInteger a1Out = parseBigInt(swap.amount1Out());

    boolean sellingToken0 = a0In.signum() > 0 && a1Out.signum() > 0;
    boolean sellingToken1 = a1In.signum() > 0 && a0Out.signum() > 0;

    if (sellingToken1 && !sellingToken0) {
      return Direction.BUY_TOKEN0;
    }
    if (sellingToken0 && !sellingToken1) {
      return Direction.BUY_TOKEN1;
    }
    return Direction.UNKNOWN;
  }

  static double estimateProfit(SwapEvent frontRun, SwapEvent backRun) {
    Double frontUSD = frontRun.volumeUSD();
    Double backUSD = backRun.volumeUSD();
    if (frontUSD == null || backUSD == null) {
      return 0.0;
    }
    return Math.abs(backUSD - frontUSD);
  }

  public enum Direction {BUY_TOKEN0, BUY_TOKEN1, UNKNOWN}
}
