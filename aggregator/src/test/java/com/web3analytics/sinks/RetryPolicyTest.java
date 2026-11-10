package com.web3analytics.sinks;

import static org.assertj.core.api.Assertions.assertThat;

import com.web3analytics.errors.RateLimitException;
import com.web3analytics.errors.ServerException;
import org.junit.jupiter.api.Test;

class RetryPolicyTest {

  private static final String ENDPOINT = "http://localhost:8080/api";

  @Test
  void rateLimitBacksOffFurtherThanTheSameAttemptCountOnAServerError() {
    final var rateLimited = new RateLimitException(ENDPOINT, 429, "");
    final var server = new ServerException(ENDPOINT, 500, "");

    // Jittered, so compare over several draws rather than one.
    long rateTotal = 0;
    long serverTotal = 0;
    for (int i = 0; i < 100; i++) {
      rateTotal += RetryPolicy.delayMillis(1, rateLimited);
      serverTotal += RetryPolicy.delayMillis(1, server);
    }
    assertThat(rateTotal).isGreaterThan(serverTotal);
  }

  @Test
  void backoffDoublesPerAttemptAndStaysUnderTheCap() {
    final var server = new ServerException(ENDPOINT, 500, "");

    // 100ms base with jitter in [0.75, 1.25].
    assertThat(RetryPolicy.delayMillis(1, server)).isBetween(75L, 125L);
    assertThat(RetryPolicy.delayMillis(2, server)).isBetween(150L, 250L);
    assertThat(RetryPolicy.delayMillis(3, server)).isBetween(300L, 500L);
    assertThat(RetryPolicy.delayMillis(4, server)).isBetween(600L, 1000L);

    // Beyond the cap the nominal delay is clamped, so the jittered value cannot exceed 12.5s.
    for (int attempt = 5; attempt <= 20; attempt++) {
      assertThat(RetryPolicy.delayMillis(attempt, server)).isLessThanOrEqualTo(12_500L);
    }
  }
}
