package com.web3analytics.analytics.stream

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/**
 * Broadcasts analytics events to all connected WebSocket clients using Kotlin SharedFlow.
 *
 * SharedFlow is a hot flow — events are emitted regardless of subscriber count.
 * `replay = 1` ensures a newly connected client gets the most recent event immediately.
 * `extraBufferCapacity` absorbs bursts from Dapr without suspending the publisher.
 */
class AnalyticsEventBus {
    private val _events = MutableSharedFlow<AnalyticsEvent>(
        replay = 1,
        extraBufferCapacity = 64
    )
    val events: SharedFlow<AnalyticsEvent> = _events.asSharedFlow()

    suspend fun emit(event: AnalyticsEvent) {
        _events.emit(event)
    }
}

sealed class AnalyticsEvent(val channel: String) {
    data class TradingUpdate(val pairAddress: String, val payload: String) : AnalyticsEvent("trading")
    data class LiquidityUpdate(val pairAddress: String, val payload: String) : AnalyticsEvent("liquidity")
    data class MevAlertEvent(val pairAddress: String, val payload: String) : AnalyticsEvent("mev")
    data class TrendUpdate(val pairAddress: String, val payload: String) : AnalyticsEvent("trend")
}
