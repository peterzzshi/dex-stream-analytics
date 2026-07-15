package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.ApiError
import com.web3analytics.analytics.fp.left
import com.web3analytics.analytics.fp.requireNonEmpty
import com.web3analytics.analytics.fp.requirePairAddress
import com.web3analytics.analytics.fp.requirePairAndRange
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.fp.right
import com.web3analytics.analytics.store.AnalyticsStore
import io.ktor.server.application.call
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import io.ktor.server.routing.route

fun Route.liquidityRoutes(store: AnalyticsStore) {
    route("/pairs/{pair}") {
        get("/liquidity") {
            val result = call.requirePairAndRange()
                .flatMap { (pair, range) -> store.getLiquidityWindows(pair, range).requireNonEmpty("no data for pair") }
            call.respondEither(result)
        }

        get("/liquidity/latest") {
            val result = call.requirePairAddress()
                .flatMap { pair ->
                    store.getLatestLiquidityWindow(pair)?.right()
                        ?: ApiError.NotFound("no data for pair").left()
                }
            call.respondEither(result)
        }

        get("/liquidity/flows") {
            val result = call.requirePairAndRange()
                .flatMap { (pair, range) -> store.getLiquidityWindows(pair, range).requireNonEmpty("no data for pair") }
                .map { windows ->
                    windows.map {
                        mapOf(
                            "windowId" to it.windowId, "windowStart" to it.windowStart, "windowEnd" to it.windowEnd,
                            "mintCount" to it.mintCount, "burnCount" to it.burnCount,
                            "totalLpTokensMinted" to it.totalLpTokensMinted, "totalLpTokensBurned" to it.totalLpTokensBurned,
                            "netLpTokenChange" to it.netLpTokenChange, "uniqueProviders" to it.uniqueProviders,
                            "churnedProviders" to it.churnedProviders
                        )
                    }
                }
            call.respondEither(result)
        }
    }
}
