package com.web3analytics.serde;

import com.web3analytics.models.SwapEvent;

public class SwapEventDeserializer extends AbstractCloudEventAvroDeserializer<SwapEvent, com.web3analytics.events.SwapEvent> {

    public SwapEventDeserializer() {
        super(com.web3analytics.events.SwapEvent.class, SwapEvent.class, "com.dex.events.swap");
    }

    @Override
    protected SwapEvent map(com.web3analytics.events.SwapEvent avroRecord) {
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
