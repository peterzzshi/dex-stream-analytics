package com.web3analytics.mappers;

import com.web3analytics.alerts.BridgeProtocol;
import com.web3analytics.alerts.TransferVolumeAlert;
import com.web3analytics.model.TransferVolumeAccumulator;
import java.io.Serial;
import java.math.BigDecimal;
import java.time.Duration;
import java.time.Instant;
import org.apache.flink.api.common.functions.MapFunction;

/**
 * Maps Flink accumulator to Avro alert schema. Computes windowStart from windowEnd and window
 * duration, severity from accumulated data, and converts BigInteger amounts to BigDecimal.
 */
public class TransferVolumeAlertMapper implements MapFunction<TransferVolumeAccumulator, TransferVolumeAlert> {

  @Serial
  private static final long serialVersionUID = 1L;

  private final long windowDurationMillis;

  public TransferVolumeAlertMapper(final Duration windowDuration) {
    this.windowDurationMillis = windowDuration.toMillis();
  }

  @Override
  public TransferVolumeAlert map(final TransferVolumeAccumulator acc) {
    final Instant windowStart = Instant.ofEpochMilli(acc.windowEnd - this.windowDurationMillis);
    final Instant windowEnd = Instant.ofEpochMilli(acc.windowEnd);

    return TransferVolumeAlert.newBuilder()
        .setBridgeAddress(acc.bridgeAddress)
        .setChain(acc.chain)
        .setProtocol(BridgeProtocol.valueOf(acc.protocol))
        .setWindowStart(windowStart)
        .setWindowEnd(windowEnd)
        .setSeverity(AlertSeverity.compute(acc))
        .setAlertType("TRANSFER_VOLUME")
        .setLockCount(acc.lockCount)
        .setUnlockCount(acc.unlockCount)
        .setTotalLockAmount(new BigDecimal(acc.totalLockAmount))
        .setTotalUnlockAmount(new BigDecimal(acc.totalUnlockAmount))
        .setToken0Symbol(acc.token0Symbol.toNullable())
        .setToken1Symbol(acc.token1Symbol.toNullable())
        .build();
  }
}
