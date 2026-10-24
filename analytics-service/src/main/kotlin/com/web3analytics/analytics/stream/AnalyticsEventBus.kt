package com.web3analytics.analytics.stream

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/**
 * Broadcasts analytics events to all connected WebSocket clients using Kotlin SharedFlow.
 *
 * SharedFlow is a hot flow — events are emitted regardless of subscriber count.
 * `replay = 1` ensures a newly connected client gets the most recent event immediately.
 * `extraBufferCapacity` plus `onBufferOverflow = DROP_OLDEST` means a stalled
 * WebSocket subscriber drops its own back-catalog of events instead of making
 * `emit()` suspend — one slow dashboard client can never block ingestion.
 */
class AnalyticsEventBus {
    private val _events =
        MutableSharedFlow<AnalyticsEvent>(
            replay = 1,
            extraBufferCapacity = 64,
            onBufferOverflow = BufferOverflow.DROP_OLDEST,
        )
    val events: SharedFlow<AnalyticsEvent> = _events.asSharedFlow()

    suspend fun emit(event: AnalyticsEvent) {
        _events.emit(event)
    }
}

/**
 * Single source of truth for channel wire-names on the Kotlin side. The
 * dashboard WebSocket's subscription filter reads [ALL] directly, but
 * `dashboard.js` is a static asset and has to mirror these strings by hand
 * in its `switch` — it can't reference this enum. If the two drift,
 * `dashboard.js`'s `default` branch logs a console warning instead of
 * silently dropping the event, so divergence is loud, not silent.
 */
enum class AnalyticsChannel(
    val wireName: String,
) {
    TRADING("trading"),
    LIQUIDITY("liquidity"),
    MEV("mev"),
    TREND("trend");

    companion object {
        val ALL: Set<String> = entries.map { it.wireName }.toSet()
    }
}

sealed class AnalyticsEvent(
    val channel: String,
) {
    data class TradingUpdate(
        val pairAddress: String,
        val payload: String,
    ) : AnalyticsEvent(AnalyticsChannel.TRADING.wireName)

    data class LiquidityUpdate(
        val pairAddress: String,
        val payload: String,
    ) : AnalyticsEvent(AnalyticsChannel.LIQUIDITY.wireName)

    data class MevAlertEvent(
        val pairAddress: String,
        val payload: String,
    ) : AnalyticsEvent(AnalyticsChannel.MEV.wireName)

    data class TrendUpdate(
        val pairAddress: String,
        val payload: String,
    ) : AnalyticsEvent(AnalyticsChannel.TREND.wireName)
}
