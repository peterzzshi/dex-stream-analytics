package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.ApiError
import com.web3analytics.analytics.fp.io
import com.web3analytics.analytics.fp.left
import com.web3analytics.analytics.fp.requirePairAddress
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.fp.respondWindows
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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope

private const val NO_DATA_MESSAGE = "no data for pair"

fun Route.tradingRoutes(store: AnalyticsStore) {
    route("/pairs/{pair}") {
        get("/twap") {
            call.respondWindows(NO_DATA_MESSAGE, store::getTradingWindows) {
                mapOf(
                    "windowId" to it.windowId,
                    "windowStart" to it.windowStart,
                    "windowEnd" to it.windowEnd,
                    "twap" to it.twap,
                )
            }
        }

        get("/ohlc") {
            call.respondWindows(NO_DATA_MESSAGE, store::getTradingWindows) {
                mapOf(
                    "windowId" to it.windowId,
                    "windowStart" to it.windowStart,
                    "windowEnd" to it.windowEnd,
                    "open" to it.openPrice,
                    "high" to it.highPrice,
                    "low" to it.lowPrice,
                    "close" to it.closePrice,
                )
            }
        }

        get("/volume") {
            call.respondWindows(NO_DATA_MESSAGE, store::getTradingWindows) {
                mapOf(
                    "windowId" to it.windowId,
                    "windowStart" to it.windowStart,
                    "windowEnd" to it.windowEnd,
                    "volumeUSD" to it.volumeUSD,
                    "totalVolume0" to it.totalVolume0,
                    "totalVolume1" to it.totalVolume1,
                    "swapCount" to it.swapCount,
                )
            }
        }

        get("/trading") {
            call.respondWindows(NO_DATA_MESSAGE, store::getTradingWindows) { it }
        }

        get("/latest") {
            val result =
                call
                    .requirePairAddress()
                    .flatMap { pair ->
                        io { store.getLatestTradingWindow(pair) }?.right()
                            ?: ApiError.NotFound(NO_DATA_MESSAGE).left()
                    }
            call.respondEither(result)
        }
    }

    get("/analytics/summary") {
        val summary =
            io {
                AnalyticsSummary(
                    tradingPairsTracked = store.tradingPairCount(),
                    liquidityPairsTracked = store.liquidityPairCount(),
                    totalTradingWindows = store.totalTradingWindows(),
                    totalLiquidityWindows = store.totalLiquidityWindows(),
                )
            }
        call.respond(summary)
    }

    get("/analytics/pairs") {
        val pairs =
            coroutineScope {
                store
                    .trackedTradingPairs()
                    .map { pair ->
                        async(Dispatchers.IO) {
                            val latest = store.getLatestTradingWindow(pair)
                            PairSummary(
                                pairAddress = pair,
                                token0Symbol = latest?.token0Symbol,
                                token1Symbol = latest?.token1Symbol,
                                latestTwap = latest?.twap,
                                latestVolumeUSD = latest?.volumeUSD,
                                windowCount = store.tradingWindowCount(pair, TimeRange(0L, System.currentTimeMillis())),
                            )
                        }
                    }.awaitAll()
            }
        call.respond(pairs)
    }
}
