package com.web3analytics.serde;

import static org.assertj.core.api.Assertions.assertThat;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.models.SwapEvent;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import org.apache.flink.util.Collector;
import org.junit.jupiter.api.Test;

class SwapEventDeserializerTest {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

  private static final byte[] VALID_SWAP_AVRO_PAYLOAD = Base64.getDecoder().decode(
      "DHR4LTE6MPYBgMSf1QwIMHh0eAAQZWlwMTU1OjEQRXRoZXJldW0MMHhwYWlyEDB4dG9rZW4wEDB4dG9rZW4xAgxXTUFUSUMCCFVTREMQMHhzZW5kZXIWMHhyZWNpcGllbnQGMTAwBjIwMAYzMDAGNDAwAAAAAAAABEACH4XrUbgeCUCQyAIWNTAwMDAwMDAwMDCCxJ/VDA=="
  );

    @Test
    void shouldRejectRawAvroPayload() throws Exception {
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();
      final List<SwapEvent> results = new ArrayList<>();

      deserializer.deserialize(VALID_SWAP_AVRO_PAYLOAD, new Collector<>() {
        @Override
        public void collect(final SwapEvent swapEvent) {
          results.add(swapEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).isEmpty();
    }

    @Test
    void shouldDeserializeCloudEventWithDataBase64() throws Exception {
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();
      final List<SwapEvent> results = new ArrayList<>();

      final ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dex.events.swap");
        envelope.put("source", "ingester");
        envelope.put("id", "evt-1");
        envelope.put("datacontenttype", "application/avro-binary");
      envelope.put("data_base64", Base64.getEncoder().encodeToString(VALID_SWAP_AVRO_PAYLOAD));

      final byte[] cloudEvent = OBJECT_MAPPER.writeValueAsString(envelope)
          .getBytes(StandardCharsets.UTF_8);

      deserializer.deserialize(cloudEvent, new Collector<>() {
        @Override
        public void collect(final SwapEvent swapEvent) {
          results.add(swapEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).hasSize(1);
      final SwapEvent event = results.getFirst();
      assertThat(event).isNotNull();
      assertThat(event.eventId()).isEqualTo("tx-1:0");
      assertThat(event.pairAddress()).isEqualTo("0xpair");
      assertThat(event.sender()).isEqualTo("0xsender");
      assertThat(event.amount0In()).isEqualTo("100");
      assertThat(event.amount1In()).isEqualTo("200");
    }

    @Test
    void shouldRejectMismatchedEventType() throws Exception {
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();
      final List<SwapEvent> results = new ArrayList<>();

      final ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dex.events.burn");
        envelope.put("source", "ingester");
        envelope.put("id", "evt-1");
      envelope.put("data_base64", Base64.getEncoder().encodeToString(VALID_SWAP_AVRO_PAYLOAD));

      final byte[] cloudEvent = OBJECT_MAPPER.writeValueAsString(envelope)
          .getBytes(StandardCharsets.UTF_8);

      deserializer.deserialize(cloudEvent, new Collector<>() {
        @Override
        public void collect(final SwapEvent swapEvent) {
          results.add(swapEvent);
        }

        @Override
        public void close() {
        }
      });

      assertThat(results).isEmpty();
    }
}
