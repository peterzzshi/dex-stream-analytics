package com.web3analytics.errors;

/**
 * Category-based deserialization exception.
 * Use pattern matching on category to decide retry/crash/continue strategy.
 */
public final class DeserializationException extends RuntimeException {

    public DeserializationException(String message, Throwable cause) {
        super(message, cause);
    }

    public DeserializationException(String message) {
        super(message);
    }
}
