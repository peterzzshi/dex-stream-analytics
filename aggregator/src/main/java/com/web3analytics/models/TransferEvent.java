package com.web3analytics.models;

/**
 * Transfer event from dex-liquidity-events topic.
 * Represents LP token movement on the pair contract token.
 */
public record TransferEvent(
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
        String from,
        String to,
        String value,
        long eventTimestamp
) implements DexEvent {}
