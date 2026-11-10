package com.web3analytics.sinks;

import com.web3analytics.errors.NetworkException;
import com.web3analytics.errors.RateLimitException;
import com.web3analytics.errors.RejectedException;
import com.web3analytics.errors.ServerException;
import com.web3analytics.errors.SinkException;
import java.util.concurrent.ThreadLocalRandom;

/**
 * How long to wait before re-sending a failed POST.
 *
 * <p>Doubling delay with jitter, mirroring the Rust ingesters' publisher so both sides of the
 * pipeline back off on the same schedule. Jitter matters when many writers hit the same endpoint:
 * without it, every writer that failed during one outage would retry in lockstep.
 */
public final class RetryPolicy {

  static final long INITIAL_BACKOFF_MS = 100;
  static final long MAX_BACKOFF_MS = 10_000;
  private static final double JITTER_MIN = 0.75;
  private static final double JITTER_MAX = 1.25;
  private static final int RATE_LIMIT_MULTIPLIER = 4;

  private RetryPolicy() {
  }

  /** Delay after the given 1-based attempt failed. Bounded by {@link #MAX_BACKOFF_MS}. */
  public static long delayMillis(final int attempt, final SinkException failure) {
    final int shift = Math.min(attempt - 1, 16);
    final long base = Math.min(INITIAL_BACKOFF_MS << shift, MAX_BACKOFF_MS);
    // A rate limit means the problem is our request rate, so waiting as short as for a 5xx would
    // tend to fail the same way. RejectedException never reaches here — the sink throws it on the
    // first attempt — but naming it keeps the switch exhaustive without a default.
    final long scaled = switch (failure) {
      case RateLimitException ignored -> base * RATE_LIMIT_MULTIPLIER;
      case NetworkException ignored -> base;
      case ServerException ignored -> base;
      case RejectedException ignored -> base;
    };
    final long capped = Math.min(scaled, MAX_BACKOFF_MS);
    return Math.round(capped * ThreadLocalRandom.current().nextDouble(JITTER_MIN, JITTER_MAX));
  }
}
