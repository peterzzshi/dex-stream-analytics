package com.web3analytics.analytics.store

import com.web3analytics.analytics.model.LiquidityAnalytics
import com.web3analytics.analytics.model.MarketTrend
import com.web3analytics.analytics.model.MevAlert
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TradingAnalytics
import com.web3analytics.analytics.model.TimeRange

interface AnalyticsStore {
    fun storeTradingAnalytics(analytics: TradingAnalytics)
    fun storeLiquidityAnalytics(analytics: LiquidityAnalytics)
    fun storeMevAlert(alert: MevAlert)
    fun storeMarketTrend(trend: MarketTrend)

    fun getTradingWindows(pairAddress: PairAddress, range: TimeRange): List<TradingAnalytics>
    fun getLatestTradingWindow(pairAddress: PairAddress): TradingAnalytics?
    fun getLiquidityWindows(pairAddress: PairAddress, range: TimeRange): List<LiquidityAnalytics>
    fun getLatestLiquidityWindow(pairAddress: PairAddress): LiquidityAnalytics?
    fun getMevAlerts(pairAddress: PairAddress, range: TimeRange): List<MevAlert>
    fun getMarketTrends(pairAddress: PairAddress, range: TimeRange): List<MarketTrend>
    fun getLatestMarketTrend(pairAddress: PairAddress): MarketTrend?

    fun tradingPairCount(): Int
    fun liquidityPairCount(): Int
    fun totalTradingWindows(): Long
    fun totalLiquidityWindows(): Long
    fun totalMevAlerts(): Long
    fun totalMarketTrends(): Long

    fun trackedTradingPairs(): Set<PairAddress>
    fun trackedLiquidityPairs(): Set<PairAddress>
    fun allTrackedPairs(): Set<PairAddress>
}
