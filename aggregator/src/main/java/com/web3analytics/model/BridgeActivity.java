package com.web3analytics.model;

import java.math.BigInteger;

/**
 * Normalized view of one of the four bridge event types, built once at the union boundary so
 * downstream {@code AggregateFunction}s branch on an explicit {@code kind} instead of null-checking
 * a positional tuple slot.
 */
public record BridgeActivity(
    EventKind kind,
    String bridgeAddress,
    String chain,
    String protocol,
    BigInteger amount) {
}
