package com.web3analytics.models;

import java.time.Instant;
import java.util.HexFormat;

/**
 * Captures decoder failures without failing the Flink job.
 */
public record DecodingError(
        String stream,
        String reason,
        int payloadSize,
        String payloadPreview,
        long failedAt
) {
    private static final int PREVIEW_BYTES = 16;

    public static DecodingError from(String stream, byte[] payload, Throwable error) {
        int payloadSize = payload == null ? 0 : payload.length;
        String preview = payloadSize == 0
                ? ""
                : HexFormat.of().formatHex(payload, 0, Math.min(PREVIEW_BYTES, payloadSize));

        String reason = error == null
                ? "unknown decoding error"
                : error.getMessage();

        return new DecodingError(
                stream,
                reason,
                payloadSize,
                preview,
                Instant.now().toEpochMilli()
        );
    }
}
