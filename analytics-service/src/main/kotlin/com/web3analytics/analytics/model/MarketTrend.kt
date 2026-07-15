package com.web3analytics.analytics.model

import com.fasterxml.jackson.annotation.JsonIgnoreProperties

@JsonIgnoreProperties(ignoreUnknown = true)
data class MarketTrend(
    val windowId: String,
    val windowStart: Long = 0,
    val windowEnd: Long = 0,
    val pairAddress: PairAddress,
    val token0Symbol: String? = null,
    val token1Symbol: String? = null,
    val avgPrice: Double = 0.0,
    val openPrice: Double = 0.0,
    val closePrice: Double = 0.0,
    val priceChangePercent: Double = 0.0,
    val volumeUSD: Double? = null,
    val swapCount: Int = 0,
    val uniqueTraders: Int = 0,
    val volatility: Double = 0.0,
    val trend: String = "NEUTRAL",
    val processedAt: Long = 0
)
