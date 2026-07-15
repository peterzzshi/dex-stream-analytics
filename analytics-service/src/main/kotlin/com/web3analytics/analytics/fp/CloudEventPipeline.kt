package com.web3analytics.analytics.fp

import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.ApplicationCall
import io.ktor.server.response.respond

/**
 * Generic CloudEvent data extraction.
 *
 * Replaces 4 copy-pasted blocks in SubscriptionRoutes with a single
 * reified inline function that returns Either<ApiError, T>.
 */
inline fun <reified T> ObjectMapper.extractCloudEventData(body: String): Either<ApiError, T> {
    val tree: JsonNode = catching { readTree(body) }
        .fold(
            onLeft = { return ApiError.BadRequest("invalid JSON: ${it.message}").left() },
            onRight = { it }
        )

    val dataNode = tree.get("data")
        ?: return ApiError.BadRequest("missing data field").left()

    return catching { treeToValue(dataNode, T::class.java) }
        .fold(
            onLeft = { ApiError.BadRequest("invalid data payload: ${it.message}").left() },
            onRight = { it.right() }
        )
}

/**
 * Terminal fold for Either<ApiError, T> in route handlers.
 *
 * Converts the Either into an HTTP response — errors become JSON error objects,
 * success values become the response body. Errors are handled once at the boundary.
 */
suspend inline fun <reified T : Any> ApplicationCall.respondEither(
    result: Either<ApiError, T>,
    successStatus: HttpStatusCode = HttpStatusCode.OK
) {
    result.fold(
        onLeft = { error -> respond(error.status, mapOf("error" to error.message)) },
        onRight = { value -> respond(successStatus, value) }
    )
}
