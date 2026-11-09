package com.web3analytics.analytics

import com.web3analytics.analytics.health.PoolHealthScorer
import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.store.InMemoryAnalyticsStore
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PoolHealthScorerTest {
    @Test
    fun `trading score increases with volume and traders`() {
        val low = PoolHealthScorer.scoreTradingActivity(0, 0.0, 0)
        val mid = PoolHealthScorer.scoreTradingActivity(12, 50000.0, 5)
        val high = PoolHealthScorer.scoreTradingActivity(24, 100000.0, 10)

        assertTrue(low < mid)
        assertTrue(mid < high)
        assertEquals(1.0, high)
    }

    @Test
    fun `trading score caps at 1`() {
        val capped = PoolHealthScorer.scoreTradingActivity(100, 500000.0, 50)
        assertEquals(1.0, capped)
    }

    @Test
    fun `liquidity score defaults to half when no data`() {
        val score = PoolHealthScorer.scoreLiquidity(0)
        assertEquals(0.5, score)
    }

    @Test
    fun `evaluate returns valid pool health`() {
        val store = InMemoryAnalyticsStore()
        val scorer = PoolHealthScorer(store)

        val health = scorer.evaluate(PairAddress("0xtest"))

        assertEquals("0xtest", health.pairAddress.value)
        assertTrue(health.overallScore in 0.0..1.0)
        assertEquals("UNKNOWN", health.trend)
    }
}
