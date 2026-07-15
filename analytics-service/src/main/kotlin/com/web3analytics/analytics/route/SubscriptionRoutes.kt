package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.extractCloudEventData
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.fp.right
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
import com.fasterxml.jackson.databind.ObjectMapper
import org.slf4j.LoggerFactory

private val logger = LoggerFactory.getLogger("SubscriptionRoutes")

fun Route.subscriptionRoutes(store: AnalyticsStore, mapper: ObjectMapper, eventBus: AnalyticsEventBus) {

    post("/events/trading-analytics") {
        val body = call.receiveText()
        val result = mapper.extractCloudEventData<TradingAnalytics>(body)
            .map { analytics ->
                store.storeTradingAnalytics(analytics)
                eventBus.emit(AnalyticsEvent.TradingUpdate(analytics.pairAddress.value, mapper.readTree(body).get("data").toString()))
                logger.info("Stored trading window: pair={} twap={} volume={}",
                    analytics.pairAddress, analytics.twap, analytics.volumeUSD)
                mapOf("status" to "SUCCESS")
            }
        call.respondEither(result)
    }

    post("/events/liquidity-analytics") {
        val body = call.receiveText()
        val result = mapper.extractCloudEventData<LiquidityAnalytics>(body)
            .map { analytics ->
                store.storeLiquidityAnalytics(analytics)
                eventBus.emit(AnalyticsEvent.LiquidityUpdate(analytics.pairAddress.value, mapper.readTree(body).get("data").toString()))
                logger.info("Stored liquidity window: pair={} mints={} burns={}",
                    analytics.pairAddress, analytics.mintCount, analytics.burnCount)
                mapOf("status" to "SUCCESS")
            }
        call.respondEither(result)
    }

    post("/events/pattern-analytics") {
        val body = call.receiveText()
        val result = mapper.extractCloudEventData<MevAlert>(body)
            .map { alert ->
                store.storeMevAlert(alert)
                eventBus.emit(AnalyticsEvent.MevAlertEvent(alert.pairAddress.value, mapper.readTree(body).get("data").toString()))
                logger.info("Stored MEV alert: pair={} type={} severity={}",
                    alert.pairAddress, alert.alertType, alert.severity)
                mapOf("status" to "SUCCESS")
            }
        call.respondEither(result)
    }

    post("/events/market-trends") {
        val body = call.receiveText()
        val result = mapper.extractCloudEventData<MarketTrend>(body)
            .map { trend ->
                store.storeMarketTrend(trend)
                eventBus.emit(AnalyticsEvent.TrendUpdate(trend.pairAddress.value, mapper.readTree(body).get("data").toString()))
                logger.info("Stored market trend: pair={} trend={} priceChange={}%",
                    trend.pairAddress, trend.trend, trend.priceChangePercent)
                mapOf("status" to "SUCCESS")
            }
        call.respondEither(result)
    }
}
