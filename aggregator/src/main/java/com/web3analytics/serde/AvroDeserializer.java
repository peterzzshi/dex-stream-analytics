package com.web3analytics.serde;

import com.web3analytics.errors.DeserializationException;
import java.io.IOException;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.avro.specific.SpecificDatumReader;
import org.apache.avro.specific.SpecificRecordBase;

/**
 * Base class for Avro binary deserialization without CloudEvent handling. Subclasses provide the
 * mapping from Avro record to domain model.
 */
abstract class AvroDeserializer<T, A extends SpecificRecordBase> {

  private final Class<A> avroClass;
  private SpecificDatumReader<A> reader;
  private BinaryDecoder decoder;

  protected AvroDeserializer(Class<A> avroClass) {
    this.avroClass = avroClass;
  }

  /**
   * Deserialize Avro binary payload directly to domain model. No CloudEvent parsing or type
   * validation.
   */
  public T deserialize(byte[] avroPayload) {
    try {
      if (reader == null) {
        reader = new SpecificDatumReader<>(avroClass);
      }
      decoder = DecoderFactory.get().binaryDecoder(avroPayload, decoder);
      A avroRecord = reader.read(null, decoder);
      return map(avroRecord);
    } catch (IOException err) {
      throw new DeserializationException(
          "Failed to decode Avro payload for " + avroClass.getSimpleName() + ": "
              + err.getMessage(), err);
    }
  }

  protected abstract T map(A avroRecord);
}
