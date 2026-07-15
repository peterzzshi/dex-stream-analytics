package com.web3analytics.analytics.fp

import io.ktor.http.HttpStatusCode

/**
 * Typed API errors that flow through Either pipelines.
 *
 * Each variant carries the HTTP status and message — the route handler
 * folds the Either once at the boundary to produce the response.
 */
sealed class ApiError(val status: HttpStatusCode, val message: String) {
    class BadRequest(message: String) : ApiError(HttpStatusCode.BadRequest, message)
    class NotFound(message: String) : ApiError(HttpStatusCode.NotFound, message)
    class Internal(message: String) : ApiError(HttpStatusCode.InternalServerError, message)
}
