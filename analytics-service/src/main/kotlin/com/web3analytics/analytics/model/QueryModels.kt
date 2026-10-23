package com.web3analytics.analytics.model

data class TimeRange(
    val from: Long,
    val to: Long
) {
    init {
        require(from <= to) { "from ($from) must be <= to ($to)" }
    }
}

data class PairSummary(
    val pairAddress: PairAddress,
    val token0Symbol: String?,
    val token1Symbol: String?,
    val latestTwap: Double?,
    val latestVolumeUSD: Double?,
    val windowCount: Int
)

data class AnalyticsSummary(
    val tradingPairsTracked: Int,
    val liquidityPairsTracked: Int,
    val totalTradingWindows: Long,
    val totalLiquidityWindows: Long
)
