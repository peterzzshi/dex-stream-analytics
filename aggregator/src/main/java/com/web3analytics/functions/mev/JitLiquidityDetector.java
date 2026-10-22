package com.web3analytics.functions.mev;

import static com.web3analytics.functions.Web3Math.normalizeAddress;

import com.web3analytics.models.BurnEvent;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MintEvent;
import com.web3analytics.models.SwapEvent;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * Detects JIT (Just-In-Time) liquidity: Mint → Swap → Burn within 2 blocks by same provider.
 */
public class JitLiquidityDetector implements MevPattern {

  private static final int MAX_BLOCK_DISTANCE = 2;

  @Override
  public List<MevCandidate> detect(List<DexEvent> events, SessionContext context) {
    List<MintEvent> mints = filterEvents(events, MintEvent.class);
    List<BurnEvent> burns = filterEvents(events, BurnEvent.class);
    List<SwapEvent> swaps = filterEvents(events, SwapEvent.class);

    if (mints.isEmpty() || burns.isEmpty() || swaps.isEmpty()) {
      return List.of();
    }

    List<MevCandidate> candidates = new ArrayList<>();

    for (MintEvent mint : mints) {
      for (BurnEvent burn : burns) {
        MevCandidate jit = detectJitPattern(mint, burn, swaps);
        if (jit != null) {
          candidates.add(jit);
        }
      }
    }

    return candidates;
  }

  private MevCandidate detectJitPattern(MintEvent mint, BurnEvent burn, List<SwapEvent> swaps) {
    String mintProvider = normalizeAddress(mint.sender());
    String burnProvider = normalizeAddress(burn.sender());

    if (!mintProvider.equals(burnProvider)) {
      return null;
    }

    long mintBlock = mint.blockNumber();
    long burnBlock = burn.blockNumber();

    if (Math.abs(burnBlock - mintBlock) > MAX_BLOCK_DISTANCE) {
      return null;
    }

    boolean hasSwapBetween = swaps.stream()
        .anyMatch(s -> s.blockNumber() >= mintBlock && s.blockNumber() <= burnBlock);

    if (!hasSwapBetween) {
      return null;
    }

    Set<String> txHashes = collectTransactionHashes(mint, burn, swaps, mintBlock, burnBlock);

    return new MevCandidate(
        "JIT_LIQUIDITY",
        mintBlock,
        mintProvider,
        List.of(),
        0.0,
        0,
        txHashes.size(),
        txHashes,
        String.format("JIT liquidity: %s added/removed liquidity around swap in blocks %d-%d",
            mintProvider, mintBlock, burnBlock)
    );
  }

  private Set<String> collectTransactionHashes(
      MintEvent mint,
      BurnEvent burn,
      List<SwapEvent> swaps,
      long mintBlock,
      long burnBlock
  ) {
    Set<String> txHashes = new HashSet<>();
    txHashes.add(mint.transactionHash());
    txHashes.add(burn.transactionHash());

    swaps.stream()
        .filter(s -> s.blockNumber() >= mintBlock && s.blockNumber() <= burnBlock)
        .forEach(s -> txHashes.add(s.transactionHash()));

    return txHashes;
  }

  private <T extends DexEvent> List<T> filterEvents(List<DexEvent> events, Class<T> type) {
    return events.stream()
        .filter(type::isInstance)
        .map(type::cast)
        .toList();
  }
}
