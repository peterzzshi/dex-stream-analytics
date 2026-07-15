package com.web3analytics.analytics.fp

import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TimeRange
import io.ktor.server.application.ApplicationCall

/**
 * Validated parameter extraction that returns Either instead of nullable.
 *
 * Replaces the `?: return@get call.respond(400)` pattern with composable
 * Either values that chain via flatMap — error handling happens once at the end.
 */
fun ApplicationCall.requirePairAddress(): Either<ApiError, PairAddress> =
    parameters["pair"]
        ?.let { PairAddress(it).right() }
        ?: ApiError.BadRequest("pair required").left()

fun ApplicationCall.requireTimeRange(): Either<ApiError, TimeRange> {
    val now = System.currentTimeMillis()
    val from = request.queryParameters["from"]?.toLongOrNull() ?: (now - 3600_000)
    val to = request.queryParameters["to"]?.toLongOrNull() ?: now
    return if (from <= to) TimeRange(from, to).right()
    else ApiError.BadRequest("invalid time range: from ($from) must be <= to ($to)").left()
}

/**
 * Convenience for validated pair + time range together.
 */
fun ApplicationCall.requirePairAndRange(): Either<ApiError, Pair<PairAddress, TimeRange>> =
    requirePairAddress().flatMap { pair ->
        requireTimeRange().map { range -> pair to range }
    }

/**
 * Lifts a possibly-empty list into Either, short-circuiting to NotFound
 * when there's no data. Composes naturally in a flatMap chain.
 */
fun <T> List<T>.requireNonEmpty(message: String = "no data found"): Either<ApiError, List<T>> =
    if (isEmpty()) ApiError.NotFound(message).left() else right()
