package com.web3analytics.serde;

import com.web3analytics.models.MintEvent;

/**
 * Deserializes MintEvent from Kafka payload.
 * Expects Dapr CloudEvents envelopes with Avro payload in data/data_base64.
 */
public class MintEventDeserializer extends AbstractCloudEventAvroDeserializer<MintEvent, com.web3analytics.events.MintEvent> {

    public MintEventDeserializer() {
        super(com.web3analytics.events.MintEvent.class, MintEvent.class);
    }

    @Override
    protected MintEvent map(com.web3analytics.events.MintEvent record) {
        return new MintEvent(
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
                record.getAmount0(),
                record.getAmount1(),
                record.getEventTimestamp()
        );
    }
}
