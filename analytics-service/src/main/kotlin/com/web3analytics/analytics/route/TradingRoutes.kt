package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.requireNonEmpty
import com.web3analytics.analytics.fp.requirePairAddress
import com.web3analytics.analytics.fp.requirePairAndRange
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.fp.ApiError
import com.web3analytics.analytics.fp.left
import com.web3analytics.analytics.fp.right
import com.web3analytics.analytics.model.AnalyticsSummary
import com.web3analytics.analytics.model.PairSummary
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.store.AnalyticsStore
import io.ktor.server.application.call
import io.ktor.server.response.respond
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import io.ktor.server.routing.route

fun Route.tradingRoutes(store: AnalyticsStore) {
    route("/pairs/{pair}") {
        get("/twap") {
            val result = call.requirePairAndRange()
                .flatMap { (pair, range) -> store.getTradingWindows(pair, range).requireNonEmpty("no data for pair") }
                .map { windows -> windows.map { mapOf("windowId" to it.windowId, "windowStart" to it.windowStart, "windowEnd" to it.windowEnd, "twap" to it.twap) } }
            call.respondEither(result)
        }

        get("/ohlc") {
            val result = call.requirePairAndRange()
                .flatMap { (pair, range) -> store.getTradingWindows(pair, range).requireNonEmpty("no data for pair") }
                .map { windows ->
                    windows.map {
                        mapOf(
                            "windowId" to it.windowId, "windowStart" to it.windowStart, "windowEnd" to it.windowEnd,
                            "open" to it.openPrice, "high" to it.highPrice, "low" to it.lowPrice, "close" to it.closePrice
                        )
                    }
                }
            call.respondEither(result)
        }

        get("/volume") {
            val result = call.requirePairAndRange()
                .flatMap { (pair, range) -> store.getTradingWindows(pair, range).requireNonEmpty("no data for pair") }
                .map { windows ->
                    windows.map {
                        mapOf(
                            "windowId" to it.windowId, "windowStart" to it.windowStart, "windowEnd" to it.windowEnd,
                            "volumeUSD" to it.volumeUSD, "totalVolume0" to it.totalVolume0,
                            "totalVolume1" to it.totalVolume1, "swapCount" to it.swapCount
                        )
                    }
                }
            call.respondEither(result)
        }

        get("/trading") {
            val result = call.requirePairAndRange()
                .flatMap { (pair, range) -> store.getTradingWindows(pair, range).requireNonEmpty("no data for pair") }
            call.respondEither(result)
        }

        get("/latest") {
            val result = call.requirePairAddress()
                .flatMap { pair ->
                    store.getLatestTradingWindow(pair)?.right()
                        ?: ApiError.NotFound("no data for pair").left()
                }
            call.respondEither(result)
        }
    }

    get("/analytics/summary") {
        call.respond(
            AnalyticsSummary(
                tradingPairsTracked = store.tradingPairCount(),
                liquidityPairsTracked = store.liquidityPairCount(),
                totalTradingWindows = store.totalTradingWindows(),
                totalLiquidityWindows = store.totalLiquidityWindows()
            )
        )
    }

    get("/analytics/pairs") {
        val pairs = store.trackedTradingPairs().map { pair ->
            val latest = store.getLatestTradingWindow(pair)
            PairSummary(
                pairAddress = pair,
                token0Symbol = latest?.token0Symbol,
                token1Symbol = latest?.token1Symbol,
                latestTwap = latest?.twap,
                latestVolumeUSD = latest?.volumeUSD,
                windowCount = store.getTradingWindows(pair, TimeRange(0L, System.currentTimeMillis())).size
            )
        }
        call.respond(pairs)
    }
}
