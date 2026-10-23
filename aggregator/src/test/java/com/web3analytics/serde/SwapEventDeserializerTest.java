package com.web3analytics.serde;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.errors.DeserializationException;
import com.web3analytics.models.SwapEvent;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.junit.jupiter.api.Test;

class SwapEventDeserializerTest {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

  private static final byte[] VALID_SWAP_AVRO_PAYLOAD = Base64.getDecoder().decode(
      "DHR4LTE6MPYBgMSf1QwIMHh0eAAQZWlwMTU1OjEQRXRoZXJldW0MMHhwYWlyEDB4dG9rZW4wEDB4dG9rZW4xAgxXTUFUSUMCCFVTREMQMHhzZW5kZXIWMHhyZWNpcGllbnQGMTAwBjIwMAYzMDAGNDAwAAAAAAAABEACH4XrUbgeCUCQyAIWNTAwMDAwMDAwMDCCxJ/VDA=="
  );

  private static com.web3analytics.events.SwapEvent validAvroSwapEvent() {
    return new com.web3analytics.events.SwapEvent(
        "tx-1:0", 123L, 1700000000L, "0xtx", 0, "eip155:1", "Ethereum", "0xpair",
        "0xtoken0", "0xtoken1", "WMATIC", "USDC", "0xsender", "0xrecipient",
        "100", "200", "0", "0", 1.5, 500.0, 100000L, "20", 1700000001L);
  }

    @Test
    void shouldRejectRawAvroPayload() {
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();
      final ListCollector<SwapEvent> collector = new ListCollector<>();

      assertThatThrownBy(() -> deserializer.deserialize(VALID_SWAP_AVRO_PAYLOAD, collector))
          .isInstanceOf(IOException.class);

      assertThat(collector.items()).isEmpty();
    }

    @Test
    void shouldDeserializeCloudEventWithDataBase64() throws Exception {
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();
      final ListCollector<SwapEvent> collector = new ListCollector<>();

      final ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dex.events.swap");
        envelope.put("source", "ingester");
        envelope.put("id", "evt-1");
        envelope.put("datacontenttype", "application/avro-binary");
      envelope.put("data_base64", Base64.getEncoder().encodeToString(VALID_SWAP_AVRO_PAYLOAD));

      final byte[] cloudEvent = OBJECT_MAPPER.writeValueAsString(envelope)
          .getBytes(StandardCharsets.UTF_8);

      deserializer.deserialize(cloudEvent, collector);

      assertThat(collector.items()).hasSize(1);
      final SwapEvent event = collector.items().getFirst();
      assertThat(event).isNotNull();
      assertThat(event.eventId()).isEqualTo("tx-1:0");
      assertThat(event.pairAddress()).isEqualTo("0xpair");
      assertThat(event.sender()).isEqualTo("0xsender");
      assertThat(event.amount0In()).isEqualTo("100");
      assertThat(event.amount1In()).isEqualTo("200");
    }

  // -- map(): field mapping and amount validation, no CloudEvent/Avro-byte ceremony needed --

    @Test
    void shouldRejectMismatchedEventType() throws Exception {
      final SwapEventDeserializer deserializer = new SwapEventDeserializer();
      final ListCollector<SwapEvent> collector = new ListCollector<>();

      final ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dex.events.burn");
        envelope.put("source", "ingester");
        envelope.put("id", "evt-1");
      envelope.put("data_base64", Base64.getEncoder().encodeToString(VALID_SWAP_AVRO_PAYLOAD));

      final byte[] cloudEvent = OBJECT_MAPPER.writeValueAsString(envelope)
          .getBytes(StandardCharsets.UTF_8);

      assertThatThrownBy(() -> deserializer.deserialize(cloudEvent, collector))
          .isInstanceOf(IOException.class).hasMessageContaining("type mismatch");

      assertThat(collector.items()).isEmpty();
    }

  @Test
  void mapCopiesFieldsFromAvroRecord() {
    final SwapEventDeserializer deserializer = new SwapEventDeserializer();

    final SwapEvent event = deserializer.map(validAvroSwapEvent());

    assertThat(event.eventId()).isEqualTo("tx-1:0");
    assertThat(event.pairAddress()).isEqualTo("0xpair");
    assertThat(event.sender()).isEqualTo("0xsender");
    assertThat(event.amount0In()).isEqualTo("100");
    assertThat(event.amount1In()).isEqualTo("200");
  }

  @Test
  void mapRejectsNegativeAmount() {
    final SwapEventDeserializer deserializer = new SwapEventDeserializer();
    final com.web3analytics.events.SwapEvent avroRecord = validAvroSwapEvent();
    avroRecord.setAmount0In("-5");

    assertThatThrownBy(() -> deserializer.map(avroRecord))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("amount0In")
        .hasMessageContaining("negative");
  }

  @Test
  void mapRejectsAllZeroAmounts() {
    final SwapEventDeserializer deserializer = new SwapEventDeserializer();
    final com.web3analytics.events.SwapEvent avroRecord = validAvroSwapEvent();
    avroRecord.setAmount0In("0");
    avroRecord.setAmount1In("0");

    assertThatThrownBy(() -> deserializer.map(avroRecord))
        .isInstanceOf(DeserializationException.class)
        .hasMessageContaining("SwapEvent")
        .hasMessageContaining("no non-zero amount field");
  }
}
