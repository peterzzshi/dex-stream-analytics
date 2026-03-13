package com.web3analytics.functions;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.LiquidityAnalytics;
import com.web3analytics.models.MintEvent;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.assertj.core.api.Assertions.assertThat;

class LiquidityWindowFunctionTest {

    @Test
    void summarize_computesLiquidityFlowAndProviderMetrics() {
        List<DexEvent> events = List.of(
                new MintEvent(
                        "mint-1", 1L, 1000L, "0xtx1", 0,
                        "0xpair", "0xt0", "0xt1", "WMATIC", "USDC",
                        "0xAlice", "100", "200", 1000L
                ),
                new MintEvent(
                        "mint-2", 1L, 1050L, "0xtx2", 1,
                        "0xpair", "0xt0", "0xt1", null, null,
                        "0xBob", "50", "100", 1050L
                ),
                new BurnEvent(
                        "burn-1", 1L, 1100L, "0xtx3", 2,
                        "0xpair", "0xt0", "0xt1", "WMATIC", "USDC",
                        "0xAlice", "0xAlice", "30", "60", 1100L
                ),
                new BurnEvent(
                        "burn-2", 1L, 1200L, "0xtx4", 3,
                        "0xpair", "0xt0", "0xt1", null, null,
                        "0xCarol", "0xCarol", "20", "40", 1200L
                )
        );

        LiquidityAnalytics analytics = LiquidityWindowFunction.summarize(
                "0xpair",
                1_700_000_000_000L,
                1_700_000_360_000L,
                events,
                1_700_000_400_000L
        );

        assertThat(analytics.windowId()).isEqualTo("0xpair:1700000000000:1700000360000");
        assertThat(analytics.token0Symbol()).isEqualTo("WMATIC");
        assertThat(analytics.token1Symbol()).isEqualTo("USDC");

        assertThat(analytics.mintCount()).isEqualTo(2);
        assertThat(analytics.burnCount()).isEqualTo(2);

        assertThat(analytics.totalMintAmount0()).isEqualTo("150");
        assertThat(analytics.totalMintAmount1()).isEqualTo("300");
        assertThat(analytics.totalBurnAmount0()).isEqualTo("50");
        assertThat(analytics.totalBurnAmount1()).isEqualTo("100");

        assertThat(analytics.netLiquidityChange0()).isEqualTo("100");
        assertThat(analytics.netLiquidityChange1()).isEqualTo("200");

        assertThat(analytics.uniqueMintProviders()).isEqualTo(2);
        assertThat(analytics.uniqueBurnProviders()).isEqualTo(2);
        assertThat(analytics.uniqueProviders()).isEqualTo(3);
        assertThat(analytics.churnedProviders()).isEqualTo(1);
        assertThat(analytics.processedAt()).isEqualTo(1_700_000_400_000L);
    }

    @Test
    void buildWindowId_includesPairAndBounds() {
        assertThat(LiquidityWindowFunction.buildWindowId("0xpair", 10L, 20L))
                .isEqualTo("0xpair:10:20");
    }
}
