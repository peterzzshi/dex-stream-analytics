package com.web3analytics.errors;

/**
 * Deserialization error categories for downstream error handling strategy.
 */
public enum DeserializationErrorCategory {
    INVALID_ENVELOPE,
    UNSUPPORTED_EVENT_TYPE,
    PAYLOAD_EXTRACTION,
    SCHEMA_LOAD,
    AVRO_DECODE,
    FIELD_MAPPING,
    UNKNOWN
}
