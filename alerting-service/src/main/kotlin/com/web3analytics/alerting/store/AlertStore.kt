package com.web3analytics.alerting.store

import com.web3analytics.alerting.model.Alert

interface AlertStore {
    suspend fun storeAlert(alert: Alert)

    suspend fun getRecentAlerts(
        bridgeAddress: String,
        chain: String,
        limit: Int = 100
    ): List<Alert>
}
