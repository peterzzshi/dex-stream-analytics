package com.web3analytics.alerting.model

import java.math.BigInteger
import com.web3analytics.alerts.CrossChainFlowAlert as AvroCrossChainFlowAlert
import com.web3analytics.alerts.ExploitDetectionAlert as AvroExploitDetectionAlert
import com.web3analytics.alerts.ReentrancyAlert as AvroReentrancyAlert
import com.web3analytics.alerts.SupplyTrackingAlert as AvroSupplyTrackingAlert
import com.web3analytics.alerts.TransferVolumeAlert as AvroTransferVolumeAlert

fun AvroTransferVolumeAlert.toKotlinAlert(): TransferVolumeAlert =
    TransferVolumeAlert(
        bridgeAddress = this.bridgeAddress.toString(),
        chain = this.chain.toString(),
        protocol = BridgeProtocol.valueOf(this.protocol.name),
        windowStart = this.windowStart,
        windowEnd = this.windowEnd,
        severity = Severity.valueOf(this.severity.name),
        alertType = this.alertType.toString(),
        lockCount = this.lockCount,
        unlockCount = this.unlockCount,
        totalLockAmount = BigInteger(this.totalLockAmount.toString()),
        totalUnlockAmount = BigInteger(this.totalUnlockAmount.toString()),
        token0Symbol = this.token0Symbol,
        token1Symbol = this.token1Symbol,
    )

fun AvroSupplyTrackingAlert.toKotlinAlert(): SupplyTrackingAlert =
    SupplyTrackingAlert(
        bridgeAddress = this.bridgeAddress.toString(),
        chain = this.chain.toString(),
        protocol = BridgeProtocol.valueOf(this.protocol.name),
        windowStart = this.windowStart,
        windowEnd = this.windowEnd,
        severity = Severity.valueOf(this.severity.name),
        alertType = this.alertType.toString(),
        mintCount = this.mintCount,
        burnCount = this.burnCount,
        totalMinted = BigInteger(this.totalMinted.toString()),
        totalBurned = BigInteger(this.totalBurned.toString()),
        netMinted = BigInteger(this.netMinted.toString()),
        tokenSymbol = this.tokenSymbol,
    )

fun AvroCrossChainFlowAlert.toKotlinAlert(): CrossChainFlowAlert =
    CrossChainFlowAlert(
        bridgeAddress = this.bridgeAddress.toString(),
        protocol = BridgeProtocol.valueOf(this.protocol.name),
        windowStart = this.windowStart,
        windowEnd = this.windowEnd,
        severity = Severity.valueOf(this.severity.name),
        alertType = this.alertType.toString(),
        totalInflow = BigInteger(this.totalInflow.toString()),
        totalOutflow = BigInteger(this.totalOutflow.toString()),
        netFlow = BigInteger(this.netFlow.toString()),
        imbalanceRatio = this.imbalanceRatio,
    )

fun AvroExploitDetectionAlert.toKotlinAlert(): ExploitDetectionAlert =
    ExploitDetectionAlert(
        bridgeAddress = this.bridgeAddress.toString(),
        chain = this.chain.toString(),
        protocol = BridgeProtocol.valueOf(this.protocol.name),
        windowStart = this.windowStart,
        windowEnd = this.windowEnd,
        severity = Severity.valueOf(this.severity.name),
        alertType = this.alertType.toString(),
        suspiciousPatterns = this.suspiciousPatterns.map { SuspiciousPattern.valueOf(it.name) },
        largestSingleTransfer = BigInteger(this.largestSingleTransfer.toString()),
        rapidTransferCount = this.rapidTransferCount,
        supplyMismatchDetected = this.supplyMismatchDetected,
        totalLocked = BigInteger(this.totalLocked.toString()),
        totalUnlocked = BigInteger(this.totalUnlocked.toString()),
        totalMinted = BigInteger(this.totalMinted.toString()),
        totalBurned = BigInteger(this.totalBurned.toString()),
    )

fun AvroReentrancyAlert.toKotlinAlert(): ReentrancyAlert =
    ReentrancyAlert(
        bridgeAddress = this.bridgeAddress.toString(),
        chain = this.chain.toString(),
        windowStart = this.windowStart,
        windowEnd = this.windowEnd,
        severity = Severity.valueOf(this.severity.name),
        alertType = this.alertType.toString(),
        transactionHash = this.transactionHash.toString(),
        lockCount = this.lockCount,
        unlockCount = this.unlockCount,
    )
