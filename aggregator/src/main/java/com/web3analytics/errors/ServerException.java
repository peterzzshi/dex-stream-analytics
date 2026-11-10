package com.web3analytics.errors;

/**
 * The service failed to process a request it accepted (5xx).
 *
 * <p>Transient by assumption: the payload was well-formed enough to reach the handler, so the
 * fault is on the receiving side and a resend may succeed.
 */
public final class ServerException extends SinkException {

  public ServerException(final String endpoint, final int status, final String body) {
    super(endpoint, status,
        String.format("HTTP %d from %s: %s", status, endpoint, body));
  }
}
