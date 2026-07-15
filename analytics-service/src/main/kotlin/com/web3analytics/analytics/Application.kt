package com.web3analytics.analytics

import com.web3analytics.analytics.config.AppConfig
import com.web3analytics.analytics.config.analyticsObjectMapper
import com.web3analytics.analytics.config.jacksonConfig
import com.web3analytics.analytics.route.dashboardRoutes
import com.web3analytics.analytics.route.dashboardWebSocket
import com.web3analytics.analytics.route.liquidityRoutes
import com.web3analytics.analytics.route.poolRoutes
import com.web3analytics.analytics.route.subscriptionRoutes
import com.web3analytics.analytics.route.tradingRoutes
import com.web3analytics.analytics.store.AnalyticsStore
import com.web3analytics.analytics.store.RedisAnalyticsStore
import com.web3analytics.analytics.stream.AnalyticsEventBus
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpMethod
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
import io.ktor.server.response.respond
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.routing
import io.ktor.server.websocket.WebSockets
import org.slf4j.LoggerFactory

private val logger = LoggerFactory.getLogger("Application")

fun main() {
    val config = AppConfig.load()
    logger.info("Starting analytics-service on port {}", config.port)

    embeddedServer(Netty, port = config.port, host = "0.0.0.0") {
        analyticsModule(config)
    }.start(wait = true)
}

fun Application.analyticsModule(
    config: AppConfig = AppConfig.load(),
    storeOverride: AnalyticsStore? = null,
    eventBusOverride: AnalyticsEventBus? = null
) {
    val mapper = analyticsObjectMapper()

    val store: AnalyticsStore = storeOverride ?: RedisAnalyticsStore(config, mapper)
    val eventBus = eventBusOverride ?: AnalyticsEventBus()

    install(ContentNegotiation) {
        jackson(block = jacksonConfig)
    }

    install(WebSockets) {
        pingPeriodMillis = 30_000
        timeoutMillis = 15_000
        maxFrameSize = Long.MAX_VALUE
        masking = false
    }

    install(CORS) {
        allowMethod(HttpMethod.Get)
        allowMethod(HttpMethod.Post)
        allowHeader(HttpHeaders.ContentType)
        anyHost()
    }

    install(StatusPages) {
        exception<IllegalArgumentException> { call, cause ->
            call.respond(HttpStatusCode.BadRequest, mapOf("error" to (cause.message ?: "bad request")))
        }
        exception<Throwable> { call, cause ->
            logger.error("Unhandled exception", cause)
            call.respond(HttpStatusCode.InternalServerError, mapOf("error" to "internal server error"))
        }
    }

    routing {
        get("/health") {
            call.respondText("ok")
        }

        tradingRoutes(store)
        liquidityRoutes(store)
        poolRoutes(store)
        subscriptionRoutes(store, mapper, eventBus)
        dashboardRoutes()
        dashboardWebSocket(eventBus)
    }
}
