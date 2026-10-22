package com.web3analytics.functions;

import com.web3analytics.functions.mev.AlertBuilder;
import com.web3analytics.functions.mev.EventGrouper;
import com.web3analytics.functions.mev.JitLiquidityDetector;
import com.web3analytics.functions.mev.MetadataExtractor;
import com.web3analytics.functions.mev.MevCandidate;
import com.web3analytics.functions.mev.SandwichDetector;
import com.web3analytics.functions.mev.SessionContext;
import com.web3analytics.models.DexEvent;
import com.web3analytics.models.MevAlert;
import java.io.Serial;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.function.BiFunction;
import org.apache.flink.streaming.api.functions.windowing.ProcessWindowFunction;
import org.apache.flink.streaming.api.windowing.windows.TimeWindow;
import org.apache.flink.util.Collector;

/**
 * Session-window function detecting MEV patterns within burst activity on a single pair.
 *
 * <p>Detected patterns:
 * <ul>
 *   <li><b>SANDWICH_ATTACK</b>: Attacker front-runs + back-runs a victim swap in the same block</li>
 *   <li><b>JIT_LIQUIDITY</b>: Mint → large Swap → Burn in same or adjacent blocks</li>
 * </ul>
 */
public class MevDetectionFunction
        extends ProcessWindowFunction<DexEvent, MevAlert, String, TimeWindow> {

  @Serial
  private static final long serialVersionUID = 1L;

  private final transient List<BiFunction<List<DexEvent>, SessionContext, List<MevCandidate>>> perBlockDetectors;
  private final transient List<BiFunction<List<DexEvent>, SessionContext, List<MevCandidate>>> sessionDetectors;

  public MevDetectionFunction() {
    this(
        List.of(new SandwichDetector()::detect),
        List.of(new JitLiquidityDetector()::detect)
    );
  }

  MevDetectionFunction(
      List<BiFunction<List<DexEvent>, SessionContext, List<MevCandidate>>> perBlockDetectors,
      List<BiFunction<List<DexEvent>, SessionContext, List<MevCandidate>>> sessionDetectors
  ) {
    this.perBlockDetectors = perBlockDetectors;
    this.sessionDetectors = sessionDetectors;
  }

    @Override
    public void process(
            String pairAddress,
            Context context,
            Iterable<DexEvent> events,
            Collector<MevAlert> out
    ) {
        List<MevAlert> alerts = detect(
                pairAddress,
                context.window().getStart(),
                context.window().getEnd(),
                events,
                Instant.now().toEpochMilli()
        );
        alerts.forEach(out::collect);
    }

  List<MevAlert> detect(
            String pairAddress,
            long windowStart,
            long windowEnd,
            Iterable<DexEvent> events,
            long detectedAt
    ) {
    List<DexEvent> sorted = EventGrouper.sortByBlockAndLog(events);
    SessionContext context = MetadataExtractor.buildContext(
        pairAddress, windowStart, windowEnd, sorted, detectedAt);

    Map<Long, List<DexEvent>> byBlock = EventGrouper.groupByBlock(sorted);

    List<MevCandidate> candidates = new ArrayList<>();

    // Run per-block detectors
    for (List<DexEvent> blockEvents : byBlock.values()) {
      for (var detector : perBlockDetectors) {
        candidates.addAll(detector.apply(blockEvents, context));
            }
    }

    // Run session-wide detectors
    for (var detector : sessionDetectors) {
      candidates.addAll(detector.apply(sorted, context));
        }

    return candidates.stream()
        .map(c -> AlertBuilder.build(c, context))
        .toList();
    }
}
