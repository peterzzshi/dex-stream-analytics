package com.web3analytics.models;

/**
 * Mint event from dex-liquidity-events topic.
 * Represents liquidity provision (token amounts added).
 */
public record MintEvent(
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
        String amount0,
        String amount1,
        long eventTimestamp
) implements DexEvent {}
