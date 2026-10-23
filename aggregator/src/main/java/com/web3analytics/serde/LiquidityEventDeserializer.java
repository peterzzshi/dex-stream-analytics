package com.web3analytics.serde;

import com.web3analytics.errors.DeserializationException;
import com.web3analytics.models.DexEvent;
import io.cloudevents.CloudEvent;
import io.cloudevents.CloudEventData;
import io.cloudevents.core.format.EventFormat;
import io.cloudevents.core.provider.EventFormatProvider;
import io.cloudevents.jackson.JsonFormat;
import java.io.IOException;
import java.io.Serial;
import org.apache.flink.api.common.serialization.DeserializationSchema;
import org.apache.flink.api.common.typeinfo.TypeInformation;
import org.apache.flink.util.Collector;

/**
 * Routes heterogeneous liquidity events by CloudEvent type, delegating to specialized deserializers.
 */
public class LiquidityEventDeserializer implements DeserializationSchema<DexEvent> {

    private static final String EVENT_TYPE_MINT = "com.dex.events.mint";
    private static final String EVENT_TYPE_BURN = "com.dex.events.burn";
    private static final String EVENT_TYPE_TRANSFER = "com.dex.events.transfer";

  private static final EventFormat EVENT_FORMAT =
      EventFormatProvider.getInstance().resolveFormat(JsonFormat.CONTENT_TYPE);
  @Serial
  private static final long serialVersionUID = -1192114782359144714L;

  private final transient MintEventDeserializer mintDeserializer = new MintEventDeserializer();
  private final transient BurnEventDeserializer burnDeserializer = new BurnEventDeserializer();
  private final transient TransferEventDeserializer transferDeserializer = new TransferEventDeserializer();

    @Override
    public void deserialize(final byte[] message, final Collector<DexEvent> out)
        throws IOException {
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
      if (eventType == null) {
        throw new IOException("CloudEvent missing type field");
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
        final DexEvent result = switch (eventType) {
          case EVENT_TYPE_MINT -> this.mintDeserializer.deserializeAvro(avroPayload);
          case EVENT_TYPE_BURN -> this.burnDeserializer.deserializeAvro(avroPayload);
          case EVENT_TYPE_TRANSFER -> this.transferDeserializer.deserializeAvro(avroPayload);
          default -> throw new DeserializationException("Unknown event type: " + eventType);
        };
        if (result != null) {
          out.collect(result);
        }
      } catch (final Exception e) {
        throw new IOException(
            "Failed to deserialize Avro payload for event type '" + eventType + "': "
                + e.getMessage(), e);
      }
    }

    @Override
    public DexEvent deserialize(final byte[] message) throws IOException {
      return null;
    }

  @Override
  public boolean isEndOfStream(final DexEvent nextElement) {
      return false;
    }

    @Override
    public TypeInformation<DexEvent> getProducedType() {
      return TypeInformation.of(DexEvent.class);
    }
}

