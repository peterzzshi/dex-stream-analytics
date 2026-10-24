package com.web3analytics.analytics.stream

import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class AnalyticsEventBusTest {
    @Test
    fun `emit never suspends when the buffer overflows without an active collector`() =
        runBlocking {
            val bus = AnalyticsEventBus()

            // Buffer capacity is 64; emitting far more than that with nobody
            // collecting would hang forever under BufferOverflow.SUSPEND.
            // DROP_OLDEST must let every emit return immediately instead.
            withTimeout(5_000) {
                repeat(1_000) { i ->
                    bus.emit(AnalyticsEvent.TradingUpdate("0xabc", "payload-$i"))
                }
            }
        }

    @Test
    fun `a late subscriber still receives the most recently emitted event via replay`() =
        runBlocking {
            val bus = AnalyticsEventBus()

            bus.emit(AnalyticsEvent.MevAlertEvent("0xabc", "first"))
            bus.emit(AnalyticsEvent.MevAlertEvent("0xabc", "latest"))

            val received = withTimeout(5_000) { bus.events.first() }

            assertEquals("latest", (received as AnalyticsEvent.MevAlertEvent).payload)
        }
}
