package com.web3analytics.errors;

/**
 * No HTTP response arrived: the connection was refused, the DNS lookup failed, or the request
 * timed out client-side.
 *
 * <p>Carries no status because there is none. Retriable, because the request itself was never
 * judged by the service.
 */
public final class NetworkException extends SinkException {

  public NetworkException(final String endpoint, final String reason) {
    super(endpoint, NO_RESPONSE,
        String.format("No response from %s: %s", endpoint, reason));
  }
}
