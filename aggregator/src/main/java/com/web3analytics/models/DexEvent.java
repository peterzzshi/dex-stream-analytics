package com.web3analytics.models;

/**
 * Sealed interface for all DEX events from blockchain.
 *
 * Enables exhaustive pattern matching (Java 17+) and provides
 * common field access across all event types.
 */
public sealed interface DexEvent permits SwapEvent, MintEvent, BurnEvent {

    // Common fields across all events
    String eventId();
    long blockNumber();
    long blockTimestamp();
    String transactionHash();
    int logIndex();
    String pairAddress();
    String token0();
    String token1();
    long eventTimestamp();

    /**
     * Get event timestamp in milliseconds (for Flink watermarks).
     */
    default long getEventTimeMillis() {
        return blockTimestamp() * 1000L;
    }
}
