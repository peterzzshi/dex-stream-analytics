package com.web3analytics.analytics.route

import com.web3analytics.analytics.stream.AnalyticsEvent
import com.web3analytics.analytics.stream.AnalyticsEventBus
import io.ktor.server.routing.Route
import io.ktor.server.websocket.webSocket
import io.ktor.websocket.Frame
import io.ktor.websocket.readText
import kotlinx.coroutines.channels.ClosedReceiveChannelException
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import java.util.Collections
import org.slf4j.LoggerFactory

private val logger = LoggerFactory.getLogger("DashboardWebSocket")

private val ALL_CHANNELS = setOf("trading", "liquidity", "mev", "trend")

/**
 * WebSocket endpoint that streams real-time analytics to the dashboard.
 *
 * Demonstrates Kotlin coroutines + SharedFlow:
 * - Each WebSocket session is a coroutine that collects from the SharedFlow
 * - A child reader coroutine lets clients subscribe to specific channels by
 *   sending filter messages (JSON array, CSV, or bare channel name)
 * - Flow operators (filter, map) compose naturally with structured concurrency
 */
fun Route.dashboardWebSocket(eventBus: AnalyticsEventBus) {

    webSocket("/ws/analytics") {
        // Mutated by the reader coroutine, read by the collector — hence synchronized.
        val subscribedChannels = Collections.synchronizedSet(ALL_CHANNELS.toMutableSet())
        logger.info("WebSocket client connected")

        // Reader coroutine: client filter messages update the live subscription set.
        val reader = launch {
            try {
                for (frame in incoming) {
                    if (frame is Frame.Text) {
                        applyChannelFilter(frame.readText(), subscribedChannels)
                    }
                }
            } catch (_: ClosedReceiveChannelException) {
                // Client closed the socket; the collector's finally block cleans up.
            }
        }

        // Collector: SharedFlow → WebSocket frames.
        // This coroutine suspends on each emission, naturally backpressuring.
        try {
            eventBus.events
                .filter { it.channel in subscribedChannels }
                .collect { event ->
                    send(Frame.Text(buildWebSocketMessage(event)))
                }
        } finally {
            reader.cancel()
            logger.info("WebSocket client disconnected")
        }
    }
}

/**
 * Updates [target] in place from a client message. Any known channel names that
 * appear in the payload become the new subscription; an empty selection
 * resubscribes to all channels.
 */
private fun applyChannelFilter(message: String, target: MutableSet<String>) {
    val normalized = message.lowercase()
    val requested = ALL_CHANNELS.filter { it in normalized }.toSet()
    synchronized(target) {
        target.clear()
        target.addAll(requested.ifEmpty { ALL_CHANNELS })
    }
}

private fun buildWebSocketMessage(event: AnalyticsEvent): String {
    val (pair, payload) = when (event) {
        is AnalyticsEvent.TradingUpdate -> event.pairAddress to event.payload
        is AnalyticsEvent.LiquidityUpdate -> event.pairAddress to event.payload
        is AnalyticsEvent.MevAlertEvent -> event.pairAddress to event.payload
        is AnalyticsEvent.TrendUpdate -> event.pairAddress to event.payload
    }
    return """{"channel":"${event.channel}","pair":"$pair","data":$payload}"""
}
