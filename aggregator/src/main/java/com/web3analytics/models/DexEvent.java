package com.web3analytics.models;

/** Sealed sum type for all blockchain events. Enables exhaustive pattern matching. */
public sealed interface DexEvent permits SwapEvent, MintEvent, BurnEvent, TransferEvent {

    String eventId();
    long blockNumber();
    long blockTimestamp();
    String transactionHash();
    int logIndex();
    String pairAddress();
    String token0();
    String token1();
    long eventTimestamp();

    /** Block timestamp in millis for Flink watermarks (blocks are in seconds). */
    default long getEventTimeMillis() {
        return blockTimestamp() * 1000L;
    }
}
