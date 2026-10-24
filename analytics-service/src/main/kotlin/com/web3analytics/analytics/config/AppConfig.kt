package com.web3analytics.analytics.config

data class AppConfig(
    val port: Int = env("APP_PORT")?.toIntOrNull() ?: 8080,
    val redisHost: String = env("REDIS_HOST") ?: "localhost",
    val redisPort: Int = env("REDIS_PORT")?.toIntOrNull() ?: 6379,
    val redisPassword: String? = env("REDIS_PASSWORD"),
    val tradingRetentionHours: Int = env("TRADING_RETENTION_HOURS")?.toIntOrNull() ?: 168,
    val liquidityRetentionHours: Int = env("LIQUIDITY_RETENTION_HOURS")?.toIntOrNull() ?: 720,
    val mevRetentionHours: Int = env("MEV_RETENTION_HOURS")?.toIntOrNull() ?: 168,
    val trendRetentionHours: Int = env("TREND_RETENTION_HOURS")?.toIntOrNull() ?: 168,
) {
    companion object {
        private fun env(name: String): String? = System.getenv(name)

        fun load(): AppConfig = AppConfig()
    }
}
