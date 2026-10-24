package com.web3analytics.analytics.fp

import com.web3analytics.analytics.model.PairAddress
import com.web3analytics.analytics.model.TimeRange
import io.ktor.server.application.ApplicationCall
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Validated parameter extraction that returns Either instead of nullable.
 *
 * Replaces the `?: return@get call.respond(400)` pattern with composable
 * Either values that chain via flatMap — error handling happens once at the end.
 *
 * The parsing logic itself is factored into plain functions (below) that take
 * only the raw string inputs, so it's unit-testable without a Ktor test server.
 */
fun ApplicationCall.requirePairAddress(): Either<ApiError, PairAddress> = parsePairAddress(parameters["pair"])

fun ApplicationCall.requireTimeRange(): Either<ApiError, TimeRange> =
    parseTimeRange(
        fromParam = request.queryParameters["from"],
        toParam = request.queryParameters["to"],
        now = System.currentTimeMillis(),
    )

fun parsePairAddress(raw: String?): Either<ApiError, PairAddress> =
    raw
        ?.let { PairAddress(it).right() }
        ?: ApiError.BadRequest("pair required").left()

/**
 * A present-but-unparsable `from`/`to` is a client error, not an invitation to
 * fall back to the default — only an absent parameter uses the default window.
 */
fun parseTimeRange(
    fromParam: String?,
    toParam: String?,
    now: Long,
): Either<ApiError, TimeRange> {
    val from =
        fromParam?.let {
            it.toLongOrNull() ?: return ApiError.BadRequest("invalid 'from': $it").left()
        } ?: (now - 3600_000)
    val to =
        toParam?.let {
            it.toLongOrNull() ?: return ApiError.BadRequest("invalid 'to': $it").left()
        } ?: now
    return if (from <= to) {
        TimeRange(from, to).right()
    } else {
        ApiError.BadRequest("invalid time range: from ($from) must be <= to ($to)").left()
    }
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

/**
 * Runs a blocking call (Jedis, JDBC, ...) off the request-handling coroutine
 * so one slow store call can't starve Ktor's call-processing threads.
 */
suspend fun <T> io(block: () -> T): T = withContext(Dispatchers.IO) { block() }

/**
 * Shared validate → fetch → require-non-empty → project → respond skeleton for
 * "pair + time range" list endpoints. [fetch] and [project] are passed in as
 * functions so each route supplies only what's different about it, instead of
 * duplicating the whole pipeline per endpoint.
 */
suspend fun <T, R> ApplicationCall.respondWindows(
    notFoundMessage: String,
    fetch: (PairAddress, TimeRange) -> List<T>,
    project: (T) -> R,
) {
    val result =
        requirePairAndRange()
            .flatMap { (pair, range) -> io { fetch(pair, range) }.requireNonEmpty(notFoundMessage) }
            .map { windows -> windows.map(project) }
    respondEither(result)
}
