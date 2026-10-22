package com.web3analytics.serde;

import com.web3analytics.models.BurnEvent;

/**
 * Deserializes BurnEvent from Avro binary payload.
 * Used by LiquidityEventDeserializer for heterogeneous topic routing.
 */
class BurnEventDeserializer {

  private final AvroDeserializer<BurnEvent, com.web3analytics.events.BurnEvent> avroDeserializer =
      new AvroDeserializer<>(com.web3analytics.events.BurnEvent.class) {
        @Override
        protected BurnEvent map(com.web3analytics.events.BurnEvent avroRecord) {
          return new BurnEvent(
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
              avroRecord.getAmount0(),
              avroRecord.getAmount1(),
              avroRecord.getEventTimestamp()
          );
        }
      };

  BurnEvent deserializeAvro(byte[] avroPayload) {
    return avroDeserializer.deserialize(avroPayload);
    }
}
