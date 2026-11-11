package com.web3analytics.analytics.store

import com.fasterxml.jackson.databind.ObjectMapper
import com.web3analytics.analytics.config.AppConfig
import com.web3analytics.analytics.model.CrossChainFlowResult
import com.web3analytics.analytics.model.ExploitDetectionResult
import com.web3analytics.analytics.model.ReentrancyResult
import com.web3analytics.analytics.model.SupplyTrackingResult
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.model.TransferVolumeResult
import org.slf4j.LoggerFactory
import redis.clients.jedis.JedisPool
import redis.clients.jedis.JedisPoolConfig

class RedisBridgeStore(
    private val config: AppConfig,
    private val mapper: ObjectMapper,
) : BridgeStore {
    private val logger = LoggerFactory.getLogger(RedisBridgeStore::class.java)
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
        logger.info("Bridge store connected to Redis at {}:{}", config.redisHost, config.redisPort)
    }

    override fun storeTransferVolume(result: TransferVolumeResult) {
        pool.resource.use { jedis ->
            val key = transferVolumeKey(result.chain, result.bridgeAddress)
            val json = mapper.writeValueAsString(result)
            jedis.zadd(key, result.windowStart.toDouble(), json)
            val cutoff = System.currentTimeMillis() - TRANSFER_VOLUME_RETENTION_HOURS * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun storeSupplyTracking(result: SupplyTrackingResult) {
        pool.resource.use { jedis ->
            val key = supplyTrackingKey(result.chain, result.bridgeAddress)
            val json = mapper.writeValueAsString(result)
            jedis.zadd(key, result.windowStart.toDouble(), json)
            val cutoff = System.currentTimeMillis() - SUPPLY_TRACKING_RETENTION_HOURS * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun storeCrossChainFlow(result: CrossChainFlowResult) {
        pool.resource.use { jedis ->
            val key = crossChainFlowKey(result.bridgeAddress)
            val json = mapper.writeValueAsString(result)
            jedis.zadd(key, result.windowStart.toDouble(), json)
            val cutoff = System.currentTimeMillis() - CROSS_CHAIN_FLOW_RETENTION_HOURS * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun storeExploitDetection(result: ExploitDetectionResult) {
        pool.resource.use { jedis ->
            val key = exploitDetectionKey(result.bridgeAddress, result.chain)
            val json = mapper.writeValueAsString(result)
            jedis.zadd(key, result.sessionStart.toDouble(), json)
            val cutoff = System.currentTimeMillis() - EXPLOIT_DETECTION_RETENTION_HOURS * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun storeReentrancy(result: ReentrancyResult) {
        pool.resource.use { jedis ->
            val key = REENTRANCY_KEY
            val json = mapper.writeValueAsString(result)
            jedis.zadd(key, result.windowStart.toDouble(), json)
            val cutoff = System.currentTimeMillis() - REENTRANCY_RETENTION_HOURS * 3600_000L
            jedis.zremrangeByScore(key, Double.NEGATIVE_INFINITY, cutoff.toDouble())
        }
    }

    override fun getTransferVolume(
        chain: String,
        bridgeAddress: String,
        range: TimeRange,
    ): List<TransferVolumeResult> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    transferVolumeKey(chain, bridgeAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
                )
            return results.map { mapper.readValue(it, TransferVolumeResult::class.java) }
        }
    }

    override fun getSupplyTracking(
        chain: String,
        bridgeAddress: String,
        range: TimeRange,
    ): List<SupplyTrackingResult> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    supplyTrackingKey(chain, bridgeAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
                )
            return results.map { mapper.readValue(it, SupplyTrackingResult::class.java) }
        }
    }

    override fun getCrossChainFlow(
        bridgeAddress: String,
        range: TimeRange,
    ): List<CrossChainFlowResult> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    crossChainFlowKey(bridgeAddress),
                    range.from.toDouble(),
                    range.to.toDouble(),
                )
            return results.map { mapper.readValue(it, CrossChainFlowResult::class.java) }
        }
    }

    override fun getExploitAlerts(
        bridgeAddress: String,
        chain: String,
        range: TimeRange,
    ): List<ExploitDetectionResult> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    exploitDetectionKey(bridgeAddress, chain),
                    range.from.toDouble(),
                    range.to.toDouble(),
                )
            return results.map { mapper.readValue(it, ExploitDetectionResult::class.java) }
        }
    }

    override fun getReentrancyDetections(range: TimeRange): List<ReentrancyResult> {
        pool.resource.use { jedis ->
            val results =
                jedis.zrangeByScore(
                    REENTRANCY_KEY,
                    range.from.toDouble(),
                    range.to.toDouble(),
                )
            return results.map { mapper.readValue(it, ReentrancyResult::class.java) }
        }
    }

    companion object {
        private const val TRANSFER_VOLUME_RETENTION_HOURS = 24
        private const val SUPPLY_TRACKING_RETENTION_HOURS = 24
        private const val CROSS_CHAIN_FLOW_RETENTION_HOURS = 24
        private const val EXPLOIT_DETECTION_RETENTION_HOURS = 168 // 7 days
        private const val REENTRANCY_RETENTION_HOURS = 168 // 7 days

        private const val REENTRANCY_KEY = "bridges:reentrancy"

        private fun transferVolumeKey(
            chain: String,
            bridgeAddress: String,
        ) = "bridges:transfer-volume:$chain:$bridgeAddress"

        private fun supplyTrackingKey(
            chain: String,
            bridgeAddress: String,
        ) = "bridges:supply:$chain:$bridgeAddress"

        private fun crossChainFlowKey(bridgeAddress: String) = "bridges:flow:$bridgeAddress"

        private fun exploitDetectionKey(
            bridgeAddress: String,
            chain: String,
        ) = "bridges:alerts:$bridgeAddress:$chain"
    }
}
