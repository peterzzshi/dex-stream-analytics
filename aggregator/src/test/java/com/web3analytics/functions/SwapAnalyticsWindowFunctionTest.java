package com.web3analytics.functions;

import static org.assertj.core.api.Assertions.assertThat;

import org.junit.jupiter.api.Test;

class SwapAnalyticsWindowFunctionTest {

    @Test
    void buildWindowId_includesPairAndWindowBounds() {
      String windowId = WindowUtils.buildWindowId("0xpair", 1700000000000L, 1700000300000L);

        assertThat(windowId).isEqualTo("0xpair:1700000000000:1700000300000");
    }
}
