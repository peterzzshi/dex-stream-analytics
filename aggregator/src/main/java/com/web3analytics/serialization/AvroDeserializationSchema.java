package com.web3analytics.serialization;

import com.web3analytics.models.SwapEvent;
import io.confluent.kafka.serializers.AbstractKafkaSchemaSerDeConfig;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.apache.flink.formats.avro.utils.MutableByteArrayInputStream;
import org.apache.flink.shaded.jackson2.com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;

public class AvroDeserializationSchema implements DeserializationSchema<SwapEvent> {
    private final ObjectMapper mapper = new ObjectMapper();

    @Override
    public SwapEvent deserialize(byte[] message) throws IOException {
        return mapper.readValue(message, SwapEvent.class);
    }

    @Override
    public boolean isEndOfStream(SwapEvent nextElement) {
        return false;
    }

    @Override
    public TypeInformation<SwapEvent> getProducedType() {
        return TypeInformation.of(SwapEvent.class);
    }
}
