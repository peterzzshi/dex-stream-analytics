package com.web3analytics.analytics.model

/**
 * Transfer volume aggregation from Flink (5-min tumbling window).
 * Maps to TransferVolumeResult in Java aggregator.
 */
data class TransferVolumeResult(
    val chain: String,
    val bridgeAddress: String,
    val windowStart: Long,
    val windowEnd: Long,
    val lockCount: Long,
    val unlockCount: Long,
    val totalLocked: String,
    val totalUnlocked: String,
)

/**
 * Supply tracking aggregation from Flink (5-min tumbling window).
 * Maps to SupplyTrackingResult in Java aggregator.
 */
data class SupplyTrackingResult(
    val chain: String,
    val bridgeAddress: String,
    val windowStart: Long,
    val windowEnd: Long,
    val netMinted: String,
    val totalWrappedSupply: String,
)

/**
 * Cross-chain flow aggregation from Flink (5-min tumbling window).
 * Maps to CrossChainFlowResult in Java aggregator.
 */
data class CrossChainFlowResult(
    val bridgeAddress: String,
    val windowStart: Long,
    val windowEnd: Long,
    val netFlow: String,
    val imbalanceRatio: Double,
)

/**
 * Exploit detection from Flink (60-sec session window).
 * Maps to ExploitDetectionResult in Java aggregator.
 */
data class ExploitDetectionResult(
    val bridgeAddress: String,
    val chain: String,
    val sessionStart: Long,
    val sessionEnd: Long,
    val eventCount: Long,
    val patterns: List<String>,
    val netLocked: String,
    val netMinted: String,
)

/**
 * Reentrancy detection from Flink (5-sec tumbling window).
 * Maps to ReentrancyResult in Java aggregator.
 */
data class ReentrancyResult(
    val transactionHash: String,
    val chain: String,
    val bridgeAddress: String,
    val windowStart: Long,
    val windowEnd: Long,
    val lockCount: Int,
    val unlockCount: Int,
)
