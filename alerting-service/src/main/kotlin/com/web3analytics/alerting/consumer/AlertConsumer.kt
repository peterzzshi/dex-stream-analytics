package com.web3analytics.alerting.consumer

import com.fasterxml.jackson.annotation.JsonProperty
import com.fasterxml.jackson.databind.ObjectMapper
import com.web3analytics.alerting.model.toKotlinAlert
import com.web3analytics.alerting.store.AlertStore
import com.web3analytics.alerting.websocket.AlertBroadcaster
import com.web3analytics.alerts.CrossChainFlowAlert
import com.web3analytics.alerts.ExploitDetectionAlert
import com.web3analytics.alerts.ReentrancyAlert
import com.web3analytics.alerts.SupplyTrackingAlert
import com.web3analytics.alerts.TransferVolumeAlert
import java.util.Base64
import org.apache.avro.io.DecoderFactory
import org.apache.avro.specific.SpecificDatumReader
import org.slf4j.LoggerFactory

data class CloudEventWrapper(
    val specversion: String,
    val type: String,
    val source: String,
    val id: String,
    val datacontenttype: String,
    @JsonProperty("data") val dataJson: String? = null,
    @JsonProperty("data_base64") val dataBase64: String? = null,
)

class AlertConsumer(
    private val store: AlertStore,
    private val broadcaster: AlertBroadcaster,
    private val mapper: ObjectMapper,
) {
    private val logger = LoggerFactory.getLogger(javaClass)

    suspend fun handleTransferVolume(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = deserialize(cloudEvent, TransferVolumeAlert::class.java)
            store.storeAlert(alert.toKotlinAlert())
            broadcaster.broadcast(alert.toKotlinAlert())
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
            val alert = deserialize(cloudEvent, SupplyTrackingAlert::class.java)
            store.storeAlert(alert.toKotlinAlert())
            broadcaster.broadcast(alert.toKotlinAlert())
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
            val alert = deserialize(cloudEvent, CrossChainFlowAlert::class.java)
            store.storeAlert(alert.toKotlinAlert())
            broadcaster.broadcast(alert.toKotlinAlert())
            logger.info("Processed CrossChainFlowAlert: ${alert.bridgeAddress} severity=${alert.severity}")
        } catch (e: Exception) {
            logger.error("Failed to handle CrossChainFlowAlert", e)
        }
    }

    suspend fun handleExploitDetection(body: String) {
        try {
            val cloudEvent = mapper.readValue(body, CloudEventWrapper::class.java)
            val alert = deserialize(cloudEvent, ExploitDetectionAlert::class.java)
            store.storeAlert(alert.toKotlinAlert())
            broadcaster.broadcast(alert.toKotlinAlert())
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
            val alert = deserialize(cloudEvent, ReentrancyAlert::class.java)
            store.storeAlert(alert.toKotlinAlert())
            broadcaster.broadcast(alert.toKotlinAlert())
            logger.info("Processed ReentrancyAlert: ${alert.transactionHash} locks=${alert.lockCount}")
        } catch (e: Exception) {
            logger.error("Failed to handle ReentrancyAlert", e)
        }
    }

    private fun <T : org.apache.avro.specific.SpecificRecordBase> deserialize(
        cloudEvent: CloudEventWrapper,
        clazz: Class<T>,
    ): T {
        val base64Data =
            cloudEvent.dataBase64
                ?: throw IllegalArgumentException("Missing data_base64 field")
        val bytes = Base64.getDecoder().decode(base64Data)
        val decoder = DecoderFactory.get().binaryDecoder(bytes, null)
        val reader = SpecificDatumReader(clazz)
        return reader.read(null, decoder)
    }
}
