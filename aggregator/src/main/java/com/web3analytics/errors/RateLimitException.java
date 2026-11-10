package com.web3analytics.errors;

/**
 * The service is alive but asked us to slow down (429), or the request timed out on its side (408).
 *
 * <p>Worth retrying, and worth waiting longer for than a server error: the problem is our request
 * rate, so an immediate resend tends to fail the same way.
 */
public final class RateLimitException extends SinkException {

  public RateLimitException(final String endpoint, final int status, final String body) {
    super(endpoint, status,
        String.format("HTTP %d (rate limited) from %s: %s", status, endpoint, body));
  }
}
