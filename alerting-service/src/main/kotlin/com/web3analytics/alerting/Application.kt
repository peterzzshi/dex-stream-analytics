package com.web3analytics.alerting

import com.web3analytics.alerting.config.AppConfig
import com.web3analytics.alerting.config.Json
import com.web3analytics.alerting.consumer.AlertConsumer
import com.web3analytics.alerting.store.RedisAlertStore
import com.web3analytics.alerting.websocket.AlertBroadcaster
import com.web3analytics.alerting.websocket.AlertFilters
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.jackson.jackson
import io.ktor.server.application.Application
import io.ktor.server.application.call
import io.ktor.server.application.install
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.plugins.cors.routing.CORS
import io.ktor.server.plugins.statuspages.StatusPages
import io.ktor.server.request.receiveText
import io.ktor.server.response.respond
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.routing
import io.ktor.server.websocket.WebSockets
import io.ktor.server.websocket.pingPeriod
import io.ktor.server.websocket.timeout
import io.ktor.server.websocket.webSocket
import org.slf4j.LoggerFactory
import java.time.Duration
import java.util.UUID

fun main() {
    embeddedServer(Netty, port = 8080, host = "0.0.0.0", module = Application::module)
        .start(wait = true)
}

fun Application.module() {
    val logger = LoggerFactory.getLogger("Application")
    val config = AppConfig()

    val jedisPool = config.createJedisPool()
    config.validateRedisConnection(jedisPool)
    logger.info("Redis connection validated: ${config.redisHost}:${config.redisPort}")

    val mapper = Json.mapper
    val store = RedisAlertStore(jedisPool, mapper, config.alertRetentionLimit)
    val broadcaster = AlertBroadcaster(mapper)
    val consumer = AlertConsumer(store, broadcaster, mapper, config.schemasPath)

    install(ContentNegotiation) {
        jackson {
            setConfig(mapper.serializationConfig)
            setConfig(mapper.deserializationConfig)
        }
    }

    install(WebSockets) {
        pingPeriod = kotlin.time.Duration.parse("${config.wsKeepaliveSeconds}s")
        timeout = kotlin.time.Duration.parse("15s")
        maxFrameSize = Long.MAX_VALUE
        masking = false
    }

    install(StatusPages) {
        exception<Throwable> { call, cause ->
            logger.error("Unhandled exception", cause)
            call.respond(
                HttpStatusCode.InternalServerError,
                mapOf("error" to (cause.message ?: "Internal server error"))
            )
        }
    }

    install(CORS) {
        anyHost()
        allowHeader("Content-Type")
    }

    routing {
        post("/dapr/subscribe") {
            val pubsubName = "kafka-pubsub"
            val subscriptions =
                listOf(
                    mapOf(
                        "pubsubname" to pubsubName,
                        "topic" to "bridge-alerts-transfer-volume",
                        "route" to "/events/transfer-volume"
                    ),
                    mapOf(
                        "pubsubname" to pubsubName,
                        "topic" to "bridge-alerts-supply-tracking",
                        "route" to "/events/supply-tracking"
                    ),
                    mapOf(
                        "pubsubname" to pubsubName,
                        "topic" to "bridge-alerts-cross-chain-flow",
                        "route" to "/events/cross-chain-flow"
                    ),
                    mapOf(
                        "pubsubname" to pubsubName,
                        "topic" to "bridge-alerts-exploit-detection",
                        "route" to "/events/exploit-detection"
                    ),
                    mapOf(
                        "pubsubname" to pubsubName,
                        "topic" to "bridge-alerts-reentrancy",
                        "route" to "/events/reentrancy"
                    )
                )
            call.respond(subscriptions)
        }

        post("/events/transfer-volume") {
            val body = call.receiveText()
            consumer.handleTransferVolume(body)
            call.respond(HttpStatusCode.OK)
        }

        post("/events/supply-tracking") {
            val body = call.receiveText()
            consumer.handleSupplyTracking(body)
            call.respond(HttpStatusCode.OK)
        }

        post("/events/cross-chain-flow") {
            val body = call.receiveText()
            consumer.handleCrossChainFlow(body)
            call.respond(HttpStatusCode.OK)
        }

        post("/events/exploit-detection") {
            val body = call.receiveText()
            consumer.handleExploitDetection(body)
            call.respond(HttpStatusCode.OK)
        }

        post("/events/reentrancy") {
            val body = call.receiveText()
            consumer.handleReentrancy(body)
            call.respond(HttpStatusCode.OK)
        }

        webSocket("/ws/alerts") {
            val sessionId = UUID.randomUUID().toString()
            val filters =
                AlertFilters.fromQueryParams(
                    call.request.queryParameters.entries().associate {
                        it.key to it.value
                    }
                )

            broadcaster.addConnection(sessionId, this, filters)

            try {
                for (frame in incoming) {
                    // Client messages ignored (one-way broadcast)
                }
            } finally {
                broadcaster.removeConnection(sessionId)
            }
        }

        get("/health") {
            try {
                jedisPool.resource.use { it.ping() }
                call.respond(
                    mapOf(
                        "status" to "ok",
                        "redis" to "connected",
                        "websocket_connections" to broadcaster.connectionCount()
                    )
                )
            } catch (e: Exception) {
                call.respond(
                    HttpStatusCode.ServiceUnavailable,
                    mapOf(
                        "status" to "error",
                        "redis" to "disconnected",
                        "error" to e.message
                    )
                )
            }
        }

        get("/alerts/{bridgeAddress}/{chain}") {
            val bridgeAddress = call.parameters["bridgeAddress"]!!
            val chain = call.parameters["chain"]!!
            val limit = call.request.queryParameters["limit"]?.toIntOrNull() ?: 100

            val alerts = store.getRecentAlerts(bridgeAddress, chain, limit)
            call.respond(alerts)
        }
    }
}
