package com.web3analytics.serde;

import com.web3analytics.bridge.BridgeLockEvent;
import io.cloudevents.CloudEvent;
import io.cloudevents.core.format.EventFormat;
import io.cloudevents.core.provider.EventFormatProvider;
import io.cloudevents.jackson.JsonFormat;
import java.io.IOException;
import java.io.Serial;
import org.apache.avro.io.BinaryDecoder;
import org.apache.avro.io.DecoderFactory;
import org.apache.avro.specific.SpecificDatumReader;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class BridgeLockEventDeserializer implements DeserializationSchema<BridgeLockEvent> {

  private static final Logger LOG = LoggerFactory.getLogger(BridgeLockEventDeserializer.class);
  private static final EventFormat JSON_FORMAT = EventFormatProvider.getInstance()
      .resolveFormat(JsonFormat.CONTENT_TYPE);
  @Serial
  private static final long serialVersionUID = 160395131595603295L;

  private transient SpecificDatumReader<BridgeLockEvent> reader;
  private transient BinaryDecoder decoder;

  @Override
  public void open(final InitializationContext context) {
    this.reader = new SpecificDatumReader<>(BridgeLockEvent.getClassSchema());
  }

  @Override
  public BridgeLockEvent deserialize(final byte[] message) throws IOException {
    if (message == null || message.length == 0) {
      return null;
    }

    try {
      final CloudEvent cloudEvent = JSON_FORMAT.deserialize(message);
      final byte[] avroPayload = cloudEvent.getData().toBytes();

      this.decoder = DecoderFactory.get().binaryDecoder(avroPayload, this.decoder);
      return this.reader.read(null, this.decoder);
    } catch (final Exception e) {
      LOG.error("Failed to deserialize BridgeLockEvent", e);
      return null;
    }
  }

  @Override
  public boolean isEndOfStream(final BridgeLockEvent nextElement) {
    return false;
  }

  @Override
  public TypeInformation<BridgeLockEvent> getProducedType() {
    return TypeInformation.of(BridgeLockEvent.class);
  }
}
