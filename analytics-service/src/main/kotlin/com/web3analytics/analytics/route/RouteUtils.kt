package com.web3analytics.analytics.route

import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TimeRange
import io.ktor.server.application.ApplicationCall

fun ApplicationCall.timeRange(): TimeRange? {
    val now = System.currentTimeMillis()
    val from = request.queryParameters["from"]?.toLongOrNull() ?: (now - 3600_000) // default: last hour
    val to = request.queryParameters["to"]?.toLongOrNull() ?: now
    return if (from <= to) TimeRange(from, to) else null
}

fun ApplicationCall.pairAddress(): PairAddress? = parameters["pair"]?.let(::PairAddress)
