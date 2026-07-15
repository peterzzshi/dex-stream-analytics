package com.web3analytics.analytics.health

import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.PoolHealth
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.store.AnalyticsStore
import kotlin.math.min

class PoolHealthScorer(private val store: AnalyticsStore) {

    fun evaluate(pairAddress: PairAddress): PoolHealth {
        val now = System.currentTimeMillis()
        val range24h = TimeRange(now - 86_400_000, now)

        val tradingWindows = store.getTradingWindows(pairAddress, range24h)
        val liquidityWindows = store.getLiquidityWindows(pairAddress, range24h)
        val alerts = store.getMevAlerts(pairAddress, range24h)
        val trends = store.getMarketTrends(pairAddress, range24h)

        val volumeUSD = tradingWindows.mapNotNull { it.volumeUSD }.sum()
        val uniqueTraders = tradingWindows.map { it.uniqueTraders }.maxOrNull() ?: 0
        val latestTrend = store.getLatestMarketTrend(pairAddress)

        val tradingScore = scoreTradingActivity(tradingWindows.size, volumeUSD, uniqueTraders)
        val liquidityScore = scoreLiquidity(liquidityWindows.size)
        val safetyScore = scoreSafety(alerts.size)

        return PoolHealth.compute(
            pairAddress = pairAddress,
            tradingScore = tradingScore,
            liquidityScore = liquidityScore,
            safetyScore = safetyScore,
            trend = latestTrend?.trend ?: "UNKNOWN",
            recentAlertCount = alerts.size,
            volumeUSD24h = volumeUSD,
            uniqueTraders24h = uniqueTraders
        )
    }

    companion object {
        fun scoreTradingActivity(windowCount: Int, volumeUSD: Double, uniqueTraders: Int): Double {
            // More windows = more active trading
            val activityScore = min(windowCount / 24.0, 1.0)
            // Volume diversity: >$100k/day is healthy
            val volumeScore = min(volumeUSD / 100_000.0, 1.0)
            // Trader diversity: >10 unique traders is healthy
            val traderScore = min(uniqueTraders / 10.0, 1.0)
            return (activityScore * 0.3 + volumeScore * 0.4 + traderScore * 0.3)
        }

        fun scoreLiquidity(windowCount: Int): Double {
            if (windowCount == 0) return 0.5 // neutral when no data
            // Having liquidity events is positive (active pool)
            return min(windowCount / 12.0, 1.0)
        }

        fun scoreSafety(alertCount: Int): Double {
            // No alerts = perfect safety; each alert degrades the score
            return when {
                alertCount == 0 -> 1.0
                alertCount <= 2 -> 0.7
                alertCount <= 5 -> 0.4
                else -> 0.1
            }
        }
    }
}
