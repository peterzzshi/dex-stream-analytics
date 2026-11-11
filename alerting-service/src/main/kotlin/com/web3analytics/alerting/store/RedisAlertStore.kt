package com.web3analytics.alerting.store

import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.module.kotlin.readValue
import com.web3analytics.alerting.model.Alert
import com.web3analytics.alerting.model.CrossChainFlowAlert
import com.web3analytics.alerting.model.ExploitDetectionAlert
import com.web3analytics.alerting.model.ReentrancyAlert
import com.web3analytics.alerting.model.SupplyTrackingAlert
import com.web3analytics.alerting.model.TransferVolumeAlert
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import redis.clients.jedis.JedisPool
import redis.clients.jedis.params.ZAddParams

class RedisAlertStore(
    private val jedisPool: JedisPool,
    private val mapper: ObjectMapper,
    private val retentionLimit: Int = 100
) : AlertStore {
    override suspend fun storeAlert(alert: Alert): Unit =
        withContext(Dispatchers.IO) {
            val key = buildKey(alert)
            val score = alert.windowEnd.toEpochMilli().toDouble()
            val json = mapper.writeValueAsString(alert)

            jedisPool.resource.use { jedis ->
                val params = ZAddParams().gt()
                jedis.zadd(key, score, json, params)
                jedis.zremrangeByRank(key, 0, -(retentionLimit + 1).toLong())
            }
        }

    override suspend fun getRecentAlerts(
        bridgeAddress: String,
        chain: String,
        limit: Int
    ): List<Alert> =
        withContext(Dispatchers.IO) {
            val key = "bridge:alerts:$bridgeAddress:$chain"
            jedisPool.resource.use { jedis ->
                val jsons = jedis.zrevrange(key, 0, (limit - 1).toLong())
                jsons.mapNotNull { json ->
                    try {
                        deserializeAlert(json)
                    } catch (e: Exception) {
                        null
                    }
                }
            }
        }

    private fun buildKey(alert: Alert): String {
        val chain =
            when (alert) {
                is TransferVolumeAlert -> alert.chain
                is SupplyTrackingAlert -> alert.chain
                is ExploitDetectionAlert -> alert.chain
                is ReentrancyAlert -> alert.chain
                is CrossChainFlowAlert -> "cross-chain"
            }
        return "bridge:alerts:${alert.bridgeAddress}:$chain"
    }

    private fun deserializeAlert(json: String): Alert {
        val tree = mapper.readTree(json)
        val alertType = tree["alertType"].asText()
        return when (alertType) {
            "TRANSFER_VOLUME" -> mapper.readValue<TransferVolumeAlert>(json)
            "SUPPLY_TRACKING" -> mapper.readValue<SupplyTrackingAlert>(json)
            "CROSS_CHAIN_FLOW" -> mapper.readValue<CrossChainFlowAlert>(json)
            "EXPLOIT_DETECTION" -> mapper.readValue<ExploitDetectionAlert>(json)
            "REENTRANCY" -> mapper.readValue<ReentrancyAlert>(json)
            else -> throw IllegalArgumentException("Unknown alert type: $alertType")
        }
    }
}
