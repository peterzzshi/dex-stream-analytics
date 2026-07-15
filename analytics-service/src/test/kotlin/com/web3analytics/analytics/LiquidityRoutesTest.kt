package com.web3analytics.analytics

import com.web3analytics.analytics.model.LiquidityAnalytics
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

class LiquidityRoutesTest {

    private val mapper = ObjectMapper().apply {
        findAndRegisterModules()
    }

    private fun testApp(block: suspend ApplicationTestBuilder.(InMemoryAnalyticsStore) -> Unit) = testApplication {
        val store = InMemoryAnalyticsStore()
        application { analyticsModule(storeOverride = store) }
        block(store)
    }

    @Test
    fun `GET pairs liquidity returns 404 for unknown pair`() = testApp { _ ->
        val response = client.get("/pairs/0xunknown/liquidity?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.NotFound, response.status)
    }

    @Test
    fun `GET pairs liquidity returns data for known pair`() = testApp { store ->
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 1000, mints = 3, burns = 1))

        val response = client.get("/pairs/0xabc/liquidity?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals(3, body[0]["mintCount"])
        assertEquals(1, body[0]["burnCount"])
    }

    @Test
    fun `GET pairs liquidity latest returns most recent`() = testApp { store ->
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 1000, mints = 1, burns = 0))
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 5000, mints = 4, burns = 2))

        val response = client.get("/pairs/0xabc/liquidity/latest")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: Map<String, Any> = mapper.readValue(response.bodyAsText())
        assertEquals(4, body["mintCount"])
        assertEquals(2, body["burnCount"])
    }

    @Test
    fun `GET pairs liquidity flows returns LP token flow summary`() = testApp { store ->
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 1000,
            mints = 2, burns = 1, lpMinted = "1000000", lpBurned = "500000", netLp = "500000"))

        val response = client.get("/pairs/0xabc/liquidity/flows?from=0&to=9999999999999")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals("1000000", body[0]["totalLpTokensMinted"])
        assertEquals("500000", body[0]["totalLpTokensBurned"])
        assertEquals("500000", body[0]["netLpTokenChange"])
        assertEquals(5, body[0]["uniqueProviders"])
    }

    @Test
    fun `liquidity time range filters correctly`() = testApp { store ->
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 1000, mints = 1, burns = 0))
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 10000, mints = 2, burns = 0))
        store.storeLiquidityAnalytics(liquidityWindow("0xabc", 20000, mints = 3, burns = 0))

        val response = client.get("/pairs/0xabc/liquidity?from=8000&to=15000")
        assertEquals(HttpStatusCode.OK, response.status)

        val body: List<Map<String, Any>> = mapper.readValue(response.bodyAsText())
        assertEquals(1, body.size)
        assertEquals(2, body[0]["mintCount"])
    }

    private fun liquidityWindow(
        pair: String,
        start: Long,
        mints: Int = 0,
        burns: Int = 0,
        lpMinted: String? = null,
        lpBurned: String? = null,
        netLp: String? = null
    ) = LiquidityAnalytics(
        windowId = "$pair:$start",
        windowStart = start,
        windowEnd = start + 3600000,
        pairAddress = PairAddress(pair),
        token0Symbol = "WETH",
        token1Symbol = "USDC",
        mintCount = mints,
        burnCount = burns,
        totalLpTokensMinted = lpMinted,
        totalLpTokensBurned = lpBurned,
        netLpTokenChange = netLp,
        uniqueProviders = 5,
        churnedProviders = 1,
        processedAt = System.currentTimeMillis()
    )
}
