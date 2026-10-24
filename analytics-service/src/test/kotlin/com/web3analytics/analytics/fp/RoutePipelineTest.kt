package com.web3analytics.analytics.fp

import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TimeRange
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RoutePipelineTest {
    @Test
    fun `parsePairAddress accepts a present value`() {
        val result = parsePairAddress("0xabc")

        assertEquals(Either.Right(PairAddress("0xabc")), result)
    }

    @Test
    fun `parsePairAddress rejects a missing value`() {
        val result = parsePairAddress(null)

        assertTrue(result is Either.Left)
        assertEquals(400, (result as Either.Left).value.status.value)
    }

    @Test
    fun `parseTimeRange defaults to the last hour when both params are absent`() {
        val now = 10_000_000L
        val result = parseTimeRange(fromParam = null, toParam = null, now = now)

        assertEquals(Either.Right(TimeRange(now - 3_600_000, now)), result)
    }

    @Test
    fun `parseTimeRange uses provided from and to when both are valid`() {
        val result = parseTimeRange(fromParam = "100", toParam = "200", now = 999L)

        assertEquals(Either.Right(TimeRange(100, 200)), result)
    }

    @Test
    fun `parseTimeRange rejects a present but unparsable from instead of silently defaulting`() {
        val result = parseTimeRange(fromParam = "banana", toParam = "5000", now = 999L)

        assertTrue(result is Either.Left)
        assertEquals(400, (result as Either.Left).value.status.value)
    }

    @Test
    fun `parseTimeRange rejects a present but unparsable to instead of silently defaulting`() {
        val result = parseTimeRange(fromParam = "100", toParam = "banana", now = 999L)

        assertTrue(result is Either.Left)
        assertEquals(400, (result as Either.Left).value.status.value)
    }

    @Test
    fun `parseTimeRange rejects from greater than to`() {
        val result = parseTimeRange(fromParam = "200", toParam = "100", now = 999L)

        assertTrue(result is Either.Left)
        assertEquals(400, (result as Either.Left).value.status.value)
    }

    @Test
    fun `requireNonEmpty passes a non-empty list through unchanged`() {
        val result = listOf(1, 2, 3).requireNonEmpty()

        assertEquals(Either.Right(listOf(1, 2, 3)), result)
    }

    @Test
    fun `requireNonEmpty turns an empty list into a NotFound with the given message`() {
        val result = emptyList<Int>().requireNonEmpty("nothing here")

        assertTrue(result is Either.Left)
        assertEquals(404, (result as Either.Left).value.status.value)
        assertEquals("nothing here", result.value.message)
    }
}
