package com.web3analytics.serde;

import io.cloudevents.CloudEvent;
import io.cloudevents.CloudEventData;
import io.cloudevents.core.format.EventFormat;
import io.cloudevents.core.provider.EventFormatProvider;
import io.cloudevents.jackson.JsonFormat;
import java.io.IOException;
import java.io.Serial;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.avro.specific.SpecificDatumReader;
import org.apache.avro.specific.SpecificRecord;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.apache.flink.util.Collector;

/**
 * Template for CloudEvent envelope unwrap and Avro binary decode.
 * Subclasses provide only the Avro-to-model mapping.
 */
public abstract class AbstractCloudEventAvroDeserializer<T, S extends SpecificRecord> implements DeserializationSchema<T> {

  private static final EventFormat EVENT_FORMAT =
      EventFormatProvider.getInstance().resolveFormat(JsonFormat.CONTENT_TYPE);
  @Serial
  private static final long serialVersionUID = 5597819939986432037L;

  private final Class<S> recordClass;
  private final TypeInformation<T> producedType;
  private final String expectedEventType;

    private transient SpecificDatumReader<S> reader;
    private transient BinaryDecoder decoder;

  protected AbstractCloudEventAvroDeserializer(final Class<S> recordClass,
      final Class<T> producedClass, final String expectedEventType) {
    this.recordClass = recordClass;
    this.producedType = TypeInformation.of(producedClass);
        this.expectedEventType = expectedEventType;
    }

    @Override
    public void deserialize(final byte[] message, final Collector<T> out) throws IOException {
      if (message == null || message.length == 0) {
        return;
        }

      final CloudEvent cloudEvent;
      try {
        cloudEvent = EVENT_FORMAT.deserialize(message);
      } catch (final Exception e) {
        throw new IOException("Failed to parse CloudEvent: " + e.getMessage(), e);
      }
      if (cloudEvent == null) {
        throw new IOException("CloudEvent deserialization returned null");
      }

      final String eventType = cloudEvent.getType();
      if (!this.expectedEventType.equals(eventType)) {
        throw new IOException("CloudEvent type mismatch: expected '" + this.expectedEventType
            + "' but got '" + eventType + "'");
      }

      final CloudEventData data = cloudEvent.getData();
      if (data == null) {
        throw new IOException("CloudEvent missing data field for type '" + eventType + "'");
      }

      final byte[] avroPayload = data.toBytes();
      if (avroPayload == null || avroPayload.length == 0) {
        throw new IOException("CloudEvent data is empty for type '" + eventType + "'");
      }

      try {
        if (this.reader == null) {
          this.reader = new SpecificDatumReader<>(this.recordClass);
        }
        this.decoder = DecoderFactory.get().binaryDecoder(avroPayload, this.decoder);
        final S avroRecord = this.reader.read(null, this.decoder);
        final T result = this.map(avroRecord);
        if (result != null) {
          out.collect(result);
        }
      } catch (final Exception e) {
        throw new IOException(
            "Avro decoding failed for event type '" + eventType + "': " + e.getMessage(), e);
      }
    }

  @Override
  public T deserialize(final byte[] message) throws IOException {
    return null;
  }

  protected abstract T map(S avroRecord);

  @Override
  public final boolean isEndOfStream(final T nextElement) {
    return false;
  }

  @Override
  public final TypeInformation<T> getProducedType() {
    return this.producedType;
  }
}
