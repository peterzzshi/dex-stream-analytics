package com.web3analytics.errors;

public final class SerializationException extends RuntimeException {

    private final Object failedObject;

    public SerializationException(Object failedObject, Throwable cause) {
        super("Failed to serialize: " +
              (failedObject != null ? failedObject.getClass().getName() : "null"),
              cause);
        this.failedObject = failedObject;
    }

    public Object failedObject() {
        return failedObject;
    }
}
