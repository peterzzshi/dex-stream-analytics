package com.web3analytics.serde;

import com.web3analytics.models.MintEvent;

/**
 * Deserializes MintEvent from Avro binary payload.
 * Used by LiquidityEventDeserializer for heterogeneous topic routing.
 */
class MintEventDeserializer {

  private final AvroDeserializer<MintEvent, com.web3analytics.events.MintEvent> avroDeserializer =
      new AvroDeserializer<>(com.web3analytics.events.MintEvent.class) {
        @Override
        protected MintEvent map(com.web3analytics.events.MintEvent avroRecord) {
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
