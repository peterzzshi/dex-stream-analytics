package com.web3analytics.analytics.model

import com.fasterxml.jackson.annotation.JsonIgnoreProperties

@JsonIgnoreProperties(ignoreUnknown = true)
data class LiquidityAnalytics(
    val windowId: String,
    val windowStart: Long,
    val windowEnd: Long,
    val pairAddress: PairAddress,
    val token0Symbol: String? = null,
    val token1Symbol: String? = null,
    val mintCount: Int = 0,
    val burnCount: Int = 0,
    val totalMintAmount0: String? = null,
    val totalMintAmount1: String? = null,
    val totalBurnAmount0: String? = null,
    val totalBurnAmount1: String? = null,
    val netLiquidityChange0: String? = null,
    val netLiquidityChange1: String? = null,
    val totalLpTokensMinted: String? = null,
    val totalLpTokensBurned: String? = null,
    val netLpTokenChange: String? = null,
    val transferCount: Int = 0,
    val uniqueMintProviders: Int = 0,
    val uniqueBurnProviders: Int = 0,
    val uniqueProviders: Int = 0,
    val churnedProviders: Int = 0,
    val processedAt: Long = 0
)
