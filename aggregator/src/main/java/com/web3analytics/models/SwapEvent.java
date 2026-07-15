package com.web3analytics.models;

/**
 * Swap event from dex-trading-events topic.
 * Represents a token swap (trade) on the DEX pair.
 */
public record SwapEvent(
        String eventId,
        long blockNumber,
        long blockTimestamp,
        String transactionHash,
        int logIndex,
        String pairAddress,
        String token0,
        String token1,
        String token0Symbol,
        String token1Symbol,
        String sender,
        String recipient,
        String amount0In,
        String amount1In,
        String amount0Out,
        String amount1Out,
        double price,
        Double volumeUSD,
        long gasUsed,
        String gasPrice,
        long eventTimestamp
) implements DexEvent {}
