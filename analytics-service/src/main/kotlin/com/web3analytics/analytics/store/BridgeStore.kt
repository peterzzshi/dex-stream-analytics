package com.web3analytics.analytics.store

import com.web3analytics.analytics.model.CrossChainFlowResult
import com.web3analytics.analytics.model.ExploitDetectionResult
import com.web3analytics.analytics.model.ReentrancyResult
import com.web3analytics.analytics.model.SupplyTrackingResult
import com.web3analytics.analytics.model.TimeRange
import com.web3analytics.analytics.model.TransferVolumeResult

/**
 * Storage interface for bridge security analytics.
 */
interface BridgeStore {
    fun storeTransferVolume(result: TransferVolumeResult)

    fun storeSupplyTracking(result: SupplyTrackingResult)

    fun storeCrossChainFlow(result: CrossChainFlowResult)

    fun storeExploitDetection(result: ExploitDetectionResult)

    fun storeReentrancy(result: ReentrancyResult)

    fun getTransferVolume(
        chain: String,
        bridgeAddress: String,
        range: TimeRange
    ): List<TransferVolumeResult>

    fun getSupplyTracking(
        chain: String,
        bridgeAddress: String,
        range: TimeRange
    ): List<SupplyTrackingResult>

    fun getCrossChainFlow(
        bridgeAddress: String,
        range: TimeRange
    ): List<CrossChainFlowResult>

    fun getExploitAlerts(
        bridgeAddress: String,
        chain: String,
        range: TimeRange
    ): List<ExploitDetectionResult>

    fun getReentrancyDetections(range: TimeRange): List<ReentrancyResult>
}
