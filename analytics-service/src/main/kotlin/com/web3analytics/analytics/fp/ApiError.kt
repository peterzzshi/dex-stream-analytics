package com.web3analytics.analytics.fp

import io.ktor.http.HttpStatusCode

sealed class ApiError(val status: HttpStatusCode, val message: String) {
    class BadRequest(message: String) : ApiError(HttpStatusCode.BadRequest, message)
    class NotFound(message: String) : ApiError(HttpStatusCode.NotFound, message)
}
