package com.web3analytics.serde;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.web3analytics.errors.DeserializationErrorCategory;
import com.web3analytics.errors.DeserializationException;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Base64;

final class CloudEventPayloadExtractor {

    private CloudEventPayloadExtractor() {
    }

    record Envelope(String type, byte[] payload) {}

    static byte[] extract(byte[] message, ObjectMapper objectMapper) throws IOException {
        return extractEnvelope(message, objectMapper).payload();
    }

    static Envelope extractEnvelope(byte[] message, ObjectMapper objectMapper) throws IOException {
        if (message == null || message.length == 0) {
            throw new DeserializationException(DeserializationErrorCategory.INVALID_ENVELOPE,
                    "CloudEvent envelope is null or empty");
        }

        JsonNode root;
        try {
            root = objectMapper.readTree(message);
        } catch (IOException err) {
            throw new DeserializationException(DeserializationErrorCategory.INVALID_ENVELOPE,
                    "Expected CloudEvent JSON envelope", err);
        }

        if (root == null || !root.hasNonNull("specversion")) {
            throw new DeserializationException(DeserializationErrorCategory.INVALID_ENVELOPE,
                    "Invalid CloudEvent: missing specversion field");
        }

        String type = root.hasNonNull("type") ? root.get("type").asText() : "";
        if (root.hasNonNull("data_base64")) {
            return new Envelope(type, Base64.getDecoder().decode(root.get("data_base64").asText()));
        }

        JsonNode dataNode = root.get("data");
        if (dataNode == null || dataNode.isNull()) {
            throw new DeserializationException(DeserializationErrorCategory.PAYLOAD_EXTRACTION,
                    "CloudEvent has no payload: data field is missing or null");
        }

        if (dataNode.isBinary()) {
            return new Envelope(type, dataNode.binaryValue());
        }

        if (dataNode.isTextual()) {
            // Dapr may rewrite data_base64 as escaped text in `data`.
            return new Envelope(type, dataNode.asText().getBytes(StandardCharsets.ISO_8859_1));
        }

        throw new DeserializationException(DeserializationErrorCategory.PAYLOAD_EXTRACTION,
                "Unsupported CloudEvent data type: " + dataNode.getNodeType()
                        + ". Expected data_base64 or textual data field");
    }
}
