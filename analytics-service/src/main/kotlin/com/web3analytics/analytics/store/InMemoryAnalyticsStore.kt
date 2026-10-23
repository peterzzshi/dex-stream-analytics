package com.web3analytics.analytics.store

import com.web3analytics.analytics.model.LiquidityAnalytics
import com.web3analytics.analytics.model.MarketTrend
import com.web3analytics.analytics.model.MevAlert
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.model.TradingAnalytics
import java.util.Collections
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

class InMemoryAnalyticsStore : AnalyticsStore {
    private val tradingByPair = ConcurrentHashMap<PairAddress, MutableList<TradingAnalytics>>()
    private val liquidityByPair = ConcurrentHashMap<PairAddress, MutableList<LiquidityAnalytics>>()
    private val mevByPair = ConcurrentHashMap<PairAddress, MutableList<MevAlert>>()
    private val trendsByPair = ConcurrentHashMap<PairAddress, MutableList<MarketTrend>>()
    private val tradingCount = AtomicLong(0)
    private val liquidityCount = AtomicLong(0)
    private val mevCount = AtomicLong(0)
    private val trendCount = AtomicLong(0)

    override fun storeTradingAnalytics(analytics: TradingAnalytics) {
        tradingByPair.computeIfAbsent(analytics.pairAddress) {
            Collections.synchronizedList(mutableListOf())
        }.add(analytics)
        tradingCount.incrementAndGet()
    }

    override fun storeLiquidityAnalytics(analytics: LiquidityAnalytics) {
        liquidityByPair.computeIfAbsent(analytics.pairAddress) {
            Collections.synchronizedList(mutableListOf())
        }.add(analytics)
        liquidityCount.incrementAndGet()
    }

    override fun storeMevAlert(alert: MevAlert) {
        mevByPair.computeIfAbsent(alert.pairAddress) {
            Collections.synchronizedList(mutableListOf())
        }.add(alert)
        mevCount.incrementAndGet()
    }

    override fun storeMarketTrend(trend: MarketTrend) {
        trendsByPair.computeIfAbsent(trend.pairAddress) {
            Collections.synchronizedList(mutableListOf())
        }.add(trend)
        trendCount.incrementAndGet()
    }

    override fun getTradingWindows(pairAddress: PairAddress, range: TimeRange): List<TradingAnalytics> {
        return tradingByPair[pairAddress]
            ?.filter { it.windowStart >= range.from && it.windowStart <= range.to }
            ?.sortedBy { it.windowStart }
            ?: emptyList()
    }

    override fun getLatestTradingWindow(pairAddress: PairAddress): TradingAnalytics? {
        return tradingByPair[pairAddress]?.maxByOrNull { it.windowStart }
    }

    override fun getLiquidityWindows(pairAddress: PairAddress, range: TimeRange): List<LiquidityAnalytics> {
        return liquidityByPair[pairAddress]
            ?.filter { it.windowStart >= range.from && it.windowStart <= range.to }
            ?.sortedBy { it.windowStart }
            ?: emptyList()
    }

    override fun getLatestLiquidityWindow(pairAddress: PairAddress): LiquidityAnalytics? {
        return liquidityByPair[pairAddress]?.maxByOrNull { it.windowStart }
    }

    override fun getMevAlerts(pairAddress: PairAddress, range: TimeRange): List<MevAlert> {
        return mevByPair[pairAddress]
            ?.filter { it.detectedAt >= range.from && it.detectedAt <= range.to }
            ?.sortedByDescending { it.detectedAt }
            ?: emptyList()
    }

    override fun getMarketTrends(pairAddress: PairAddress, range: TimeRange): List<MarketTrend> {
        return trendsByPair[pairAddress]
            ?.filter { it.windowStart >= range.from && it.windowStart <= range.to }
            ?.sortedBy { it.windowStart }
            ?: emptyList()
    }

    override fun getLatestMarketTrend(pairAddress: PairAddress): MarketTrend? {
        return trendsByPair[pairAddress]?.maxByOrNull { it.windowStart }
    }

    override fun tradingPairCount(): Int = tradingByPair.size
    override fun liquidityPairCount(): Int = liquidityByPair.size
    override fun totalTradingWindows(): Long = tradingCount.get()
    override fun totalLiquidityWindows(): Long = liquidityCount.get()
    override fun totalMevAlerts(): Long = mevCount.get()
    override fun totalMarketTrends(): Long = trendCount.get()

    override fun trackedTradingPairs(): Set<PairAddress> = tradingByPair.keys.toSet()
    override fun trackedLiquidityPairs(): Set<PairAddress> = liquidityByPair.keys.toSet()
    override fun allTrackedPairs(): Set<PairAddress> =
        tradingByPair.keys + liquidityByPair.keys + mevByPair.keys + trendsByPair.keys
}
