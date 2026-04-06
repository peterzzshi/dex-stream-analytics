package com.web3analytics.models;

/**
 * Burn event from dex-liquidity-events topic.
 * Represents liquidity removal (token amounts removed).
 */
public record BurnEvent(
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
        String amount0,
        String amount1,
        long eventTimestamp
) implements DexEvent {}
