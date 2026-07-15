package com.web3analytics.analytics.model

import com.fasterxml.jackson.annotation.JsonIgnoreProperties

@JsonIgnoreProperties(ignoreUnknown = true)
data class TradingAnalytics(
    val windowId: String,
    val windowStart: Long,
    val windowEnd: Long,
    val pairAddress: PairAddress,
    val token0Symbol: String? = null,
    val token1Symbol: String? = null,
    val twap: Double = 0.0,
    val openPrice: Double = 0.0,
    val closePrice: Double = 0.0,
    val highPrice: Double = 0.0,
    val lowPrice: Double = 0.0,
    val priceVolatility: Double = 0.0,
    val totalVolume0: String? = null,
    val totalVolume1: String? = null,
    val volumeUSD: Double? = null,
    val swapCount: Int = 0,
    val uniqueTraders: Int = 0,
    val largestSwapValue: String? = null,
    val largestSwapAddress: String? = null,
    val totalGasUsed: Long = 0,
    val averageGasPrice: String? = null,
    val arbitrageCount: Int = 0,
    val repeatedTraders: List<String> = emptyList(),
    val processedAt: Long = 0
)
