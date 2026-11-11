package com.web3analytics.alerting.consumer

import com.fasterxml.jackson.databind.ObjectMapper
import com.web3analytics.alerting.model.CloudEventDeserializer
import com.web3analytics.alerting.model.CloudEventWrapper
import com.web3analytics.alerting.store.AlertStore
import com.web3analytics.alerting.websocket.AlertBroadcaster
import org.apache.avro.Schema
import org.slf4j.LoggerFactory
import java.io.File

class AlertConsumer(
    private val store: AlertStore,
    private val broadcaster: AlertBroadcaster,
    private val mapper: ObjectMapper,
    schemasPath: String = "/app/schemas"
) {
    private val logger = LoggerFactory.getLogger(javaClass)

    private val transferVolumeSchema: Schema = loadSchema("$schemasPath/TransferVolumeAlert.avsc")
    private val supplyTrackingSchema: Schema = loadSchema("$schemasPath/SupplyTrackingAlert.avsc")
    private val crossChainFlowSchema: Schema = loadSchema("$schemasPath/CrossChainFlowAlert.avsc")
    private val exploitDetectionSchema: Schema = loadSchema("$schemasPath/ExploitDetectionAlert.avsc")
    private val reentrancySchema: Schema = loadSchema("$schemasPath/ReentrancyAlert.avsc")

    private fun loadSchema(path: String): Schema = Schema.Parser().parse(File(path))

    suspend fun handleTransferVolume(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = CloudEventDeserializer.deserializeTransferVolume(cloudEvent, transferVolumeSchema)
            store.storeAlert(alert)
            broadcaster.broadcast(alert)
            logger.info(
                "Processed TransferVolumeAlert: ${alert.bridgeAddress} ${alert.chain} severity=${alert.severity}"
            )
        } catch (e: Exception) {
            logger.error("Failed to handle TransferVolumeAlert", e)
        }
    }

    suspend fun handleSupplyTracking(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = CloudEventDeserializer.deserializeSupplyTracking(cloudEvent, supplyTrackingSchema)
            store.storeAlert(alert)
            broadcaster.broadcast(alert)
            logger.info(
                "Processed SupplyTrackingAlert: ${alert.bridgeAddress} ${alert.chain} severity=${alert.severity}"
            )
        } catch (e: Exception) {
            logger.error("Failed to handle SupplyTrackingAlert", e)
        }
    }

    suspend fun handleCrossChainFlow(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = CloudEventDeserializer.deserializeCrossChainFlow(cloudEvent, crossChainFlowSchema)
            store.storeAlert(alert)
            broadcaster.broadcast(alert)
            logger.info("Processed CrossChainFlowAlert: ${alert.bridgeAddress} severity=${alert.severity}")
        } catch (e: Exception) {
            logger.error("Failed to handle CrossChainFlowAlert", e)
        }
    }

    suspend fun handleExploitDetection(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = CloudEventDeserializer.deserializeExploitDetection(cloudEvent, exploitDetectionSchema)
            store.storeAlert(alert)
            broadcaster.broadcast(alert)
            logger.info(
                "Processed ExploitDetectionAlert: ${alert.bridgeAddress} ${alert.chain} patterns=${alert.suspiciousPatterns.size}"
            )
        } catch (e: Exception) {
            logger.error("Failed to handle ExploitDetectionAlert", e)
        }
    }

    suspend fun handleReentrancy(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = CloudEventDeserializer.deserializeReentrancy(cloudEvent, reentrancySchema)
            store.storeAlert(alert)
            broadcaster.broadcast(alert)
            logger.info("Processed ReentrancyAlert: ${alert.transactionHash} locks=${alert.lockCount}")
        } catch (e: Exception) {
            logger.error("Failed to handle ReentrancyAlert", e)
        }
    }
}
