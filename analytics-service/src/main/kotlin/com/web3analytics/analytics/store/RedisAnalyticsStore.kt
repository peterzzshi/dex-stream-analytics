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
import redis.clients.jedis.Jedis
import redis.clients.jedis.JedisPool
import redis.clients.jedis.JedisPoolConfig

private data class WindowWrite(
    val key: String,
    val pairsKey: String,
    val countKey: String,
    val score: Double,
    val pairAddress: PairAddress,
    val retentionHours: Int,
    val json: String,
)

class RedisAnalyticsStore(
    private val config: AppConfig,
    private val mapper: ObjectMapper
) : AnalyticsStore {
    private val logger = LoggerFactory.getLogger(RedisAnalyticsStore::class.java)
    private val pool: JedisPool

    init {
        val poolConfig =
            JedisPoolConfig().apply {
                maxTotal = 16
                maxIdle = 8
                minIdle = 2
                testOnBorrow = true
            }
        pool =
            if (config.redisPassword.isNullOrBlank()) {
                JedisPool(poolConfig, config.redisHost, config.redisPort)
            } else {
                JedisPool(poolConfig, config.redisHost, config.redisPort, 2000, config.redisPassword)
            }
        logger.info("Redis store connected to {}:{}", config.redisHost, config.redisPort)
    }

    /**
     * Shared write path for every "windowed" model: trim entries older than
     * [retentionHours], add the new one, and track its pair + running count.
     * The only thing that varies per call site is which keys/score/retention
     * to use, so those are passed in rather than duplicating the four Redis
     * calls per model.
     */
    private fun writeWindow(
        jedis: Jedis,
        write: WindowWrite,
    ) {
        val cutoff = System.currentTimeMillis() - write.retentionHours * 3600_000L
        jedis.zremrangeByScore(write.key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        jedis.zadd(write.key, write.score, write.json)
        jedis.sadd(write.pairsKey, write.pairAddress.value)
        jedis.incr(write.countKey)
    }

    override fun storeTradingAnalytics(analytics: TradingAnalytics) {
        pool.resource.use { jedis ->
            writeWindow(
                jedis,
                WindowWrite(
                    key = tradingKey(analytics.pairAddress),
                    pairsKey = TRADING_PAIRS_KEY,
                    countKey = TRADING_WINDOW_COUNT_KEY,
                    score = analytics.windowStart.toDouble(),
                    pairAddress = analytics.pairAddress,
                    retentionHours = config.tradingRetentionHours,
                    json = mapper.writeValueAsString(analytics),
                ),
            )
        }
    }

    override fun storeLiquidityAnalytics(analytics: LiquidityAnalytics) {
        pool.resource.use { jedis ->
            writeWindow(
                jedis,
                WindowWrite(
                    key = liquidityKey(analytics.pairAddress),
                    pairsKey = LIQUIDITY_PAIRS_KEY,
                    countKey = LIQUIDITY_WINDOW_COUNT_KEY,
                    score = analytics.windowStart.toDouble(),
                    pairAddress = analytics.pairAddress,
                    retentionHours = config.liquidityRetentionHours,
                    json = mapper.writeValueAsString(analytics),
                ),
            )
        }
    }

    override fun getTradingWindows(
        pairAddress: PairAddress,
        range: TimeRange,
    ): List<TradingAnalytics> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    tradingKey(pairAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
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

    override fun tradingWindowCount(
        pairAddress: PairAddress,
        range: TimeRange,
    ): Int {
        pool.resource.use { jedis ->
            return jedis.zcount(tradingKey(pairAddress), range.from.toDouble(), range.to.toDouble()).toInt()
        }
    }

    override fun getLiquidityWindows(
        pairAddress: PairAddress,
        range: TimeRange,
    ): List<LiquidityAnalytics> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    liquidityKey(pairAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
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
            writeWindow(
                jedis,
                WindowWrite(
                    key = mevKey(alert.pairAddress),
                    pairsKey = MEV_PAIRS_KEY,
                    countKey = MEV_ALERT_COUNT_KEY,
                    score = alert.detectedAt.toDouble(),
                    pairAddress = alert.pairAddress,
                    retentionHours = config.mevRetentionHours,
                    json = mapper.writeValueAsString(alert),
                ),
            )
        }
    }

    override fun storeMarketTrend(trend: MarketTrend) {
        pool.resource.use { jedis ->
            writeWindow(
                jedis,
                WindowWrite(
                    key = trendKey(trend.pairAddress),
                    pairsKey = TREND_PAIRS_KEY,
                    countKey = TREND_WINDOW_COUNT_KEY,
                    score = trend.windowStart.toDouble(),
                    pairAddress = trend.pairAddress,
                    retentionHours = config.trendRetentionHours,
                    json = mapper.writeValueAsString(trend),
                ),
            )
        }
    }

    override fun getMevAlerts(
        pairAddress: PairAddress,
        range: TimeRange,
    ): List<MevAlert> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    mevKey(pairAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
                )
            return results
                .map { mapper.readValue(it, MevAlert::class.java) }
                .sortedByDescending { it.detectedAt }
        }
    }

    override fun getMarketTrends(
        pairAddress: PairAddress,
        range: TimeRange,
    ): List<MarketTrend> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    trendKey(pairAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
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
