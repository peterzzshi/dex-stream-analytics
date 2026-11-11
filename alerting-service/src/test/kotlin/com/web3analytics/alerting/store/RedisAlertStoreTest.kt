package com.web3analytics.alerting.store

import com.web3analytics.alerting.model.*
import kotlinx.coroutines.runBlocking
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import java.math.BigInteger
import java.time.Instant

class RedisAlertStoreTest {
    private lateinit var store: MockRedisAlertStore

    @BeforeEach
    fun setup() {
        store = MockRedisAlertStore()
    }

    @Test
    fun `storeAlert stores alert`() =
        runBlocking {
            val alert = createTransferVolumeAlert()
            store.storeAlert(alert)

            val stored = store.getRecentAlerts("0xBridge123", "ethereum", 10)
            assertEquals(1, stored.size)
        }

    @Test
    fun `getRecentAlerts returns alerts in reverse chronological order`() =
        runBlocking {
            val older =
                createTransferVolumeAlert().copy(
                    windowEnd = Instant.now().minusSeconds(600)
                )
            val newer =
                createTransferVolumeAlert().copy(
                    windowEnd = Instant.now().minusSeconds(300)
                )
            val newest =
                createTransferVolumeAlert().copy(
                    windowEnd = Instant.now()
                )

            store.storeAlert(older)
            store.storeAlert(newer)
            store.storeAlert(newest)

            val alerts = store.getRecentAlerts("0xBridge123", "ethereum", 10)
            assertEquals(3, alerts.size)
            assertEquals(newest.windowEnd, alerts[0].windowEnd)
            assertEquals(newer.windowEnd, alerts[1].windowEnd)
            assertEquals(older.windowEnd, alerts[2].windowEnd)
        }

    @Test
    fun `storeAlert caps at limit`() =
        runBlocking {
            repeat(150) { i ->
                val alert =
                    createTransferVolumeAlert().copy(
                        windowEnd = Instant.now().plusSeconds(i.toLong())
                    )
                store.storeAlert(alert)
            }

            val alerts = store.getRecentAlerts("0xBridge123", "ethereum", 200)
            assertEquals(100, alerts.size)
        }

    @Test
    fun `getRecentAlerts filters by bridgeAddress and chain`() =
        runBlocking {
            val ethAlert = createTransferVolumeAlert()
            val solAlert = createSupplyTrackingAlert()

            store.storeAlert(ethAlert)
            store.storeAlert(solAlert)

            val ethAlerts = store.getRecentAlerts("0xBridge123", "ethereum", 10)
            assertEquals(1, ethAlerts.size)
            assertTrue(ethAlerts.first() is TransferVolumeAlert)
            assertEquals("0xBridge123", ethAlerts.first().bridgeAddress)

            val solAlerts = store.getRecentAlerts("0xBridge456", "solana", 10)
            assertEquals(1, solAlerts.size)
            assertTrue(solAlerts.first() is SupplyTrackingAlert)
        }

    private fun createTransferVolumeAlert() =
        TransferVolumeAlert(
            bridgeAddress = "0xBridge123",
            chain = "ethereum",
            protocol = BridgeProtocol.wormhole,
            windowStart = Instant.now().minusSeconds(300),
            windowEnd = Instant.now(),
            severity = Severity.HIGH,
            lockCount = 5,
            unlockCount = 3,
            totalLockAmount = BigInteger.valueOf(1000000),
            totalUnlockAmount = BigInteger.valueOf(500000),
            token0Symbol = "USDC",
            token1Symbol = "WETH"
        )

    private fun createSupplyTrackingAlert() =
        SupplyTrackingAlert(
            bridgeAddress = "0xBridge456",
            chain = "solana",
            protocol = BridgeProtocol.layerzero,
            windowStart = Instant.now().minusSeconds(300),
            windowEnd = Instant.now(),
            severity = Severity.MEDIUM,
            mintCount = 10,
            burnCount = 8,
            totalMinted = BigInteger.valueOf(2000000),
            totalBurned = BigInteger.valueOf(1800000),
            netMinted = BigInteger.valueOf(200000),
            tokenSymbol = "USDT"
        )

    class MockRedisAlertStore : AlertStore {
        private val storage = mutableMapOf<String, MutableList<Pair<Double, Alert>>>()

        override suspend fun storeAlert(alert: Alert) {
            val key = "bridge:alerts:${alert.bridgeAddress}:${getChain(alert)}"
            val score = alert.windowEnd.toEpochMilli().toDouble()

            val list = storage.getOrPut(key) { mutableListOf() }
            list.add(score to alert)
            storage[key] =
                list
                    .sortedByDescending { it.first }
                    .take(100)
                    .toMutableList()
        }

        override suspend fun getRecentAlerts(
            bridgeAddress: String,
            chain: String,
            limit: Int
        ): List<Alert> {
            val key = "bridge:alerts:$bridgeAddress:$chain"
            return storage[key]
                ?.take(limit)
                ?.map { it.second }
                ?: emptyList()
        }

        private fun getChain(alert: Alert): String =
            when (alert) {
                is TransferVolumeAlert -> alert.chain
                is SupplyTrackingAlert -> alert.chain
                is ExploitDetectionAlert -> alert.chain
                is ReentrancyAlert -> alert.chain
                is CrossChainFlowAlert -> "cross-chain"
            }
    }
}
