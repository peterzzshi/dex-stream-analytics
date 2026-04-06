package com.web3analytics.serde;

import com.web3analytics.errors.SerializationException;
import org.apache.flink.api.common.serialization.SerializationSchema;
import org.apache.flink.shaded.jackson2.com.fasterxml.jackson.databind.ObjectMapper;

public class AvroSerializationSchema<T> implements SerializationSchema<T> {
    private final ObjectMapper mapper = new ObjectMapper();

    @Override
    public byte[] serialize(T element) {
        try {
            return mapper.writeValueAsBytes(element);
        } catch (Exception e) {
            throw new SerializationException(element, e);
        }
    }
}
