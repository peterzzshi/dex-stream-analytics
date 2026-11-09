package com.web3analytics.serde;

import com.web3analytics.bridge.BridgeMintEvent;
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

public class BridgeMintEventDeserializer implements DeserializationSchema<BridgeMintEvent> {

  private static final Logger LOG = LoggerFactory.getLogger(BridgeMintEventDeserializer.class);
  private static final EventFormat JSON_FORMAT = EventFormatProvider.getInstance()
      .resolveFormat(JsonFormat.CONTENT_TYPE);
  @Serial
  private static final long serialVersionUID = 3470397823825778002L;

  private transient SpecificDatumReader<BridgeMintEvent> reader;
  private transient BinaryDecoder decoder;

  @Override
  public void open(final InitializationContext context) {
    this.reader = new SpecificDatumReader<>(BridgeMintEvent.getClassSchema());
  }

  @Override
  public BridgeMintEvent deserialize(final byte[] message) throws IOException {
    if (message == null || message.length == 0) {
      return null;
    }

    try {
      final CloudEvent cloudEvent = JSON_FORMAT.deserialize(message);
      final byte[] avroPayload = cloudEvent.getData().toBytes();

      this.decoder = DecoderFactory.get().binaryDecoder(avroPayload, this.decoder);
      return this.reader.read(null, this.decoder);
    } catch (final Exception e) {
      LOG.error("Failed to deserialize BridgeMintEvent", e);
      return null;
    }
  }

  @Override
  public boolean isEndOfStream(final BridgeMintEvent nextElement) {
    return false;
  }

  @Override
  public TypeInformation<BridgeMintEvent> getProducedType() {
    return TypeInformation.of(BridgeMintEvent.class);
  }
}
