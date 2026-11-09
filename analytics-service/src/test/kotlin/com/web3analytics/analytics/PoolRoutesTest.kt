package com.web3analytics.analytics

import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.module.kotlin.readValue
import com.web3analytics.analytics.model.MarketTrend
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TradingAnalytics
import com.web3analytics.analytics.store.InMemoryAnalyticsStore
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import io.ktor.server.testing.*
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PoolRoutesTest {
    private val mapper =
        ObjectMapper().apply {
            findAndRegisterModules()
        }

    private fun testApp(block: suspend ApplicationTestBuilder.(InMemoryAnalyticsStore) -> Unit) =
        testApplication {
            val store = InMemoryAnalyticsStore()
            application { analyticsModule(storeOverride = store) }
            block(store)
        }

    @Test
    fun `GET pools health returns health score`() =
        testApp { store ->
            store.storeMarketTrend(marketTrend("0xpair", trend = "BULLISH"))

            val response = client.get("/pools/0xpair/health")
            assertEquals(HttpStatusCode.OK, response.status)

            val body = mapper.readTree(response.bodyAsText())
            assertEquals("0xpair", body["pairAddress"].asText())
            assertTrue(body["overallScore"].asDouble() > 0)
            assertEquals("BULLISH", body["trend"].asText())
        }

    @Test
    fun `GET pools trends returns trends for pair`() =
        testApp { store ->
            store.storeMarketTrend(marketTrend("0xpair", trend = "BULLISH", windowStart = 1000))
            store.storeMarketTrend(marketTrend("0xpair", trend = "BEARISH", windowStart = 2000))

            val response = client.get("/pools/0xpair/trends?from=0&to=5000")
            assertEquals(HttpStatusCode.OK, response.status)

            val trends: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
            assertEquals(2, trends.size)
        }

    @Test
    fun `GET pools leaderboard returns ranked pools`() =
        testApp { store ->
            // Active pool: high trading volume and trader count (higher trading score)
            store.storeTradingAnalytics(tradingAnalytics("0xactive", volumeUSD = 500_000.0, uniqueTraders = 50))

            // Quiet pool: a tracked pair with no trading activity (lower trading score)
            store.storeMarketTrend(marketTrend("0xquiet"))

            val response = client.get("/pools/leaderboard")
            assertEquals(HttpStatusCode.OK, response.status)

            val leaderboard: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
            assertEquals(2, leaderboard.size)
            assertEquals("0xactive", leaderboard[0]["pairAddress"])
        }

    @Test
    fun `GET pools health for unknown pair returns default scores`() =
        testApp { _ ->
            val response = client.get("/pools/0xunknown/health")
            assertEquals(HttpStatusCode.OK, response.status)

            val body = mapper.readTree(response.bodyAsText())
            assertEquals("0xunknown", body["pairAddress"].asText())
            assertTrue(body["overallScore"].asDouble() >= 0)
        }

    private fun tradingAnalytics(
        pair: String,
        volumeUSD: Double = 0.0,
        uniqueTraders: Int = 0,
        windowStart: Long = System.currentTimeMillis()
    ) = TradingAnalytics(
        windowId = "$pair:trading:$windowStart",
        windowStart = windowStart,
        windowEnd = windowStart + 300000,
        pairAddress = PairAddress(pair),
        volumeUSD = volumeUSD,
        uniqueTraders = uniqueTraders,
        processedAt = System.currentTimeMillis()
    )

    private fun marketTrend(
        pair: String,
        trend: String = "NEUTRAL",
        windowStart: Long = System.currentTimeMillis()
    ) = MarketTrend(
        windowId = "$pair:trend:$windowStart",
        windowStart = windowStart,
        windowEnd = windowStart + 300000,
        pairAddress = PairAddress(pair),
        trend = trend,
        processedAt = System.currentTimeMillis()
    )
}
