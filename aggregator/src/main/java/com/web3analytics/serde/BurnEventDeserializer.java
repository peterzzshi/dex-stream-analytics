package com.web3analytics.serde;

import com.web3analytics.models.BurnEvent;

/**
 * Deserializes BurnEvent from Kafka payload.
 * Expects Dapr CloudEvents envelopes with Avro payload in data/data_base64.
 */
public class BurnEventDeserializer extends AbstractCloudEventAvroDeserializer<BurnEvent, com.web3analytics.events.BurnEvent> {

    public BurnEventDeserializer() {
        super(com.web3analytics.events.BurnEvent.class, BurnEvent.class);
    }

    @Override
    protected BurnEvent map(com.web3analytics.events.BurnEvent record) {
        return new BurnEvent(
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
                record.getSender(),
                record.getRecipient(),
                record.getAmount0(),
                record.getAmount1(),
                record.getEventTimestamp()
        );
    }
}
