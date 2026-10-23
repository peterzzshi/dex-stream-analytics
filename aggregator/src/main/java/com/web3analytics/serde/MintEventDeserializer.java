package com.web3analytics.serde;

import com.web3analytics.models.MintEvent;
import java.math.BigDecimal;

/**
 * Deserializes MintEvent from Avro binary payload.
 * Used by LiquidityEventDeserializer for heterogeneous topic routing.
 */
class MintEventDeserializer {

  private final AvroDeserializer<MintEvent, com.web3analytics.events.MintEvent> avroDeserializer =
      new AvroDeserializer<>(com.web3analytics.events.MintEvent.class) {
        @Override
        protected MintEvent map(com.web3analytics.events.MintEvent avroRecord) {
          final BigDecimal amount0 =
              AmountValidation.requireNonNegative("amount0", avroRecord.getAmount0());
          final BigDecimal amount1 =
              AmountValidation.requireNonNegative("amount1", avroRecord.getAmount1());
          AmountValidation.requireAtLeastOnePositive("MintEvent", amount0, amount1);

          return new MintEvent(
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
              avroRecord.getAmount0(),
              avroRecord.getAmount1(),
              avroRecord.getEventTimestamp()
          );
        }
      };

  MintEvent deserializeAvro(byte[] avroPayload) {
    return avroDeserializer.deserialize(avroPayload);
    }
}
