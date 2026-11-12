package com.web3analytics.mappers;

import com.web3analytics.alerts.BridgeProtocol;
import com.web3analytics.alerts.CrossChainFlowAlert;
import com.web3analytics.model.CrossChainFlowAccumulator;
import java.io.Serial;
import java.math.BigDecimal;
import java.time.Duration;
import java.time.Instant;
import org.apache.flink.api.common.functions.MapFunction;

public class CrossChainFlowAlertMapper implements MapFunction<CrossChainFlowAccumulator, CrossChainFlowAlert> {

  @Serial
  private static final long serialVersionUID = 1L;

  private final long windowDurationMillis;

  public CrossChainFlowAlertMapper(final Duration windowDuration) {
    this.windowDurationMillis = windowDuration.toMillis();
  }

  @Override
  public CrossChainFlowAlert map(final CrossChainFlowAccumulator acc) {
    final Instant windowStart = Instant.ofEpochMilli(acc.windowEnd - this.windowDurationMillis);
    final Instant windowEnd = Instant.ofEpochMilli(acc.windowEnd);

    return CrossChainFlowAlert.newBuilder()
        .setBridgeAddress(acc.bridgeAddress)
        .setProtocol(BridgeProtocol.valueOf(acc.protocol))
        .setWindowStart(windowStart)
        .setWindowEnd(windowEnd)
        .setSeverity(AlertSeverity.compute(acc))
        .setAlertType("CROSS_CHAIN_FLOW")
        .setTotalInflow(new BigDecimal(acc.totalInflow))
        .setTotalOutflow(new BigDecimal(acc.totalOutflow))
        .setNetFlow(new BigDecimal(acc.netFlow))
        .setImbalanceRatio(acc.imbalanceRatio)
        .build();
  }
}
