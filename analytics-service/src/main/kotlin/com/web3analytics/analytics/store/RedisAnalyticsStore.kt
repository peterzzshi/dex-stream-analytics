package com.web3analytics.analytics.store

import com.fasterxml.jackson.databind.ObjectMapper
import com.web3analytics.analytics.config.AppConfig
import com.web3analytics.analytics.model.LiquidityAnalytics
import com.web3analytics.analytics.model.MarketTrend
import com.web3analytics.analytics.model.MevAlert
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.model.TradingAnalytics
import org.slf4j.LoggerFactory
import redis.clients.jedis.JedisPool
import redis.clients.jedis.JedisPoolConfig

class RedisAnalyticsStore(
    private val config: AppConfig,
    private val mapper: ObjectMapper
) : AnalyticsStore {

    private val logger = LoggerFactory.getLogger(RedisAnalyticsStore::class.java)
    private val pool: JedisPool

    init {
        val poolConfig = JedisPoolConfig().apply {
            maxTotal = 16
            maxIdle = 8
            minIdle = 2
            testOnBorrow = true
        }
        pool = if (config.redisPassword.isNullOrBlank()) {
            JedisPool(poolConfig, config.redisHost, config.redisPort)
        } else {
            JedisPool(poolConfig, config.redisHost, config.redisPort, 2000, config.redisPassword)
        }
        logger.info("Redis store connected to {}:{}", config.redisHost, config.redisPort)
    }

    override fun storeTradingAnalytics(analytics: TradingAnalytics) {
        pool.resource.use { jedis ->
            val key = tradingKey(analytics.pairAddress)
            val json = mapper.writeValueAsString(analytics)
            jedis.zadd(key, analytics.windowStart.toDouble(), json)
            jedis.sadd(TRADING_PAIRS_KEY, analytics.pairAddress.value)
            jedis.incr(TRADING_WINDOW_COUNT_KEY)

            // TTL: expire individual entries beyond retention
            val cutoff = System.currentTimeMillis() - config.tradingRetentionHours * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun storeLiquidityAnalytics(analytics: LiquidityAnalytics) {
        pool.resource.use { jedis ->
            val key = liquidityKey(analytics.pairAddress)
            val json = mapper.writeValueAsString(analytics)
            jedis.zadd(key, analytics.windowStart.toDouble(), json)
            jedis.sadd(LIQUIDITY_PAIRS_KEY, analytics.pairAddress.value)
            jedis.incr(LIQUIDITY_WINDOW_COUNT_KEY)

            val cutoff = System.currentTimeMillis() - config.liquidityRetentionHours * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun getTradingWindows(pairAddress: PairAddress, range: TimeRange): List<TradingAnalytics> {
        pool.resource.use { jedis ->
            val results = jedis.zrangeByScore(
                tradingKey(pairAddress),
                range.from.toDouble(),
                range.to.toDouble()
            )
            return results.map { mapper.readValue(it, TradingAnalytics::class.java) }
        }
    }

    override fun getLatestTradingWindow(pairAddress: PairAddress): TradingAnalytics? {
        pool.resource.use { jedis ->
            val results = jedis.zrevrange(tradingKey(pairAddress), 0, 0)
            return results.firstOrNull()?.let { mapper.readValue(it, TradingAnalytics::class.java) }
        }
    }

    override fun getLiquidityWindows(pairAddress: PairAddress, range: TimeRange): List<LiquidityAnalytics> {
        pool.resource.use { jedis ->
            val results = jedis.zrangeByScore(
                liquidityKey(pairAddress),
                range.from.toDouble(),
                range.to.toDouble()
            )
            return results.map { mapper.readValue(it, LiquidityAnalytics::class.java) }
        }
    }

    override fun getLatestLiquidityWindow(pairAddress: PairAddress): LiquidityAnalytics? {
        pool.resource.use { jedis ->
            val results = jedis.zrevrange(liquidityKey(pairAddress), 0, 0)
            return results.firstOrNull()?.let { mapper.readValue(it, LiquidityAnalytics::class.java) }
        }
    }

    override fun tradingPairCount(): Int {
        pool.resource.use { jedis -> return jedis.scard(TRADING_PAIRS_KEY).toInt() }
    }

    override fun liquidityPairCount(): Int {
        pool.resource.use { jedis -> return jedis.scard(LIQUIDITY_PAIRS_KEY).toInt() }
    }

    override fun totalTradingWindows(): Long {
        pool.resource.use { jedis -> return jedis.get(TRADING_WINDOW_COUNT_KEY)?.toLongOrNull() ?: 0L }
    }

    override fun totalLiquidityWindows(): Long {
        pool.resource.use { jedis -> return jedis.get(LIQUIDITY_WINDOW_COUNT_KEY)?.toLongOrNull() ?: 0L }
    }

    override fun trackedTradingPairs(): Set<PairAddress> {
        pool.resource.use { jedis -> return jedis.smembers(TRADING_PAIRS_KEY).map(::PairAddress).toSet() }
    }

    override fun trackedLiquidityPairs(): Set<PairAddress> {
        pool.resource.use { jedis -> return jedis.smembers(LIQUIDITY_PAIRS_KEY).map(::PairAddress).toSet() }
    }

    override fun storeMevAlert(alert: MevAlert) {
        pool.resource.use { jedis ->
            val key = mevKey(alert.pairAddress)
            val json = mapper.writeValueAsString(alert)
            jedis.zadd(key, alert.detectedAt.toDouble(), json)
            jedis.sadd(MEV_PAIRS_KEY, alert.pairAddress.value)
            jedis.incr(MEV_ALERT_COUNT_KEY)

            val cutoff = System.currentTimeMillis() - config.tradingRetentionHours * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun storeMarketTrend(trend: MarketTrend) {
        pool.resource.use { jedis ->
            val key = trendKey(trend.pairAddress)
            val json = mapper.writeValueAsString(trend)
            jedis.zadd(key, trend.windowStart.toDouble(), json)
            jedis.sadd(TREND_PAIRS_KEY, trend.pairAddress.value)
            jedis.incr(TREND_WINDOW_COUNT_KEY)

            val cutoff = System.currentTimeMillis() - config.tradingRetentionHours * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun getMevAlerts(pairAddress: PairAddress, range: TimeRange): List<MevAlert> {
        pool.resource.use { jedis ->
            val results = jedis.zrangeByScore(
                mevKey(pairAddress), range.from.toDouble(), range.to.toDouble()
            )
            return results.map { mapper.readValue(it, MevAlert::class.java) }
                .sortedByDescending { it.detectedAt }
        }
    }

    override fun getMarketTrends(pairAddress: PairAddress, range: TimeRange): List<MarketTrend> {
        pool.resource.use { jedis ->
            val results = jedis.zrangeByScore(
                trendKey(pairAddress), range.from.toDouble(), range.to.toDouble()
            )
            return results.map { mapper.readValue(it, MarketTrend::class.java) }
        }
    }

    override fun getLatestMarketTrend(pairAddress: PairAddress): MarketTrend? {
        pool.resource.use { jedis ->
            val results = jedis.zrevrange(trendKey(pairAddress), 0, 0)
            return results.firstOrNull()?.let { mapper.readValue(it, MarketTrend::class.java) }
        }
    }

    override fun totalMevAlerts(): Long {
        pool.resource.use { jedis -> return jedis.get(MEV_ALERT_COUNT_KEY)?.toLongOrNull() ?: 0L }
    }

    override fun totalMarketTrends(): Long {
        pool.resource.use { jedis -> return jedis.get(TREND_WINDOW_COUNT_KEY)?.toLongOrNull() ?: 0L }
    }

    override fun allTrackedPairs(): Set<PairAddress> {
        pool.resource.use { jedis ->
            val pipeline = jedis.pipelined()
            val trading = pipeline.smembers(TRADING_PAIRS_KEY)
            val liquidity = pipeline.smembers(LIQUIDITY_PAIRS_KEY)
            val mev = pipeline.smembers(MEV_PAIRS_KEY)
            val trend = pipeline.smembers(TREND_PAIRS_KEY)
            pipeline.sync()
            return (trading.get() + liquidity.get() + mev.get() + trend.get())
                .map(::PairAddress)
                .toSet()
        }
    }

    companion object {
        private const val TRADING_PAIRS_KEY = "analytics:trading:pairs"
        private const val LIQUIDITY_PAIRS_KEY = "analytics:liquidity:pairs"
        private const val MEV_PAIRS_KEY = "analytics:mev:pairs"
        private const val TREND_PAIRS_KEY = "analytics:trends:pairs"
        private const val TRADING_WINDOW_COUNT_KEY = "analytics:trading:window_count"
        private const val LIQUIDITY_WINDOW_COUNT_KEY = "analytics:liquidity:window_count"
        private const val MEV_ALERT_COUNT_KEY = "analytics:mev:alert_count"
        private const val TREND_WINDOW_COUNT_KEY = "analytics:trends:window_count"

        private fun tradingKey(pair: PairAddress) = "analytics:trading:${pair.value}"
        private fun liquidityKey(pair: PairAddress) = "analytics:liquidity:${pair.value}"
        private fun mevKey(pair: PairAddress) = "analytics:mev:${pair.value}"
        private fun trendKey(pair: PairAddress) = "analytics:trends:${pair.value}"
    }
}
