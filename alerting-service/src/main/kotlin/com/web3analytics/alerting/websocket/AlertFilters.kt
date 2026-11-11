package com.web3analytics.alerting.websocket

import com.web3analytics.alerting.model.Severity

data class AlertFilters(
    val bridgeAddress: String? = null,
    val chain: String? = null,
    val minSeverity: Severity? = null,
    val alertTypes: Set<String>? = null
) {
    companion object {
        fun fromQueryParams(params: Map<String, List<String>>): AlertFilters =
            AlertFilters(
                bridgeAddress = params["bridgeAddress"]?.firstOrNull(),
                chain = params["chain"]?.firstOrNull(),
                minSeverity = params["severity"]?.firstOrNull()?.let { Severity.valueOf(it.uppercase()) },
                alertTypes = params["alertTypes"]?.toSet()
            )
    }

    fun matches(alert: com.web3analytics.alerting.model.Alert): Boolean {
        if (bridgeAddress != null && alert.bridgeAddress != bridgeAddress) return false

        val alertChain =
            when (alert) {
                is com.web3analytics.alerting.model.TransferVolumeAlert -> alert.chain
                is com.web3analytics.alerting.model.SupplyTrackingAlert -> alert.chain
                is com.web3analytics.alerting.model.ExploitDetectionAlert -> alert.chain
                is com.web3analytics.alerting.model.ReentrancyAlert -> alert.chain
                is com.web3analytics.alerting.model.CrossChainFlowAlert -> null
            }
        if (chain != null && alertChain != chain) return false

        if (minSeverity != null && alert.severity.ordinal < minSeverity.ordinal) return false

        if (alertTypes != null && alert.alertType !in alertTypes) return false

        return true
    }
}
