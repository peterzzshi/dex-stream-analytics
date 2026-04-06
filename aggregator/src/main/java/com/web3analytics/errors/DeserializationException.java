package com.web3analytics.errors;

/**
 * Category-based deserialization exception.
 * Use pattern matching on category to decide retry/crash/continue strategy.
 */
public final class DeserializationException extends RuntimeException {
    private final DeserializationErrorCategory category;

    public DeserializationException(String message, Throwable cause) {
        this(DeserializationErrorCategory.UNKNOWN, message, cause);
    }

    public DeserializationException(String message) {
        this(DeserializationErrorCategory.UNKNOWN, message);
    }

    public DeserializationException(DeserializationErrorCategory category, String message, Throwable cause) {
        super(message, cause);
        this.category = category;
    }

    public DeserializationException(DeserializationErrorCategory category, String message) {
        super(message);
        this.category = category;
    }

    public DeserializationErrorCategory category() {
        return category;
    }
}
