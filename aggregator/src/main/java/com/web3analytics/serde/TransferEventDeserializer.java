package com.web3analytics.serde;

import com.web3analytics.models.TransferEvent;
import java.math.BigDecimal;

/**
 * Deserializes TransferEvent from Avro binary payload.
 * Used by LiquidityEventDeserializer for heterogeneous topic routing.
 */
class TransferEventDeserializer {

  private final AvroDeserializer<TransferEvent, com.web3analytics.events.TransferEvent> avroDeserializer =
      new AvroDeserializer<>(com.web3analytics.events.TransferEvent.class) {
        @Override
        protected TransferEvent map(com.web3analytics.events.TransferEvent avroRecord) {
          final BigDecimal value =
              AmountValidation.requireNonNegative("value", avroRecord.getValue());
          AmountValidation.requireAtLeastOnePositive("TransferEvent", value);

          return new TransferEvent(
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
              avroRecord.getFrom(),
              avroRecord.getTo(),
              avroRecord.getValue(),
              avroRecord.getEventTimestamp()
          );
        }
      };

  TransferEvent deserializeAvro(byte[] avroPayload) {
    return avroDeserializer.deserialize(avroPayload);
    }
}
