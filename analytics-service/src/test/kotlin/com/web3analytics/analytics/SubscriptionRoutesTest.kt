package com.web3analytics.analytics

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

class SubscriptionRoutesTest {

    private val mapper = ObjectMapper().apply {
        findAndRegisterModules()
    }

    private fun testApp(block: suspend ApplicationTestBuilder.(InMemoryAnalyticsStore) -> Unit) = testApplication {
        val store = InMemoryAnalyticsStore()
        application { analyticsModule(storeOverride = store) }
        block(store)
    }

    @Test
    fun `POST events trading-analytics stores data`() = testApp { store ->
        val envelope = """
        {
            "id": "evt-1",
            "source": "aggregator",
            "specversion": "1.0",
            "type": "trading-analytics",
            "data": {
                "windowId": "0xabc:1000",
                "windowStart": 1000,
                "windowEnd": 301000,
                "pairAddress": "0xabc",
                "token0Symbol": "WETH",
                "token1Symbol": "USDC",
                "twap": 1.85,
                "openPrice": 1.80,
                "closePrice": 1.90,
                "highPrice": 1.95,
                "lowPrice": 1.75,
                "volumeUSD": 50000.0,
                "swapCount": 25,
                "uniqueTraders": 8,
                "processedAt": 999999
            }
        }
        """.trimIndent()

        val response = client.post("/events/trading-analytics") {
            contentType(ContentType.Application.Json)
            setBody(envelope)
        }
        assertEquals(HttpStatusCode.OK, response.status)

        val latest = store.getLatestTradingWindow(PairAddress("0xabc"))
        assertEquals(1.85, latest?.twap)
        assertEquals(50000.0, latest?.volumeUSD)
        assertEquals(25, latest?.swapCount)
    }

    @Test
    fun `POST events liquidity-analytics stores data`() = testApp { store ->
        val envelope = """
        {
            "id": "evt-2",
            "source": "aggregator",
            "specversion": "1.0",
            "type": "liquidity-analytics",
            "data": {
                "windowId": "0xabc:1000",
                "windowStart": 1000,
                "windowEnd": 3601000,
                "pairAddress": "0xabc",
                "mintCount": 5,
                "burnCount": 2,
                "totalLpTokensMinted": "9876543210",
                "totalLpTokensBurned": "1234567890",
                "netLpTokenChange": "8641975320",
                "uniqueProviders": 4,
                "processedAt": 999999
            }
        }
        """.trimIndent()

        val response = client.post("/events/liquidity-analytics") {
            contentType(ContentType.Application.Json)
            setBody(envelope)
        }
        assertEquals(HttpStatusCode.OK, response.status)

        val latest = store.getLatestLiquidityWindow(PairAddress("0xabc"))
        assertEquals(5, latest?.mintCount)
        assertEquals(2, latest?.burnCount)
        assertEquals("9876543210", latest?.totalLpTokensMinted)
    }

    @Test
    fun `POST events trading-analytics with missing data returns 400`() = testApp { _ ->
        val envelope = """{"id": "evt-1", "source": "aggregator"}"""

        val response = client.post("/events/trading-analytics") {
            contentType(ContentType.Application.Json)
            setBody(envelope)
        }
        assertEquals(HttpStatusCode.BadRequest, response.status)
    }
}
