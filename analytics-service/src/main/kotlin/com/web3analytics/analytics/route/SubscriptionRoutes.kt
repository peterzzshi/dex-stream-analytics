package com.web3analytics.analytics.route

import com.fasterxml.jackson.databind.ObjectMapper
import com.web3analytics.analytics.fp.extractCloudEventData
import com.web3analytics.analytics.fp.io
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.model.LiquidityAnalytics
import com.web3analytics.analytics.model.MarketTrend
import com.web3analytics.analytics.model.MevAlert
import com.web3analytics.analytics.model.TradingAnalytics
import com.web3analytics.analytics.store.AnalyticsStore
import com.web3analytics.analytics.stream.AnalyticsEvent
import com.web3analytics.analytics.stream.AnalyticsEventBus
import io.ktor.server.application.call
import io.ktor.server.request.receiveText
import io.ktor.server.routing.Route
import io.ktor.server.routing.post
import org.slf4j.LoggerFactory

private val logger = LoggerFactory.getLogger("SubscriptionRoutes")

fun Route.subscriptionRoutes(
    store: AnalyticsStore,
    mapper: ObjectMapper,
    eventBus: AnalyticsEventBus,
) {
    post("/events/trading-analytics") {
        val body = call.receiveText()
        val result =
            mapper
                .extractCloudEventData<TradingAnalytics>(body)
                .map { analytics ->
                    io { store.storeTradingAnalytics(analytics) }
                    val payload = mapper.writeValueAsString(analytics)
                    logger.info(
                        "Stored trading window: pair={} twap={} volume={}",
                        analytics.pairAddress,
                        analytics.twap,
                        analytics.volumeUSD,
                    )
                    mapOf("status" to "SUCCESS").also {
                        eventBus.emit(AnalyticsEvent.TradingUpdate(analytics.pairAddress.value, payload))
                    }
                }
        call.respondEither(result)
    }

    post("/events/liquidity-analytics") {
        val body = call.receiveText()
        val result =
            mapper
                .extractCloudEventData<LiquidityAnalytics>(body)
                .map { analytics ->
                    io { store.storeLiquidityAnalytics(analytics) }
                    val payload = mapper.writeValueAsString(analytics)
                    logger.info(
                        "Stored liquidity window: pair={} mints={} burns={}",
                        analytics.pairAddress,
                        analytics.mintCount,
                        analytics.burnCount,
                    )
                    mapOf("status" to "SUCCESS").also {
                        eventBus.emit(AnalyticsEvent.LiquidityUpdate(analytics.pairAddress.value, payload))
                    }
                }
        call.respondEither(result)
    }

    post("/events/pattern-analytics") {
        val body = call.receiveText()
        val result =
            mapper
                .extractCloudEventData<MevAlert>(body)
                .map { alert ->
                    io { store.storeMevAlert(alert) }
                    val payload = mapper.writeValueAsString(alert)
                    logger.info(
                        "Stored MEV alert: pair={} type={} severity={}",
                        alert.pairAddress,
                        alert.alertType,
                        alert.severity,
                    )
                    mapOf("status" to "SUCCESS").also {
                        eventBus.emit(AnalyticsEvent.MevAlertEvent(alert.pairAddress.value, payload))
                    }
                }
        call.respondEither(result)
    }

    post("/events/market-trends") {
        val body = call.receiveText()
        val result =
            mapper
                .extractCloudEventData<MarketTrend>(body)
                .map { trend ->
                    io { store.storeMarketTrend(trend) }
                    val payload = mapper.writeValueAsString(trend)
                    logger.info(
                        "Stored market trend: pair={} trend={} priceChange={}%",
                        trend.pairAddress,
                        trend.trend,
                        trend.priceChangePercent,
                    )
                    mapOf("status" to "SUCCESS").also {
                        eventBus.emit(AnalyticsEvent.TrendUpdate(trend.pairAddress.value, payload))
                    }
                }
        call.respondEither(result)
    }
}
