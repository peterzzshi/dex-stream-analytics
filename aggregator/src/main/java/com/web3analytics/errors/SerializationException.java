package com.web3analytics.errors;

import java.io.Serial;

/**
 * Thrown when serializing a domain model to bytes fails. Not intended for serialization itself -
 * used only for control flow in Flink pipelines.
 */
public final class SerializationException extends RuntimeException {

  // Suppresses Serializable warning - this exception is not serialized
  @Serial
  private static final long serialVersionUID = 1L;

  public SerializationException(String message, Throwable cause) {
    super(message, cause);
    }
}
