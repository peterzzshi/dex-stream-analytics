package com.web3analytics.functions;

import static com.web3analytics.functions.Web3Math.normalizeAddress;
import static com.web3analytics.functions.Web3Math.parseBigInt;
import static com.web3analytics.functions.WindowUtils.buildWindowId;
import static com.web3analytics.functions.WindowUtils.preferSymbol;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.LiquidityAnalytics;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.TransferEvent;
import java.math.BigInteger;
import java.time.Instant;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

/**
 * Full-window scan for LP behavior metrics.
 * Intentionally non-incremental: lower-frequency stream where buffering the full
 * window is acceptable and simplifies the churn/provider-set logic.
 * Transfer events are correlated to Mint/Burn by transactionHash to compute
 * LP token amounts minted/burned per window.
 */
public class LiquidityWindowFunction
        extends ProcessWindowFunction<DexEvent, LiquidityAnalytics, String, TimeWindow> {

  static LiquidityAnalytics summarize(
      String pairAddress,
      long windowStart,
      long windowEnd,
      Iterable<DexEvent> events
  ) {
    Accumulator acc = new Accumulator();

    for (DexEvent event : events) {
      processEvent(event, acc);
    }

    return buildResult(pairAddress, windowStart, windowEnd, acc);
  }

  private static void processEvent(DexEvent event, Accumulator acc) {
    if (acc.chain == null) {
      acc.chain = event.chain();
      acc.chainName = event.chainName();
    }

    switch (event) {
      case MintEvent mint -> processMint(mint, acc);
      case BurnEvent burn -> processBurn(burn, acc);
      case TransferEvent transfer -> processTransfer(transfer, acc);
      case com.web3analytics.models.SwapEvent ignored -> {
        // SwapEvents are ignored in liquidity analysis
      }
    }
  }

  private static void processMint(MintEvent mint, Accumulator acc) {
    acc.mintCount++;
    acc.totalMintAmount0 = acc.totalMintAmount0.add(parseBigInt(mint.amount0()));
    acc.totalMintAmount1 = acc.totalMintAmount1.add(parseBigInt(mint.amount1()));
    acc.mintProviders.add(normalizeAddress(mint.sender()));
    acc.mintTxHashes.add(mint.transactionHash());

    if (acc.token0Symbol == null || acc.token1Symbol == null) {
      acc.token0Symbol = preferSymbol(acc.token0Symbol, mint.token0Symbol());
      acc.token1Symbol = preferSymbol(acc.token1Symbol, mint.token1Symbol());
    }
  }

  private static void processBurn(BurnEvent burn, Accumulator acc) {
    acc.burnCount++;
    acc.totalBurnAmount0 = acc.totalBurnAmount0.add(parseBigInt(burn.amount0()));
    acc.totalBurnAmount1 = acc.totalBurnAmount1.add(parseBigInt(burn.amount1()));
    acc.burnProviders.add(normalizeAddress(burn.sender()));
    acc.burnTxHashes.add(burn.transactionHash());

    if (acc.token0Symbol == null || acc.token1Symbol == null) {
      acc.token0Symbol = preferSymbol(acc.token0Symbol, burn.token0Symbol());
      acc.token1Symbol = preferSymbol(acc.token1Symbol, burn.token1Symbol());
    }
  }

  private static void processTransfer(TransferEvent transfer, Accumulator acc) {
    acc.transferCount++;
    BigInteger value = parseBigInt(transfer.value());
    acc.transferValuesByTx.merge(transfer.transactionHash(), value, BigInteger::add);
  }

  private static LiquidityAnalytics buildResult(
      String pairAddress,
      long windowStart,
      long windowEnd,
      Accumulator acc
  ) {
    // Correlate: transfers in mint txs are LP tokens minted
    // Transfers in burn txs are LP tokens burned
    BigInteger totalLpMinted = BigInteger.ZERO;
    BigInteger totalLpBurned = BigInteger.ZERO;

    for (String txHash : acc.mintTxHashes) {
      BigInteger lpValue = acc.transferValuesByTx.get(txHash);
      if (lpValue != null) {
        totalLpMinted = totalLpMinted.add(lpValue);
      }
    }
    for (String txHash : acc.burnTxHashes) {
      BigInteger lpValue = acc.transferValuesByTx.get(txHash);
      if (lpValue != null) {
        totalLpBurned = totalLpBurned.add(lpValue);
      }
    }

    Set<String> uniqueProviders = new HashSet<>(acc.mintProviders);
    uniqueProviders.addAll(acc.burnProviders);

    Set<String> churnedProviders = new HashSet<>(acc.mintProviders);
    churnedProviders.retainAll(acc.burnProviders);

    return new LiquidityAnalytics(
        buildWindowId(pairAddress, windowStart, windowEnd),
        windowStart, windowEnd,
        acc.chain != null ? acc.chain : "",
        acc.chainName != null ? acc.chainName : "",
        pairAddress,
        acc.token0Symbol, acc.token1Symbol,
        acc.mintCount, acc.burnCount,
        acc.totalMintAmount0.toString(), acc.totalMintAmount1.toString(),
        acc.totalBurnAmount0.toString(), acc.totalBurnAmount1.toString(),
        acc.totalMintAmount0.subtract(acc.totalBurnAmount0).toString(),
        acc.totalMintAmount1.subtract(acc.totalBurnAmount1).toString(),
        totalLpMinted.toString(),
        totalLpBurned.toString(),
        totalLpMinted.subtract(totalLpBurned).toString(),
        acc.transferCount,
        acc.mintProviders.size(), acc.burnProviders.size(),
        uniqueProviders.size(), churnedProviders.size(),
        Instant.now().toEpochMilli()
    );
  }

    @Override
    public void process(
            String pairAddress,
            Context context,
            Iterable<DexEvent> events,
            Collector<LiquidityAnalytics> out
    ) {
        out.collect(summarize(
                pairAddress,
                context.window().getStart(),
                context.window().getEnd(),
            events
        ));
    }

  private static class Accumulator {

    int mintCount = 0;
    int burnCount = 0;
    int transferCount = 0;

    BigInteger totalMintAmount0 = BigInteger.ZERO;
    BigInteger totalMintAmount1 = BigInteger.ZERO;
    BigInteger totalBurnAmount0 = BigInteger.ZERO;
    BigInteger totalBurnAmount1 = BigInteger.ZERO;

    Set<String> mintProviders = new HashSet<>();
    Set<String> burnProviders = new HashSet<>();

    Set<String> mintTxHashes = new HashSet<>();
    Set<String> burnTxHashes = new HashSet<>();

    Map<String, BigInteger> transferValuesByTx = new HashMap<>();

    String token0Symbol;
    String token1Symbol;
    String chain;
    String chainName;
    }
}
