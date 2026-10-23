package com.web3analytics.analytics.route

import com.web3analytics.analytics.fp.requirePairAddress
import com.web3analytics.analytics.fp.requirePairAndRange
import com.web3analytics.analytics.fp.respondEither
import com.web3analytics.analytics.health.PoolHealthScorer
import com.web3analytics.analytics.store.AnalyticsStore
import io.ktor.server.application.call
import io.ktor.server.response.respond
import io.ktor.server.routing.Route
import io.ktor.server.routing.get

fun Route.poolRoutes(store: AnalyticsStore) {
    val scorer = PoolHealthScorer(store)

    get("/pools/{pair}/health") {
        val result = call.requirePairAddress().map { scorer.evaluate(it) }
        call.respondEither(result)
    }

    get("/pools/{pair}/alerts") {
        val result = call.requirePairAndRange()
            .map { (pair, range) -> store.getMevAlerts(pair, range) }
        call.respondEither(result)
    }

    get("/pools/{pair}/trends") {
        val result = call.requirePairAndRange()
            .map { (pair, range) -> store.getMarketTrends(pair, range) }
        call.respondEither(result)
    }

    get("/pools/leaderboard") {
        val pairs = store.allTrackedPairs()
        val healthScores = pairs.map { scorer.evaluate(it) }
            .sortedByDescending { it.overallScore }
        call.respond(healthScores)
    }
}
