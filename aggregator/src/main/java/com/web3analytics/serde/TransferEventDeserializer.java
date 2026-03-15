package com.web3analytics.serde;

import com.web3analytics.models.TransferEvent;

/**
 * Deserializes TransferEvent from Kafka payload.
 * Expects Dapr CloudEvents envelopes with Avro payload in data/data_base64.
 */
public class TransferEventDeserializer extends AbstractCloudEventAvroDeserializer<TransferEvent, com.web3analytics.events.TransferEvent> {

    public TransferEventDeserializer() {
        super(com.web3analytics.events.TransferEvent.class, TransferEvent.class);
    }

    @Override
    protected TransferEvent map(com.web3analytics.events.TransferEvent record) {
        return new TransferEvent(
                record.getEventId(),
                record.getBlockNumber(),
                record.getBlockTimestamp(),
                record.getTransactionHash(),
                record.getLogIndex(),
                record.getPairAddress(),
                record.getToken0(),
                record.getToken1(),
                record.getToken0Symbol(),
                record.getToken1Symbol(),
                record.getFrom(),
                record.getTo(),
                record.getValue(),
                record.getEventTimestamp()
        );
    }
}
