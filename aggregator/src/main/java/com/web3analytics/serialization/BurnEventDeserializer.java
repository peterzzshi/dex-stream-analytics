package com.web3analytics.serialization;

import com.web3analytics.models.BurnEvent;
import org.apache.avro.generic.GenericRecord;

import java.io.IOException;

/**
 * Deserializes BurnEvent from Kafka payload.
 * Expects Dapr CloudEvents envelopes with Avro payload in data/data_base64.
 */
public class BurnEventDeserializer extends AbstractCloudEventAvroDeserializer<BurnEvent> {

    public BurnEventDeserializer() throws IOException {
        super("/avro/BurnEvent.avsc", BurnEvent.class);
    }

    @Override
    protected BurnEvent map(GenericRecord record) {
        return new BurnEvent(
                AvroFieldReader.string(record, "eventId"),
                AvroFieldReader.longValue(record, "blockNumber"),
                AvroFieldReader.longValue(record, "blockTimestamp"),
                AvroFieldReader.string(record, "transactionHash"),
                AvroFieldReader.intValue(record, "logIndex"),
                AvroFieldReader.string(record, "pairAddress"),
                AvroFieldReader.string(record, "token0"),
                AvroFieldReader.string(record, "token1"),
                AvroFieldReader.nullableString(record, "token0Symbol"),
                AvroFieldReader.nullableString(record, "token1Symbol"),
                AvroFieldReader.string(record, "sender"),
                AvroFieldReader.string(record, "recipient"),
                AvroFieldReader.string(record, "amount0"),
                AvroFieldReader.string(record, "amount1"),
                AvroFieldReader.longValue(record, "eventTimestamp")
        );
    }
}
