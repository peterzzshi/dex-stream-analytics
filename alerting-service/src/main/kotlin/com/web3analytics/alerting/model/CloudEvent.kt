package com.web3analytics.alerting.model

import com.fasterxml.jackson.annotation.JsonProperty
import org.apache.avro.Schema
import org.apache.avro.generic.GenericDatumReader
import org.apache.avro.generic.GenericRecord
import org.apache.avro.io.DecoderFactory
import java.math.BigInteger
import java.nio.ByteBuffer
import java.time.Instant
import java.util.Base64

private inline fun <reified T> GenericRecord.field(name: String): T {
    val value = this[name] ?: throw IllegalArgumentException("Missing field: $name")
    return when {
        value is T -> value
        T::class == String::class && value is org.apache.avro.util.Utf8 -> value.toString() as T
        T::class == BigInteger::class && value is ByteBuffer -> bytesToBigInteger(value) as T
        else -> throw ClassCastException("Field $name: expected ${T::class.simpleName}, got ${value::class.simpleName}")
    }
}

private inline fun <reified T> GenericRecord.fieldOrNull(name: String): T? {
    val value = this[name]
    return when {
        value == null -> null
        value is T -> value
        T::class == String::class && value is org.apache.avro.util.Utf8 -> value.toString() as T
        else -> throw ClassCastException(
            "Field $name: expected ${T::class.simpleName}?, got ${value::class.simpleName}"
        )
    }
}

private fun bytesToBigInteger(buffer: ByteBuffer): BigInteger {
    val bytes = ByteArray(buffer.remaining())
    buffer.get(bytes)
    return BigInteger(bytes)
}

data class CloudEventWrapper<T>(
    val specversion: String,
    val type: String,
    val source: String,
    val id: String,
    val datacontenttype: String,
    @JsonProperty("data") val dataJson: String? = null,
    @JsonProperty("data_base64") val dataBase64: String? = null
)

object CloudEventDeserializer {
    fun deserializeTransferVolume(
        cloudEvent: CloudEventWrapper<*>,
        schema: Schema
    ): TransferVolumeAlert {
        val record = decodeAvro(cloudEvent, schema)
        return TransferVolumeAlert(
            bridgeAddress = record.field("bridgeAddress"),
            chain = record.field("chain"),
            protocol = BridgeProtocol.valueOf(record.field<String>("protocol")),
            windowStart = Instant.ofEpochMilli(record.field("windowStart")),
            windowEnd = Instant.ofEpochMilli(record.field("windowEnd")),
            severity = Severity.valueOf(record.field<String>("severity")),
            alertType = record.field("alertType"),
            lockCount = record.field("lockCount"),
            unlockCount = record.field("unlockCount"),
            totalLockAmount = record.field("totalLockAmount"),
            totalUnlockAmount = record.field("totalUnlockAmount"),
            token0Symbol = record.fieldOrNull("token0Symbol"),
            token1Symbol = record.fieldOrNull("token1Symbol")
        )
    }

    fun deserializeSupplyTracking(
        cloudEvent: CloudEventWrapper<*>,
        schema: Schema
    ): SupplyTrackingAlert {
        val record = decodeAvro(cloudEvent, schema)
        return SupplyTrackingAlert(
            bridgeAddress = record.field("bridgeAddress"),
            chain = record.field("chain"),
            protocol = BridgeProtocol.valueOf(record.field<String>("protocol")),
            windowStart = Instant.ofEpochMilli(record.field("windowStart")),
            windowEnd = Instant.ofEpochMilli(record.field("windowEnd")),
            severity = Severity.valueOf(record.field<String>("severity")),
            alertType = record.field("alertType"),
            mintCount = record.field("mintCount"),
            burnCount = record.field("burnCount"),
            totalMinted = record.field("totalMinted"),
            totalBurned = record.field("totalBurned"),
            netMinted = record.field("netMinted"),
            tokenSymbol = record.fieldOrNull("tokenSymbol")
        )
    }

    fun deserializeCrossChainFlow(
        cloudEvent: CloudEventWrapper<*>,
        schema: Schema
    ): CrossChainFlowAlert {
        val record = decodeAvro(cloudEvent, schema)
        return CrossChainFlowAlert(
            bridgeAddress = record.field("bridgeAddress"),
            protocol = BridgeProtocol.valueOf(record.field<String>("protocol")),
            windowStart = Instant.ofEpochMilli(record.field("windowStart")),
            windowEnd = Instant.ofEpochMilli(record.field("windowEnd")),
            severity = Severity.valueOf(record.field<String>("severity")),
            alertType = record.field("alertType"),
            totalInflow = record.field("totalInflow"),
            totalOutflow = record.field("totalOutflow"),
            netFlow = record.field("netFlow"),
            imbalanceRatio = record.field("imbalanceRatio")
        )
    }

    fun deserializeExploitDetection(
        cloudEvent: CloudEventWrapper<*>,
        schema: Schema
    ): ExploitDetectionAlert {
        val record = decodeAvro(cloudEvent, schema)

        @Suppress("UNCHECKED_CAST")
        val patterns =
            (record["suspiciousPatterns"] as List<*>).map {
                SuspiciousPattern.valueOf(it.toString())
            }
        return ExploitDetectionAlert(
            bridgeAddress = record.field("bridgeAddress"),
            chain = record.field("chain"),
            protocol = BridgeProtocol.valueOf(record.field<String>("protocol")),
            windowStart = Instant.ofEpochMilli(record.field("windowStart")),
            windowEnd = Instant.ofEpochMilli(record.field("windowEnd")),
            severity = Severity.valueOf(record.field<String>("severity")),
            alertType = record.field("alertType"),
            suspiciousPatterns = patterns,
            largestSingleTransfer = record.field("largestSingleTransfer"),
            rapidTransferCount = record.field("rapidTransferCount"),
            supplyMismatchDetected = record.field("supplyMismatchDetected"),
            totalLocked = record.field("totalLocked"),
            totalUnlocked = record.field("totalUnlocked"),
            totalMinted = record.field("totalMinted"),
            totalBurned = record.field("totalBurned")
        )
    }

    fun deserializeReentrancy(
        cloudEvent: CloudEventWrapper<*>,
        schema: Schema
    ): ReentrancyAlert {
        val record = decodeAvro(cloudEvent, schema)
        return ReentrancyAlert(
            bridgeAddress = record.field("bridgeAddress"),
            chain = record.field("chain"),
            windowStart = Instant.ofEpochMilli(record.field("windowStart")),
            windowEnd = Instant.ofEpochMilli(record.field("windowEnd")),
            severity = Severity.valueOf(record.field<String>("severity")),
            alertType = record.field("alertType"),
            transactionHash = record.field("transactionHash"),
            lockCount = record.field("lockCount"),
            unlockCount = record.field("unlockCount")
        )
    }

    private fun decodeAvro(
        cloudEvent: CloudEventWrapper<*>,
        schema: Schema
    ): GenericRecord {
        val base64Data =
            cloudEvent.dataBase64
                ?: throw IllegalArgumentException("Missing data_base64 field")
        val bytes = Base64.getDecoder().decode(base64Data)
        val decoder = DecoderFactory.get().binaryDecoder(bytes, null)
        val reader = GenericDatumReader<GenericRecord>(schema)
        return reader.read(null, decoder)
    }
}
