package com.web3analytics.serialization;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.web3analytics.errors.DeserializationException;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Base64;

final class CloudEventPayloadExtractor {

    private CloudEventPayloadExtractor() {
    }

    static byte[] extract(byte[] message, ObjectMapper objectMapper) throws IOException {
        if (message == null || message.length == 0) {
            return message;
        }

        JsonNode root;
        try {
            root = objectMapper.readTree(message);
        } catch (IOException err) {
            throw new DeserializationException("Expected CloudEvent JSON envelope", err);
        }

        if (root == null || !root.hasNonNull("specversion")) {
            throw new DeserializationException("Invalid CloudEvent: missing specversion field");
        }

        if (root.hasNonNull("data_base64")) {
            return Base64.getDecoder().decode(root.get("data_base64").asText());
        }

        JsonNode dataNode = root.get("data");
        if (dataNode == null || dataNode.isNull()) {
            return new byte[0];
        }

        if (dataNode.isBinary()) {
            return dataNode.binaryValue();
        }

        if (dataNode.isTextual()) {
            String dataText = dataNode.asText();
            if (looksLikeBase64(dataText)) {
                try {
                    return Base64.getDecoder().decode(dataText);
                } catch (IllegalArgumentException ignored) {
                    // Fall through to ISO-8859-1 fallback.
                }
            }
            // Dapr may serialize binary payload as escaped text in `data`.
            return dataText.getBytes(StandardCharsets.ISO_8859_1);
        }

        if (dataNode.isObject() || dataNode.isArray()) {
            return objectMapper.writeValueAsBytes(dataNode);
        }

        throw new DeserializationException("Unsupported CloudEvent data type: " + dataNode.getNodeType());
    }

    private static boolean looksLikeBase64(String value) {
        if (value == null || value.length() < 16 || value.length() % 4 != 0) {
            return false;
        }

        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            boolean allowed = (c >= 'A' && c <= 'Z')
                    || (c >= 'a' && c <= 'z')
                    || (c >= '0' && c <= '9')
                    || c == '+'
                    || c == '/'
                    || c == '=';
            if (!allowed) {
                return false;
            }
        }
        return true;
    }
}
