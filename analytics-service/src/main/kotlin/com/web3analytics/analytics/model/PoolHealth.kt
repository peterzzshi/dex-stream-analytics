package com.web3analytics.analytics.model

data class PoolHealth(
    val pairAddress: PairAddress,
    val overallScore: Double,
    val tradingScore: Double,
    val liquidityScore: Double,
    val trend: String,
    val volumeUSD24h: Double,
    val uniqueTraders24h: Int,
    val evaluatedAt: Long
) {
    companion object {
        fun compute(
            pairAddress: PairAddress,
            tradingScore: Double,
            liquidityScore: Double,
            trend: String,
            volumeUSD24h: Double,
            uniqueTraders24h: Int
        ): PoolHealth {
            val overall = tradingScore * 0.5 + liquidityScore * 0.5
            return PoolHealth(
                pairAddress = pairAddress,
                overallScore = Math.round(overall * 100.0) / 100.0,
                tradingScore = tradingScore,
                liquidityScore = liquidityScore,
                trend = trend,
                volumeUSD24h = volumeUSD24h,
                uniqueTraders24h = uniqueTraders24h,
                evaluatedAt = System.currentTimeMillis()
            )
        }
    }
}
