package com.web3analytics.alerting.config

import redis.clients.jedis.JedisPool
import redis.clients.jedis.JedisPoolConfig

data class AppConfig(
    val redisHost: String = System.getenv("REDIS_HOST") ?: "localhost",
    val redisPort: Int = System.getenv("REDIS_PORT")?.toIntOrNull() ?: 6379,
    val alertRetentionLimit: Int = System.getenv("ALERT_RETENTION_LIMIT")?.toIntOrNull() ?: 100,
    val wsKeepaliveSeconds: Long = System.getenv("WS_KEEPALIVE_SECONDS")?.toLongOrNull() ?: 30,
    val schemasPath: String = System.getenv("SCHEMAS_PATH") ?: "/app/schemas"
) {
    fun createJedisPool(): JedisPool {
        val poolConfig =
            JedisPoolConfig().apply {
                maxTotal = 16
                maxIdle = 8
                minIdle = 2
            }
        return JedisPool(poolConfig, redisHost, redisPort)
    }

    fun validateRedisConnection(pool: JedisPool) {
        pool.resource.use { jedis ->
            val pong = jedis.ping()
            require(pong == "PONG") { "Redis ping failed: expected PONG, got $pong" }
        }
    }
}
