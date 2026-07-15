package com.web3analytics.analytics.model

import com.fasterxml.jackson.annotation.JsonIgnoreProperties

@JsonIgnoreProperties(ignoreUnknown = true)
data class MevAlert(
    val alertId: String,
    val alertType: String,
    val windowStart: Long = 0,
    val windowEnd: Long = 0,
    val pairAddress: PairAddress,
    val token0Symbol: String? = null,
    val token1Symbol: String? = null,
    val blockNumber: Long = 0,
    val attackerAddress: String? = null,
    val victimAddresses: List<String> = emptyList(),
    val estimatedProfitUSD: Double = 0.0,
    val involvedSwapCount: Int = 0,
    val involvedEventCount: Int = 0,
    val severity: String = "LOW",
    val description: String? = null,
    val involvedTransactions: List<String> = emptyList(),
    val detectedAt: Long = 0
)
