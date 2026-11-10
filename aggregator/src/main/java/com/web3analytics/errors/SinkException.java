package com.web3analytics.errors;

import java.io.IOException;
import java.io.Serial;

/**
 * A POST that did not succeed, classified by what the failure says about the request.
 *
 * <p>Extends {@link IOException} because that is what {@code SinkWriter.write} declares, so every
 * variant travels out of the sink without wrapping.
 *
 * <p>The distinction that matters downstream is retriable versus not, and it is expressed by the
 * subtype: {@link RejectedException} means the payload itself was refused, every other subtype
 * means this attempt failed.
 *
 * <p>Sealed so callers can decide that by pattern-matching, with the compiler checking the match
 * is complete. The alternative — a {@code retriable()} predicate defaulting to {@code true} — hides
 * the decision: a new subtype inherits "retry", which is the wrong default to inherit silently.
 * Here a new subtype is a compile error at every site that classifies a failure.
 */
public abstract sealed class SinkException extends IOException
    permits NetworkException, ServerException, RateLimitException, RejectedException {

  /**
   * No response arrived, so there is no HTTP status to report.
   */
  public static final int NO_RESPONSE = -1;
  @Serial
  private static final long serialVersionUID = -6809000600010811693L;

  private final String endpoint;
  private final int status;

  protected SinkException(final String endpoint, final int status, final String message) {
    super(message);
    this.endpoint = endpoint;
    this.status = status;
  }

  public String endpoint() {
    return this.endpoint;
  }

  public int status() {
    return this.status;
  }
}
