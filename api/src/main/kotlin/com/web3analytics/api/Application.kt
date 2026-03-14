package com.web3analytics.api

import io.ktor.http.HttpStatusCode
import io.ktor.serialization.jackson.jackson
import io.ktor.server.application.Application
import io.ktor.server.application.call
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.routing
import java.util.concurrent.ConcurrentHashMap

data class AnalyticsEvent(
    val pairAddress: String?,
    val twap: Double?,
    val volumeUSD: Double?
)

data class CloudEventEnvelope(
    val id: String? = null,
    val source: String? = null,
    val specversion: String? = null,
    val type: String? = null,
    val datacontenttype: String? = null,
    val topic: String? = null,
    val pubsubname: String? = null,
    val data: AnalyticsEvent? = null
)

private class MemoryStore {
    private val twapByPair = ConcurrentHashMap<String, Double>()
    private val volumeByPair = ConcurrentHashMap<String, Double>()

    fun upsert(pair: String, twap: Double, volume: Double) {
        twapByPair[pair] = twap
        volumeByPair[pair] = volume
    }

    fun twap(pair: String): Double? = twapByPair[pair]

    fun volume(pair: String): Double? = volumeByPair[pair]

    fun summary(): Map<String, Any> = mapOf(
        "pairsTracked" to twapByPair.size,
        "twapByPair" to twapByPair,
        "volumeByPair" to volumeByPair
    )
}

fun main() {
    val port = System.getenv("APP_PORT")?.toIntOrNull() ?: 8080
    embeddedServer(Netty, port = port, host = "0.0.0.0") {
        dexApiModule()
    }.start(wait = true)
}

fun Application.dexApiModule() {
    install(ContentNegotiation) {
        jackson()
    }

    val store = MemoryStore()

    routing {
        get("/health") {
            call.respondText("ok")
        }

        // Optional Dapr discovery endpoint. Static subscription manifests still work too.
        get("/dapr/subscribe") {
            call.respond(emptyList<Map<String, Any>>())
        }

        post("/events/analytics") {
            val envelope = call.receive<CloudEventEnvelope>()
            val payload = envelope.data

            if (payload?.pairAddress.isNullOrBlank() || payload?.twap == null || payload.volumeUSD == null) {
                call.respond(
                    HttpStatusCode.BadRequest,
                    mapOf("error" to "invalid analytics payload")
                )
                return@post
            }

            store.upsert(payload.pairAddress, payload.twap, payload.volumeUSD)
            call.respond(HttpStatusCode.OK)
        }

        get("/pairs/{pair}/twap") {
            val pair = call.parameters["pair"]
            if (pair.isNullOrBlank()) {
                call.respond(HttpStatusCode.BadRequest, mapOf("error" to "missing pair"))
                return@get
            }

            val twap = store.twap(pair)
            if (twap == null) {
                call.respond(HttpStatusCode.NotFound, mapOf("error" to "pair not found"))
                return@get
            }

            call.respond(mapOf("pair" to pair, "twap" to twap))
        }

        get("/pairs/{pair}/volume") {
            val pair = call.parameters["pair"]
            if (pair.isNullOrBlank()) {
                call.respond(HttpStatusCode.BadRequest, mapOf("error" to "missing pair"))
                return@get
            }

            val volume = store.volume(pair)
            if (volume == null) {
                call.respond(HttpStatusCode.NotFound, mapOf("error" to "pair not found"))
                return@get
            }

            call.respond(mapOf("pair" to pair, "volume" to volume))
        }

        get("/analytics/summary") {
            call.respond(store.summary())
        }
    }
}
