package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.ApiError
import com.web3analytics.analytics.fp.io
import com.web3analytics.analytics.fp.left
import com.web3analytics.analytics.fp.requirePairAddress
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.fp.respondWindows
import com.web3analytics.analytics.fp.right
import com.web3analytics.analytics.store.AnalyticsStore
import io.ktor.server.application.call
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import io.ktor.server.routing.route

private const val NO_DATA_MESSAGE = "no data for pair"

fun Route.liquidityRoutes(store: AnalyticsStore) {
    route("/pairs/{pair}") {
        get("/liquidity") {
            call.respondWindows(NO_DATA_MESSAGE, store::getLiquidityWindows) { it }
        }

        get("/liquidity/latest") {
            val result =
                call
                    .requirePairAddress()
                    .flatMap { pair ->
                        io { store.getLatestLiquidityWindow(pair) }?.right()
                            ?: ApiError.NotFound(NO_DATA_MESSAGE).left()
                    }
            call.respondEither(result)
        }

        get("/liquidity/flows") {
            call.respondWindows(NO_DATA_MESSAGE, store::getLiquidityWindows) {
                mapOf(
                    "windowId" to it.windowId,
                    "windowStart" to it.windowStart,
                    "windowEnd" to it.windowEnd,
                    "mintCount" to it.mintCount,
                    "burnCount" to it.burnCount,
                    "totalLpTokensMinted" to it.totalLpTokensMinted,
                    "totalLpTokensBurned" to it.totalLpTokensBurned,
                    "netLpTokenChange" to it.netLpTokenChange,
                    "uniqueProviders" to it.uniqueProviders,
                    "churnedProviders" to it.churnedProviders,
                )
            }
        }
    }
}
