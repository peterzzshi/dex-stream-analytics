package com.web3analytics.serialization;

import com.web3analytics.models.SwapEvent;
import org.apache.avro.generic.GenericRecord;

import java.io.IOException;

/**
 * Deserializes SwapEvent from Kafka payload.
 * Expects Dapr CloudEvents envelopes with Avro payload in data/data_base64.
 */
public class SwapEventDeserializer extends AbstractCloudEventAvroDeserializer<SwapEvent> {

    public SwapEventDeserializer() throws IOException {
        super("/avro/SwapEvent.avsc", SwapEvent.class);
    }

    @Override
    protected SwapEvent map(GenericRecord record) {
        return new SwapEvent(
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
                AvroFieldReader.string(record, "amount0In"),
                AvroFieldReader.string(record, "amount1In"),
                AvroFieldReader.string(record, "amount0Out"),
                AvroFieldReader.string(record, "amount1Out"),
                AvroFieldReader.doubleValue(record, "price"),
                AvroFieldReader.nullableDouble(record, "volumeUSD"),
                AvroFieldReader.longValue(record, "gasUsed"),
                AvroFieldReader.string(record, "gasPrice"),
                AvroFieldReader.longValue(record, "eventTimestamp")
        );
    }
}
