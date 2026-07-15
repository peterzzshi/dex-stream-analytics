package com.web3analytics.analytics

import com.web3analytics.analytics.model.LiquidityAnalytics
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TradingAnalytics
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

class TradingRoutesTest {

    private val mapper = ObjectMapper().apply {
        findAndRegisterModules()
    }

    private fun testApp(block: suspend ApplicationTestBuilder.(InMemoryAnalyticsStore) -> Unit) = testApplication {
        val store = InMemoryAnalyticsStore()
        application { analyticsModule(storeOverride = store) }
        block(store)
    }

    @Test
    fun `GET health returns ok`() = testApp { _ ->
        val response = client.get("/health")
        assertEquals(HttpStatusCode.OK, response.status)
        assertEquals("ok", response.bodyAsText())
    }

    @Test
    fun `GET pairs twap returns 404 for unknown pair`() = testApp { _ ->
        val response = client.get("/pairs/0xunknown/twap?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.NotFound, response.status)
    }

    @Test
    fun `GET pairs twap returns data for known pair`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, 1.5))
        store.storeTradingAnalytics(tradingWindow("0xabc", 2000, 1.8))

        val response = client.get("/pairs/0xabc/twap?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(2, body.size)
        assertEquals(1.5, body[0]["twap"])
        assertEquals(1.8, body[1]["twap"])
    }

    @Test
    fun `GET pairs ohlc returns OHLC data`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, twap = 1.5, open = 1.4, high = 1.9, low = 1.3, close = 1.6))

        val response = client.get("/pairs/0xabc/ohlc?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals(1.4, body[0]["open"])
        assertEquals(1.9, body[0]["high"])
        assertEquals(1.3, body[0]["low"])
        assertEquals(1.6, body[0]["close"])
    }

    @Test
    fun `GET pairs volume returns volume data`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, volumeUSD = 5000.0, swapCount = 12))

        val response = client.get("/pairs/0xabc/volume?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals(5000.0, body[0]["volumeUSD"])
        assertEquals(12, body[0]["swapCount"])
    }

    @Test
    fun `GET pairs latest returns most recent window`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, 1.0))
        store.storeTradingAnalytics(tradingWindow("0xabc", 2000, 2.0))
        store.storeTradingAnalytics(tradingWindow("0xabc", 3000, 3.0))

        val response = client.get("/pairs/0xabc/latest")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: Map<String, Any> = mapper.readValue(response.bodyAsText())
        assertEquals(3.0, body["twap"])
    }

    @Test
    fun `GET pairs trading returns full analytics`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, 1.5))

        val response = client.get("/pairs/0xabc/trading?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertTrue(body[0].containsKey("pairAddress"))
        assertTrue(body[0].containsKey("twap"))
        assertTrue(body[0].containsKey("uniqueTraders"))
    }

    @Test
    fun `GET analytics summary returns counts`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, 1.0))
        store.storeTradingAnalytics(tradingWindow("0xdef", 1000, 2.0))
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 1000))

        val response = client.get("/analytics/summary")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: Map<String, Any> = mapper.readValue(response.bodyAsText())
        assertEquals(2, body["tradingPairsTracked"])
        assertEquals(1, body["liquidityPairsTracked"])
        assertEquals(2, body["totalTradingWindows"])
        assertEquals(1, body["totalLiquidityWindows"])
    }

    @Test
    fun `GET analytics pairs returns pair list`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, 1.5))

        val response = client.get("/analytics/pairs")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals("0xabc", body[0]["pairAddress"])
    }

    @Test
    fun `time range filters correctly`() = testApp { store ->
        store.storeTradingAnalytics(tradingWindow("0xabc", 1000, 1.0))
        store.storeTradingAnalytics(tradingWindow("0xabc", 5000, 2.0))
        store.storeTradingAnalytics(tradingWindow("0xabc", 9000, 3.0))

        val response = client.get("/pairs/0xabc/twap?from=4000&to=6000")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals(2.0, body[0]["twap"])
    }

    private fun tradingWindow(
        pair: String,
        start: Long,
        twap: Double = 0.0,
        open: Double = 0.0,
        high: Double = 0.0,
        low: Double = 0.0,
        close: Double = 0.0,
        volumeUSD: Double? = null,
        swapCount: Int = 0
    ) = TradingAnalytics(
        windowId = "$pair:$start",
        windowStart = start,
        windowEnd = start + 300000,
        pairAddress = PairAddress(pair),
        token0Symbol = "WETH",
        token1Symbol = "USDC",
        twap = twap,
        openPrice = open,
        closePrice = close,
        highPrice = high,
        lowPrice = low,
        volumeUSD = volumeUSD,
        swapCount = swapCount,
        uniqueTraders = 3,
        processedAt = System.currentTimeMillis()
    )

    private fun liquidityWindow(pair: String, start: Long) = LiquidityAnalytics(
        windowId = "$pair:$start",
        windowStart = start,
        windowEnd = start + 3600000,
        pairAddress = PairAddress(pair),
        mintCount = 2,
        burnCount = 1,
        processedAt = System.currentTimeMillis()
    )
}
