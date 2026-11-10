package com.web3analytics.errors;

/**
 * The endpoint read the payload and refused it (4xx other than 408 and 429).
 *
 * <p>Not retriable: the same bytes will be refused again, so resending cannot succeed and failing
 * the task would only loop.
 */
public final class RejectedException extends SinkException {

  public RejectedException(final String endpoint, final int status, final String body) {
    super(endpoint, status, describe(endpoint, status, body));
  }

  private static String describe(final String endpoint, final int status, final String body) {
    return String.format("HTTP %d from %s: %s", status, endpoint, body);
  }
}
