package com.web3analytics.serde;

import com.web3analytics.models.SwapEvent;

public class SwapEventDeserializer extends AbstractCloudEventAvroDeserializer<SwapEvent, com.web3analytics.events.SwapEvent> {

    public SwapEventDeserializer() {
        super(com.web3analytics.events.SwapEvent.class, SwapEvent.class);
    }

    @Override
    protected SwapEvent map(com.web3analytics.events.SwapEvent record) {
        return new SwapEvent(
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
                record.getAmount0In(),
                record.getAmount1In(),
                record.getAmount0Out(),
                record.getAmount1Out(),
                record.getPrice(),
                record.getVolumeUSD(),
                record.getGasUsed(),
                record.getGasPrice(),
                record.getEventTimestamp()
        );
    }
}
