package com.web3analytics.serde;

import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;

/**
 * Flink deserialization schema that passes through raw byte arrays from Kafka.
 */
public class ByteArrayDeserializationSchema implements DeserializationSchema<byte[]> {

  @Override
  public byte[] deserialize(byte[] message) {
    return message;
  }

  @Override
  public boolean isEndOfStream(byte[] nextElement) {
    return false;
  }

  @Override
  public TypeInformation<byte[]> getProducedType() {
    return TypeInformation.of(byte[].class);
  }
}
