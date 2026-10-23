package com.web3analytics.serde;

import com.web3analytics.models.SwapEvent;
import java.io.Serial;
import java.math.BigDecimal;

public class SwapEventDeserializer extends AbstractCloudEventAvroDeserializer<SwapEvent, com.web3analytics.events.SwapEvent> {

  @Serial
  private static final long serialVersionUID = 6343419917686105735L;

  public SwapEventDeserializer() {
        super(com.web3analytics.events.SwapEvent.class, SwapEvent.class, "com.dex.events.swap");
    }

    @Override
    protected SwapEvent map(final com.web3analytics.events.SwapEvent avroRecord) {
      final BigDecimal amount0In =
          AmountValidation.requireNonNegative("amount0In", avroRecord.getAmount0In());
      final BigDecimal amount1In =
          AmountValidation.requireNonNegative("amount1In", avroRecord.getAmount1In());
      final BigDecimal amount0Out =
          AmountValidation.requireNonNegative("amount0Out", avroRecord.getAmount0Out());
      final BigDecimal amount1Out =
          AmountValidation.requireNonNegative("amount1Out", avroRecord.getAmount1Out());
      AmountValidation.requireAtLeastOnePositive(
          "SwapEvent", amount0In, amount1In, amount0Out, amount1Out);

        return new SwapEvent(
            avroRecord.getEventId(),
            avroRecord.getBlockNumber(),
            avroRecord.getBlockTimestamp(),
            avroRecord.getTransactionHash(),
            avroRecord.getLogIndex(),
            avroRecord.getChain(),
            avroRecord.getChainName(),
            avroRecord.getPairAddress(),
            avroRecord.getToken0(),
            avroRecord.getToken1(),
            avroRecord.getToken0Symbol(),
            avroRecord.getToken1Symbol(),
            avroRecord.getSender(),
            avroRecord.getRecipient(),
            avroRecord.getAmount0In(),
            avroRecord.getAmount1In(),
            avroRecord.getAmount0Out(),
            avroRecord.getAmount1Out(),
            avroRecord.getPrice(),
            avroRecord.getVolumeUSD(),
            avroRecord.getGasUsed(),
            avroRecord.getGasPrice(),
            avroRecord.getEventTimestamp()
        );
    }
}
