package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.io
import com.web3analytics.analytics.fp.requirePairAddress
import com.web3analytics.analytics.fp.requirePairAndRange
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.health.PoolHealthScorer
import com.web3analytics.analytics.store.AnalyticsStore
import io.ktor.server.application.call
import io.ktor.server.response.respond
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope

fun Route.poolRoutes(store: AnalyticsStore) {
    val scorer = PoolHealthScorer(store)

    get("/pools/{pair}/health") {
        val result = call.requirePairAddress().map { io { scorer.evaluate(it) } }
        call.respondEither(result)
    }

    get("/pools/{pair}/alerts") {
        val result =
            call
                .requirePairAndRange()
                .map { (pair, range) -> io { store.getMevAlerts(pair, range) } }
        call.respondEither(result)
    }

    get("/pools/{pair}/trends") {
        val result =
            call
                .requirePairAndRange()
                .map { (pair, range) -> io { store.getMarketTrends(pair, range) } }
        call.respondEither(result)
    }

    get("/pools/leaderboard") {
        val pairs = store.allTrackedPairs()
        val healthScores =
            coroutineScope {
                pairs.map { pair -> async(Dispatchers.IO) { scorer.evaluate(pair) } }.awaitAll()
            }.sortedByDescending { it.overallScore }
        call.respond(healthScores)
    }
}
