package com.web3analytics.analytics

import com.web3analytics.analytics.model.MarketTrend
import com.web3analytics.analytics.model.MevAlert
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.store.InMemoryAnalyticsStore
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.http.*
import io.ktor.server.testing.*
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.module.kotlin.readValue
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PoolRoutesTest {

    private val mapper = ObjectMapper().apply {
        findAndRegisterModules()
    }

    private fun testApp(block: suspend ApplicationTestBuilder.(InMemoryAnalyticsStore) -> Unit) = testApplication {
        val store = InMemoryAnalyticsStore()
        application { analyticsModule(storeOverride = store) }
        block(store)
    }

    @Test
    fun `GET pools health returns health score`() = testApp { store ->
        store.storeMevAlert(mevAlert("0xpair", severity = "HIGH"))
        store.storeMarketTrend(marketTrend("0xpair", trend = "BULLISH"))

        val response = client.get("/pools/0xpair/health")
        assertEquals(HttpStatusCode.OK, response.status)

        val body = mapper.readTree(response.bodyAsText())
        assertEquals("0xpair", body["pairAddress"].asText())
        assertTrue(body["overallScore"].asDouble() > 0)
        assertTrue(body["safetyScore"].asDouble() > 0)
        assertEquals(1, body["recentAlertCount"].asInt())
    }

    @Test
    fun `GET pools alerts returns alerts for pair`() = testApp { store ->
        store.storeMevAlert(mevAlert("0xpair", alertType = "SANDWICH_ATTACK"))
        store.storeMevAlert(mevAlert("0xpair", alertType = "JIT_LIQUIDITY"))
        store.storeMevAlert(mevAlert("0xother", alertType = "SANDWICH_ATTACK"))

        val now = System.currentTimeMillis()
        val response = client.get("/pools/0xpair/alerts?from=${now - 60000}&to=${now + 60000}")
        assertEquals(HttpStatusCode.OK, response.status)

        val alerts: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(2, alerts.size)
    }

    @Test
    fun `GET pools trends returns trends for pair`() = testApp { store ->
        store.storeMarketTrend(marketTrend("0xpair", trend = "BULLISH", windowStart = 1000))
        store.storeMarketTrend(marketTrend("0xpair", trend = "BEARISH", windowStart = 2000))

        val response = client.get("/pools/0xpair/trends?from=0&to=5000")
        assertEquals(HttpStatusCode.OK, response.status)

        val trends: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(2, trends.size)
    }

    @Test
    fun `GET pools leaderboard returns ranked pools`() = testApp { store ->
        // Pool with alerts (lower safety score)
        store.storeMevAlert(mevAlert("0xrisky"))
        store.storeMevAlert(mevAlert("0xrisky"))
        store.storeMevAlert(mevAlert("0xrisky"))

        // Pool without alerts (higher safety score)
        store.storeMarketTrend(marketTrend("0xsafe"))

        val response = client.get("/pools/leaderboard")
        assertEquals(HttpStatusCode.OK, response.status)

        val leaderboard: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(2, leaderboard.size)
        // Safe pool should rank higher
        assertEquals("0xsafe", leaderboard[0]["pairAddress"])
    }

    @Test
    fun `GET pools health for unknown pair returns default scores`() = testApp { _ ->
        val response = client.get("/pools/0xunknown/health")
        assertEquals(HttpStatusCode.OK, response.status)

        val body = mapper.readTree(response.bodyAsText())
        assertEquals("0xunknown", body["pairAddress"].asText())
        assertTrue(body["overallScore"].asDouble() >= 0)
    }

    private fun mevAlert(
        pair: String,
        alertType: String = "SANDWICH_ATTACK",
        severity: String = "HIGH"
    ) = MevAlert(
        alertId = "alert-${System.nanoTime()}",
        alertType = alertType,
        pairAddress = PairAddress(pair),
        severity = severity,
        detectedAt = System.currentTimeMillis()
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
