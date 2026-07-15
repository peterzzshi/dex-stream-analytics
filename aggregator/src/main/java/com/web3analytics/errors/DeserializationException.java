package com.web3analytics.errors;

/**
 * Carries a {@link DeserializationErrorCategory} so downstream handlers
 * can decide retry/skip/crash strategy per category.
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
