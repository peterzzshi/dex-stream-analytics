package com.web3analytics.functions.mev;

import static com.web3analytics.functions.Web3Math.normalizeAddress;
import static com.web3analytics.functions.mev.SwapClassifier.Direction;
import static com.web3analytics.functions.mev.SwapClassifier.classify;
import static com.web3analytics.functions.mev.SwapClassifier.estimateProfit;

import com.web3analytics.models.DexEvent;
import com.web3analytics.models.SwapEvent;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * Detects sandwich attacks: attacker front-runs and back-runs victim swaps in the same block.
 */
public class SandwichDetector implements MevPattern {

  @Override
  public List<MevCandidate> detect(List<DexEvent> events, SessionContext context) {
    List<SwapEvent> swaps = events.stream()
        .filter(SwapEvent.class::isInstance)
        .map(SwapEvent.class::cast)
        .toList();

    if (swaps.size() < 3) {
      return List.of();
    }

    List<MevCandidate> candidates = new ArrayList<>();

    for (int i = 0; i < swaps.size() - 2; i++) {
      SwapEvent frontRun = swaps.get(i);
      String attackerAddr = normalizeAddress(frontRun.sender());
      Direction frontDir = classify(frontRun);

      if (frontDir == Direction.UNKNOWN) {
        continue;
      }

      MevCandidate sandwich = findMatchingBackRun(swaps, i, frontRun, attackerAddr, frontDir);
      if (sandwich != null) {
        candidates.add(sandwich);
      }
    }

    return candidates;
  }

  private MevCandidate findMatchingBackRun(
      List<SwapEvent> swaps,
      int frontIndex,
      SwapEvent frontRun,
      String attackerAddr,
      Direction frontDir
  ) {
    for (int j = frontIndex + 2; j < swaps.size(); j++) {
      SwapEvent backRun = swaps.get(j);

      if (isSameAttacker(attackerAddr, backRun) && isOppositeDirection(frontDir, backRun)) {
        List<String> victims = extractVictims(swaps, frontIndex, j, attackerAddr);
        if (!victims.isEmpty()) {
          return buildCandidate(swaps, frontIndex, j, frontRun, backRun, attackerAddr, victims);
        }
      }
    }
    return null;
  }

  private boolean isSameAttacker(String attackerAddr, SwapEvent backRun) {
    return attackerAddr.equals(normalizeAddress(backRun.sender()));
  }

  private boolean isOppositeDirection(Direction frontDir, SwapEvent backRun) {
    Direction backDir = classify(backRun);
    return frontDir != backDir;
  }

  private MevCandidate buildCandidate(
      List<SwapEvent> swaps,
      int frontIndex,
      int backIndex,
      SwapEvent frontRun,
      SwapEvent backRun,
      String attackerAddr,
      List<String> victims
  ) {
    Set<String> txHashes = collectTransactionHashes(swaps, frontIndex, backIndex);
    double profitEstimate = estimateProfit(frontRun, backRun);

    return new MevCandidate(
        "SANDWICH_ATTACK",
        frontRun.blockNumber(),
        attackerAddr,
        victims,
        profitEstimate,
        backIndex - frontIndex + 1,
        backIndex - frontIndex + 1,
        txHashes,
        String.format("Sandwich: %s front-ran %d victim(s) in block %d",
            attackerAddr, victims.size(), frontRun.blockNumber())
    );
  }

  private List<String> extractVictims(List<SwapEvent> swaps, int start, int end,
      String attackerAddr) {
    List<String> victims = new ArrayList<>();
    for (int v = start + 1; v < end; v++) {
      String victimAddr = normalizeAddress(swaps.get(v).sender());
      if (!victimAddr.equals(attackerAddr)) {
        victims.add(victimAddr);
      }
    }
    return victims;
  }

  private Set<String> collectTransactionHashes(List<SwapEvent> swaps, int start, int end) {
    Set<String> txHashes = new HashSet<>();
    for (int i = start; i <= end; i++) {
      txHashes.add(swaps.get(i).transactionHash());
    }
    return txHashes;
  }
}
