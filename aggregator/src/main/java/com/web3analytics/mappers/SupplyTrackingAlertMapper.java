package com.web3analytics.mappers;

import com.web3analytics.alerts.BridgeProtocol;
import com.web3analytics.alerts.SupplyTrackingAlert;
import com.web3analytics.model.SupplyTrackingAccumulator;
import java.io.Serial;
import java.math.BigDecimal;
import java.time.Duration;
import java.time.Instant;
import org.apache.flink.api.common.functions.MapFunction;

public class SupplyTrackingAlertMapper implements MapFunction<SupplyTrackingAccumulator, SupplyTrackingAlert> {

  @Serial
  private static final long serialVersionUID = 1L;

  private final long windowDurationMillis;

  public SupplyTrackingAlertMapper(final Duration windowDuration) {
    this.windowDurationMillis = windowDuration.toMillis();
  }

  @Override
  public SupplyTrackingAlert map(final SupplyTrackingAccumulator acc) {
    final Instant windowStart = Instant.ofEpochMilli(acc.windowEnd - this.windowDurationMillis);
    final Instant windowEnd = Instant.ofEpochMilli(acc.windowEnd);

    return SupplyTrackingAlert.newBuilder()
        .setBridgeAddress(acc.bridgeAddress)
        .setChain(acc.chain)
        .setProtocol(BridgeProtocol.valueOf(acc.protocol))
        .setWindowStart(windowStart)
        .setWindowEnd(windowEnd)
        .setSeverity(AlertSeverity.compute(acc))
        .setAlertType("SUPPLY_TRACKING")
        .setMintCount(acc.mintCount)
        .setBurnCount(acc.burnCount)
        .setTotalMinted(new BigDecimal(acc.totalMinted))
        .setTotalBurned(new BigDecimal(acc.totalBurned))
        .setNetMinted(new BigDecimal(acc.netMinted))
        .setTokenSymbol(acc.tokenSymbol.toNullable())
        .build();
  }
}
