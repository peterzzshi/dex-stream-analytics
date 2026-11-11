package com.web3analytics.analytics.route

import com.web3analytics.analytics.model.CrossChainFlowResult
import com.web3analytics.analytics.model.ExploitDetectionResult
import com.web3analytics.analytics.model.ReentrancyResult
import com.web3analytics.analytics.model.SupplyTrackingResult
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.model.TransferVolumeResult
import com.web3analytics.analytics.store.BridgeStore
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.call
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.route
import org.slf4j.LoggerFactory

private val logger = LoggerFactory.getLogger("BridgeRoutes")

fun Route.bridgeRoutes(store: BridgeStore) {
    route("/api/bridges") {
        post("/transfer-volume") {
            val result = call.receive<TransferVolumeResult>()
            logger.info(
                "Received transfer volume: chain={}, bridge={}, window=[{}, {}], locks={}, unlocks={}",
                result.chain,
                result.bridgeAddress,
                result.windowStart,
                result.windowEnd,
                result.lockCount,
                result.unlockCount,
            )
            store.storeTransferVolume(result)
            call.respond(HttpStatusCode.OK, mapOf("status" to "stored"))
        }

        post("/supply") {
            val result = call.receive<SupplyTrackingResult>()
            logger.info(
                "Received supply tracking: chain={}, bridge={}, window=[{}, {}], netMinted={}, totalSupply={}",
                result.chain,
                result.bridgeAddress,
                result.windowStart,
                result.windowEnd,
                result.netMinted,
                result.totalWrappedSupply,
            )
            store.storeSupplyTracking(result)
            call.respond(HttpStatusCode.OK, mapOf("status" to "stored"))
        }

        post("/flow") {
            val result = call.receive<CrossChainFlowResult>()
            logger.info(
                "Received cross-chain flow: bridge={}, window=[{}, {}], netFlow={}, imbalance={}",
                result.bridgeAddress,
                result.windowStart,
                result.windowEnd,
                result.netFlow,
                result.imbalanceRatio,
            )
            store.storeCrossChainFlow(result)
            call.respond(HttpStatusCode.OK, mapOf("status" to "stored"))
        }

        post("/alert") {
            val result = call.receive<ExploitDetectionResult>()
            logger.warn(
                "Received exploit alert: bridge={}, chain={}, session=[{}, {}], patterns={}, eventCount={}",
                result.bridgeAddress,
                result.chain,
                result.sessionStart,
                result.sessionEnd,
                result.patterns,
                result.eventCount,
            )
            store.storeExploitDetection(result)
            call.respond(HttpStatusCode.OK, mapOf("status" to "stored"))
        }

        post("/reentrancy") {
            val result = call.receive<ReentrancyResult>()
            logger.warn(
                "Received reentrancy detection: tx={}, chain={}, bridge={}, locks={}, unlocks={}",
                result.transactionHash,
                result.chain,
                result.bridgeAddress,
                result.lockCount,
                result.unlockCount,
            )
            store.storeReentrancy(result)
            call.respond(HttpStatusCode.OK, mapOf("status" to "stored"))
        }

        get("/transfer-volume/{chain}/{bridgeAddress}") {
            val chain = call.parameters["chain"] ?: throw IllegalArgumentException("chain required")
            val bridgeAddress =
                call.parameters["bridgeAddress"] ?: throw IllegalArgumentException("bridgeAddress required")
            val from = call.parameters["from"]?.toLongOrNull() ?: (System.currentTimeMillis() - 3600_000)
            val to = call.parameters["to"]?.toLongOrNull() ?: System.currentTimeMillis()
            val results = store.getTransferVolume(chain, bridgeAddress, TimeRange(from, to))
            call.respond(results)
        }

        get("/supply/{chain}/{bridgeAddress}") {
            val chain = call.parameters["chain"] ?: throw IllegalArgumentException("chain required")
            val bridgeAddress =
                call.parameters["bridgeAddress"] ?: throw IllegalArgumentException("bridgeAddress required")
            val from = call.parameters["from"]?.toLongOrNull() ?: (System.currentTimeMillis() - 3600_000)
            val to = call.parameters["to"]?.toLongOrNull() ?: System.currentTimeMillis()
            val results = store.getSupplyTracking(chain, bridgeAddress, TimeRange(from, to))
            call.respond(results)
        }

        get("/flow/{bridgeAddress}") {
            val bridgeAddress =
                call.parameters["bridgeAddress"] ?: throw IllegalArgumentException("bridgeAddress required")
            val from = call.parameters["from"]?.toLongOrNull() ?: (System.currentTimeMillis() - 3600_000)
            val to = call.parameters["to"]?.toLongOrNull() ?: System.currentTimeMillis()
            val results = store.getCrossChainFlow(bridgeAddress, TimeRange(from, to))
            call.respond(results)
        }

        get("/alerts/{bridgeAddress}/{chain}") {
            val bridgeAddress =
                call.parameters["bridgeAddress"] ?: throw IllegalArgumentException("bridgeAddress required")
            val chain = call.parameters["chain"] ?: throw IllegalArgumentException("chain required")
            val from = call.parameters["from"]?.toLongOrNull() ?: (System.currentTimeMillis() - 86400_000)
            val to = call.parameters["to"]?.toLongOrNull() ?: System.currentTimeMillis()
            val results = store.getExploitAlerts(bridgeAddress, chain, TimeRange(from, to))
            call.respond(results)
        }

        get("/reentrancy") {
            val from = call.parameters["from"]?.toLongOrNull() ?: (System.currentTimeMillis() - 86400_000)
            val to = call.parameters["to"]?.toLongOrNull() ?: System.currentTimeMillis()
            val results = store.getReentrancyDetections(TimeRange(from, to))
            call.respond(results)
        }
    }
}
