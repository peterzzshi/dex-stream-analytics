package com.web3analytics.serialization;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.web3analytics.errors.DeserializationException;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.Base64;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

class CloudEventPayloadExtractorTest {

    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    @Test
    void shouldRejectRawAvroBytes() {
        byte[] raw = new byte[]{0x01, 0x02, 0x03, 0x04};

        assertThatThrownBy(() -> CloudEventPayloadExtractor.extract(raw, OBJECT_MAPPER))
                .isInstanceOf(DeserializationException.class)
                .hasMessageContaining("Expected CloudEvent JSON envelope");
    }

    @Test
    void shouldExtractDataBase64Field() throws Exception {
        byte[] raw = new byte[]{0x08, 0x10, 0x20, 0x40};
        ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dapr.event.sent");
        envelope.put("source", "ingester");
        envelope.put("id", "test-id");
        envelope.put("data_base64", Base64.getEncoder().encodeToString(raw));

        byte[] extracted = CloudEventPayloadExtractor.extract(
                OBJECT_MAPPER.writeValueAsString(envelope).getBytes(StandardCharsets.UTF_8),
                OBJECT_MAPPER
        );

        assertThat(extracted).isEqualTo(raw);
    }

    @Test
    void shouldExtractEscapedTextDataField() throws Exception {
        byte[] raw = new byte[]{0x08, 0x73, 0x77, 0x61, 0x70};
        String dataText = new String(raw, StandardCharsets.ISO_8859_1);
        ObjectNode envelope = OBJECT_MAPPER.createObjectNode();
        envelope.put("specversion", "1.0");
        envelope.put("type", "com.dapr.event.sent");
        envelope.put("source", "ingester");
        envelope.put("id", "test-id");
        envelope.put("data", dataText);

        byte[] extracted = CloudEventPayloadExtractor.extract(
                OBJECT_MAPPER.writeValueAsString(envelope).getBytes(StandardCharsets.UTF_8),
                OBJECT_MAPPER
        );

        assertThat(extracted).isEqualTo(raw);
    }
}
