package com.web3analytics.errors;

import static org.assertj.core.api.Assertions.assertThat;

import org.junit.jupiter.api.Test;

class SinkExceptionTest {

  private static final String ENDPOINT = "http://localhost:8080/api";

  @Test
  void networkFailureCarriesNoStatus() {
    final SinkException failure = new NetworkException(ENDPOINT, "connection refused");

    assertThat(failure.status()).isEqualTo(SinkException.NO_RESPONSE);
    assertThat(failure.getMessage()).contains("connection refused");
  }
}
