package com.web3analytics.functions.mev;

import com.web3analytics.models.DexEvent;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Groups and sorts events for MEV detection.
 */
public final class EventGrouper {

  private EventGrouper() {
    throw new UnsupportedOperationException("Utility class");
  }

  public static List<DexEvent> sortByBlockAndLog(Iterable<DexEvent> events) {
    List<DexEvent> sorted = new ArrayList<>();
    events.forEach(sorted::add);
    sorted.sort(Comparator.comparingLong(DexEvent::blockTimestamp)
        .thenComparingInt(DexEvent::logIndex));
    return sorted;
  }

  public static Map<Long, List<DexEvent>> groupByBlock(List<DexEvent> events) {
    Map<Long, List<DexEvent>> byBlock = new HashMap<>();
    for (DexEvent e : events) {
      byBlock.computeIfAbsent(e.blockNumber(), k -> new ArrayList<>()).add(e);
    }
    return byBlock;
  }
}
