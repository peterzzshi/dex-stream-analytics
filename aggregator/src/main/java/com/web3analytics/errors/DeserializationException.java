package com.web3analytics.errors;

public final class DeserializationException extends RuntimeException {

    public DeserializationException(String message, Throwable cause) {
        super(message, cause);
    }

  public DeserializationException(String message) {
        super(message);
    }

}
