package com.web3analytics.serde;

import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;

/** Passes Kafka values through as raw bytes for downstream decode + side-output routing. */
public class ByteArrayPassthroughDeserializer implements DeserializationSchema<byte[]> {

    @Override
    public byte[] deserialize(byte[] message) { return message; }

    @Override
    public boolean isEndOfStream(byte[] nextElement) { return false; }

    @Override
    public TypeInformation<byte[]> getProducedType() { return TypeInformation.of(byte[].class); }
}
