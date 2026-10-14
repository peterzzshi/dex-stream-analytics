package com.web3analytics.models;

import java.time.Instant;
import java.util.HexFormat;

public record DecodingError(
        String stream,
        String reason,
        int payloadSize,
        String payloadPreview,
        long failedAt
) {
    private static final int PREVIEW_BYTES = 16;

    public static DecodingError from(String stream, byte[] payload, Throwable error) {
        int size = payload == null ? 0 : payload.length;
        String preview = size == 0
                ? ""
                : HexFormat.of().formatHex(payload, 0, Math.min(PREVIEW_BYTES, size));
        String reason = error == null ? "unknown" : error.getMessage();
        return new DecodingError(stream, reason, size, preview, Instant.now().toEpochMilli());
    }
}
