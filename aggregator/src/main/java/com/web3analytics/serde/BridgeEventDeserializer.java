package com.web3analytics.serde;

import io.cloudevents.CloudEvent;
import io.cloudevents.core.format.EventFormat;
import io.cloudevents.core.provider.EventFormatProvider;
import io.cloudevents.jackson.JsonFormat;
import io.cloudevents.rw.CloudEventRWException;
import java.io.IOException;
import java.io.Serial;
import org.apache.avro.AvroTypeException;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.avro.specific.SpecificData;
import org.apache.avro.specific.SpecificDatumReader;
import org.apache.avro.specific.SpecificRecord;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.apache.flink.metrics.Counter;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Turns a CloudEvent-wrapped Avro payload from Kafka into a generated bridge event record. One
 * instance per event type; the schema is taken from the record class itself, so the decoder cannot
 * drift from the type it is declared to produce.
 */
public class BridgeEventDeserializer<T extends SpecificRecord>
    implements DeserializationSchema<T> {

  private static final Logger LOG = LoggerFactory.getLogger(BridgeEventDeserializer.class);
  private static final EventFormat JSON_FORMAT =
      EventFormatProvider.getInstance().resolveFormat(JsonFormat.CONTENT_TYPE);

  @Serial
  private static final long serialVersionUID = 1L;

  private final Class<T> eventClass;
  private transient SpecificDatumReader<T> reader;
  private transient BinaryDecoder decoder;
  private transient Counter malformedRecords;

  public BridgeEventDeserializer(final Class<T> eventClass) {
    this.eventClass = eventClass;
  }

  @Override
  public void open(final InitializationContext context) {
    this.reader = new SpecificDatumReader<>(SpecificData.get().getSchema(this.eventClass));
    this.malformedRecords = context.getMetricGroup().counter("malformedRecords");
  }

  @Override
  public T deserialize(final byte[] message) {
    if (message == null || message.length == 0) {
      return null;
    }
    try {
      final CloudEvent cloudEvent = JSON_FORMAT.deserialize(message);
      if (cloudEvent.getData() == null) {
        throw new MalformedRecordException("CloudEvent carries no data payload");
      }
      final byte[] avroPayload = cloudEvent.getData().toBytes();
      this.decoder = DecoderFactory.get().binaryDecoder(avroPayload, this.decoder);
      return this.reader.read(null, this.decoder);
    } catch (final MalformedRecordException | CloudEventRWException | AvroTypeException
                   | IOException e) {
      // Bad bytes are a per-record problem: dropping keeps the job running, and the counter
      // keeps the drop visible instead of indistinguishable from a clean run.
      this.malformedRecords.inc();
      LOG.error("Dropping malformed {}", this.eventClass.getSimpleName(), e);
      return null;
    }
    // Any other exception is a defect in this pipeline (a null reader, an unexpected type) and
    // must fail the job rather than be silently counted as bad input.
  }

  @Override
  public boolean isEndOfStream(final T nextElement) {
    return false;
  }

  @Override
  public TypeInformation<T> getProducedType() {
    return TypeInformation.of(this.eventClass);
  }

  private static final class MalformedRecordException extends RuntimeException {

    @Serial
    private static final long serialVersionUID = 1L;

    MalformedRecordException(final String message) {
      super(message);
    }
  }
}
