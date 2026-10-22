package com.web3analytics.functions.mev;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;

import java.util.List;

/**
 * Extracts session metadata from events.
 */
public final class MetadataExtractor {

  private MetadataExtractor() {
    throw new UnsupportedOperationException("Utility class");
  }

  public static SessionContext buildContext(
      String pairAddress,
      long windowStart,
      long windowEnd,
      List<DexEvent> events,
      long detectedAt
  ) {
    String chain = null;
    String chainName = null;
    String token0Symbol = null;
    String token1Symbol = null;

    for (DexEvent e : events) {
      if (chain == null) {
        chain = e.chain();
        chainName = e.chainName();
      }

      TokenPair tokens = switch (e) {
        case SwapEvent s -> new TokenPair(s.token0Symbol(), s.token1Symbol());
        case MintEvent m -> new TokenPair(m.token0Symbol(), m.token1Symbol());
        case BurnEvent b -> new TokenPair(b.token0Symbol(), b.token1Symbol());
        default -> null;
      };

      if (tokens != null) {
        if (token0Symbol == null) {
          token0Symbol = tokens.token0();
        }
        if (token1Symbol == null) {
          token1Symbol = tokens.token1();
        }
      }

      if (token0Symbol != null && token1Symbol != null) {
        break;
      }
    }

    return new SessionContext(
        pairAddress,
        windowStart,
        windowEnd,
        chain,
        chainName,
        token0Symbol,
        token1Symbol,
        detectedAt
    );
  }

  private record TokenPair(String token0, String token1) {

  }
}
